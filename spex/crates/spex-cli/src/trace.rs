use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::process::Command;

const ROOT_ID: &str = "source";

/// Captures a traceroute to `host` and converts the hop-by-hop result into a
/// `spex_graph::Graph`: a simple chain rooted at a synthetic "source" node
/// (this machine), one node per hop, colored later by average RTT.
///
/// `use_icmp` selects the probing mechanism; both produce the same `Vec<Hop>`
/// shape, so the hop -> `Graph` conversion (`hops_to_graph`) below is shared
/// — only the probe itself differs:
/// - `false` (default): shells out to the system `traceroute` (UDP-based,
///   no elevated privileges needed) and parses its real text output — the
///   original, still-default behavior.
/// - `true`: sends real ICMP echo requests over a raw socket, closer to the
///   classic `traceroute` implementation. This needs a raw socket, which
///   the OS only grants to root or a process with `CAP_NET_RAW` — see
///   `icmp::probe`'s doc comment for the exact, actionable error this
///   produces when that privilege isn't available.
pub fn run(host: &str, use_icmp: bool) -> Result<Graph> {
    let hops = if use_icmp {
        icmp::probe(host)?
    } else {
        run_udp_shellout(host)?
    };

    if hops.is_empty() {
        bail!("could not capture any hops for {host}");
    }

    Ok(hops_to_graph(host, &hops))
}

/// The original probing mechanism: shells out to the real system
/// `traceroute` (standard UDP mode, no elevated privileges needed) and
/// parses its real text output into `Hop`s.
fn run_udp_shellout(host: &str) -> Result<Vec<Hop>> {
    let output = Command::new("traceroute")
        .args(["-w", "2", "-q", "3", host])
        .output()
        .context("running `traceroute` (is it installed and on PATH?)")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        bail!(
            "traceroute produced no output (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let hops = parse_traceroute(&stdout);
    if hops.is_empty() {
        bail!("could not parse any hops from traceroute output");
    }
    Ok(hops)
}

/// The graph-building logic shared by every probing mechanism: turns a
/// sequence of hops (however they were captured) into a `spex_graph::Graph`.
fn hops_to_graph(host: &str, hops: &[Hop]) -> Graph {
    let mut nodes = vec![GraphNode {
        id: ROOT_ID.to_string(),
        label: "you".to_string(),
        parent: None,
        metric: None,
        metadata: Map::new(),
    }];

    let mut prev_id = ROOT_ID.to_string();
    for hop in hops {
        let id = format!("hop-{}", hop.number);
        let mut metadata = Map::new();
        metadata.insert("hop".to_string(), Value::from(hop.number));
        if let Some((hostname, ip)) = hop.host.as_ref() {
            metadata.insert("hostname".to_string(), Value::from(hostname.clone()));
            metadata.insert("ip".to_string(), Value::from(ip.clone()));
        } else {
            metadata.insert("timeout".to_string(), Value::from(true));
        }
        if !hop.rtts_ms.is_empty() {
            metadata.insert(
                "rttSamplesMs".to_string(),
                Value::from(hop.rtts_ms.to_vec()),
            );
        }

        let metric = if hop.rtts_ms.is_empty() {
            None
        } else {
            Some(hop.rtts_ms.iter().sum::<f64>() / hop.rtts_ms.len() as f64)
        };
        let label = hop
            .host
            .as_ref()
            .map(|(hostname, _)| hostname.clone())
            .unwrap_or_else(|| "*".to_string());

        nodes.push(GraphNode {
            id: id.clone(),
            label,
            parent: Some(prev_id.clone()),
            metric,
            metadata,
        });
        prev_id = id;
    }

    Graph {
        title: Some(format!("traceroute to {host}")),
        metric_label: Some("avg RTT (ms)".to_string()),
        nodes,
    }
}

/// One traceroute hop, regardless of which probing mechanism produced it.
struct Hop {
    number: u32,
    /// (hostname, ip) of the first responder seen for this hop; `None` if every probe timed out.
    host: Option<(String, String)>,
    rtts_ms: Vec<f64>,
}

fn parse_traceroute(output: &str) -> Vec<Hop> {
    let mut hops: Vec<Hop> = Vec::new();
    let mut current: Option<Hop> = None;

    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let rest: &[&str] = if let Ok(number) = tokens[0].parse::<u32>() {
            if let Some(hop) = current.take() {
                hops.push(hop);
            }
            current = Some(Hop {
                number,
                host: None,
                rtts_ms: Vec::new(),
            });
            &tokens[1..]
        } else if current.is_some() {
            // Continuation line: an additional responder/RTT sample for the current hop.
            &tokens[..]
        } else {
            // Preamble (e.g. "traceroute to ... hops max, ... byte packets"): skip.
            continue;
        };

        let hop = current.as_mut().expect("current hop set above");
        let mut i = 0;
        while i < rest.len() {
            let tok = rest[i];
            if tok == "*" {
                i += 1;
                continue;
            }
            if i + 1 < rest.len() && rest[i + 1] == "ms" {
                if let Ok(v) = tok.parse::<f64>() {
                    hop.rtts_ms.push(v);
                }
                i += 2;
                continue;
            }
            // A hostname (optionally followed by "(ip)"); only keep the first responder.
            if hop.host.is_none() && !tok.starts_with('(') {
                let hostname = tok.to_string();
                if i + 1 < rest.len() && rest[i + 1].starts_with('(') {
                    let ip = rest[i + 1].trim_matches(|c| c == '(' || c == ')').to_string();
                    hop.host = Some((hostname, ip));
                    i += 2;
                } else {
                    hop.host = Some((hostname.clone(), hostname));
                    i += 1;
                }
                continue;
            }
            i += 1;
        }
    }
    if let Some(hop) = current.take() {
        hops.push(hop);
    }
    hops
}

/// Real ICMP echo-request/reply probing over a raw socket — closer to the
/// classic `traceroute` implementation than shelling out to the system
/// binary's UDP mode. Split into pure packet construction/parsing (testable
/// without any socket at all) and the actual socket I/O (which needs a raw
/// socket, and therefore real elevated privileges, so it isn't exercised in
/// CI — see `probe`'s doc comment).
mod icmp {
    use super::Hop;
    use anyhow::Result;
    use std::net::Ipv4Addr;

    const ICMP_ECHO_REPLY: u8 = 0;
    const ICMP_ECHO_REQUEST: u8 = 8;
    const ICMP_DEST_UNREACHABLE: u8 = 3;
    const ICMP_TIME_EXCEEDED: u8 = 11;

    /// Standard Internet checksum (RFC 1071) over `data`, treated as a
    /// sequence of 16-bit big-endian words (an odd trailing byte is
    /// zero-padded on the low byte).
    fn checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        let mut chunks = data.chunks_exact(2);
        for chunk in &mut chunks {
            sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        }
        if let [last] = chunks.remainder() {
            sum += (*last as u32) << 8;
        }
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !(sum as u16)
    }

    /// Builds a real ICMP echo request packet (type 8, code 0) with a
    /// correct checksum: `identifier`/`sequence` let a reply be matched back
    /// to the probe that produced it, `payload` is arbitrary trailing data
    /// classic `traceroute`/`ping` also send (and get echoed back).
    fn build_echo_request(identifier: u16, sequence: u16, payload: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(8 + payload.len());
        packet.push(ICMP_ECHO_REQUEST);
        packet.push(0); // code
        packet.extend_from_slice(&[0, 0]); // checksum placeholder, filled below
        packet.extend_from_slice(&identifier.to_be_bytes());
        packet.extend_from_slice(&sequence.to_be_bytes());
        packet.extend_from_slice(payload);
        let sum = checksum(&packet);
        packet[2..4].copy_from_slice(&sum.to_be_bytes());
        packet
    }

    /// Splits a raw IPv4 packet into its header's source address and the
    /// payload after the (variable-length, per IHL) header. A raw ICMP
    /// socket on Linux/macOS/BSD delivers the *whole* IP packet, header
    /// included, so this has to run before any ICMP parsing.
    fn split_ipv4_packet(data: &[u8]) -> Option<(Ipv4Addr, &[u8])> {
        if data.len() < 20 || data[0] >> 4 != 4 {
            return None;
        }
        let header_len = (data[0] & 0x0F) as usize * 4;
        if data.len() < header_len {
            return None;
        }
        let source = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
        Some((source, &data[header_len..]))
    }

    /// A parsed, relevant ICMP response to one of our own echo requests.
    struct IcmpReply {
        /// The IP address that actually sent this ICMP message — for a
        /// time-exceeded/unreachable reply this is the responding router,
        /// not (necessarily) our final target.
        source_ip: Ipv4Addr,
        /// `true` for an echo reply (we reached the target); `false` for a
        /// time-exceeded/unreachable message from an intermediate hop.
        is_echo_reply: bool,
    }

    /// Parses a raw received IPv4 packet and returns `Some` only if it's a
    /// real response to *our own* echo request (`expect_identifier`/
    /// `expect_sequence` match) — either the destination's echo reply, or an
    /// intermediate router's time-exceeded/unreachable message quoting our
    /// original echo request back at us (the classic `traceroute` trick:
    /// the quoted 8 bytes are enough to recover our identifier/sequence).
    /// Anything else (someone else's ping, an unrelated ICMP message)
    /// returns `None` so the caller skips it.
    fn parse_icmp_reply(raw_packet: &[u8], expect_identifier: u16, expect_sequence: u16) -> Option<IcmpReply> {
        let (source_ip, icmp) = split_ipv4_packet(raw_packet)?;
        if icmp.len() < 8 {
            return None;
        }
        match icmp[0] {
            ICMP_ECHO_REPLY => {
                let id = u16::from_be_bytes([icmp[4], icmp[5]]);
                let seq = u16::from_be_bytes([icmp[6], icmp[7]]);
                (id == expect_identifier && seq == expect_sequence).then_some(IcmpReply {
                    source_ip,
                    is_echo_reply: true,
                })
            }
            ICMP_TIME_EXCEEDED | ICMP_DEST_UNREACHABLE => {
                // Bytes 8.. are the original IP header + first 8 bytes of
                // our original datagram (our echo request), quoted back.
                let embedded = icmp.get(8..)?;
                let (_, orig_icmp) = split_ipv4_packet(embedded)?;
                if orig_icmp.len() < 8 {
                    return None;
                }
                let id = u16::from_be_bytes([orig_icmp[4], orig_icmp[5]]);
                let seq = u16::from_be_bytes([orig_icmp[6], orig_icmp[7]]);
                (id == expect_identifier && seq == expect_sequence).then_some(IcmpReply {
                    source_ip,
                    is_echo_reply: false,
                })
            }
            _ => None,
        }
    }

    /// Real ICMP echo-request/reply probing, one increasing TTL at a time
    /// (classic `traceroute`): opens a single raw ICMP socket, and for each
    /// TTL sends `PROBES_PER_HOP` echo requests, waiting for either the
    /// target's echo reply (done) or an intermediate router's time-exceeded
    /// message (record it, keep going).
    ///
    /// Opening a raw socket needs real elevated privileges — the OS grants
    /// `SOCK_RAW` only to root or a process with `CAP_NET_RAW`. Without one
    /// of those this fails with a permission-denied error, which is wrapped
    /// below into a specific, actionable message rather than silently
    /// falling back to the UDP shell-out path (that would be confusing:
    /// silently running a different probing method than the one asked for).
    /// Only implemented for Unix (Linux/macOS) — see the `cfg(not(unix))`
    /// arm.
    pub(crate) fn probe(host: &str) -> Result<Vec<Hop>> {
        #[cfg(unix)]
        {
            unix::probe(host)
        }
        #[cfg(not(unix))]
        {
            let _ = host;
            anyhow::bail!(
                "ICMP raw-socket probing (`--icmp`) is only implemented for Unix (Linux/macOS) \
                 in this build; use the default UDP traceroute mode instead"
            );
        }
    }

    #[cfg(unix)]
    mod unix {
        use super::{build_echo_request, parse_icmp_reply};
        use crate::trace::Hop;
        use anyhow::{Context, Result};
        use socket2::{Domain, Protocol, SockAddr, Socket, Type};
        use std::mem::MaybeUninit;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
        use std::time::{Duration, Instant};

        const MAX_TTL: u32 = 30;
        const PROBES_PER_HOP: u16 = 3;
        const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

        pub(crate) fn probe(host: &str) -> Result<Vec<Hop>> {
            let dest = resolve_ipv4(host)?;

            let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).context(
                "opening a raw ICMP socket for `spex trace --icmp` (this needs elevated \
                 privileges: re-run with `sudo`, or grant this binary the capability once with \
                 `sudo setcap cap_net_raw+ep <path-to-spex>`)",
            )?;
            socket
                .set_read_timeout(Some(PROBE_TIMEOUT))
                .context("setting read timeout on the ICMP socket")?;

            let identifier = (std::process::id() & 0xFFFF) as u16;
            let target = SockAddr::from(SocketAddr::new(IpAddr::V4(dest), 0));

            let mut hops = Vec::new();
            for ttl in 1..=MAX_TTL {
                socket
                    .set_ttl_v4(ttl)
                    .with_context(|| format!("setting TTL {ttl} on the ICMP socket"))?;

                let mut hop = Hop {
                    number: ttl,
                    host: None,
                    rtts_ms: Vec::new(),
                };
                let mut reached = false;

                for seq in 0..PROBES_PER_HOP {
                    let packet = build_echo_request(identifier, seq, b"spex trace --icmp");
                    let sent_at = Instant::now();
                    if socket.send_to(&packet, &target).is_err() {
                        continue; // e.g. host unreachable at send time; count as a lost probe
                    }

                    let mut buf = [MaybeUninit::<u8>::uninit(); 1024];
                    let Ok((n, _from)) = socket.recv_from(&mut buf) else {
                        continue; // timed out — same as a "*" probe in classic traceroute
                    };
                    let elapsed_ms = sent_at.elapsed().as_secs_f64() * 1000.0;
                    // SAFETY: `recv_from` guarantees the first `n` bytes are initialized.
                    let received: &[u8] =
                        unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), n) };

                    let Some(reply) = parse_icmp_reply(received, identifier, seq) else {
                        continue; // not a response to our own probe — ignore
                    };
                    let ip = reply.source_ip.to_string();
                    if hop.host.is_none() {
                        hop.host = Some((ip.clone(), ip));
                    }
                    hop.rtts_ms.push(elapsed_ms);
                    if reply.is_echo_reply {
                        reached = true;
                    }
                }

                hops.push(hop);
                if reached {
                    break;
                }
            }

            Ok(hops)
        }

        fn resolve_ipv4(host: &str) -> Result<Ipv4Addr> {
            (host, 0)
                .to_socket_addrs()
                .with_context(|| format!("resolving {host}"))?
                .find_map(|addr| match addr.ip() {
                    IpAddr::V4(v4) => Some(v4),
                    IpAddr::V6(_) => None,
                })
                .with_context(|| {
                    format!("{host} has no IPv4 address (IPv6 targets aren't supported by --icmp yet)")
                })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn checksum_of_empty_is_all_ones() {
            assert_eq!(checksum(&[]), 0xFFFF);
        }

        #[test]
        fn checksum_zeroes_out_when_folded_back_in() {
            // The standard property every real internet-checksum implementation
            // must satisfy: recomputing the checksum over data that already
            // contains a correct checksum field yields exactly zero.
            let packet = build_echo_request(0x1234, 7, b"payload");
            assert_eq!(checksum(&packet), 0);
        }

        #[test]
        fn build_echo_request_frames_type_code_and_ids() {
            let packet = build_echo_request(0xABCD, 0x0042, b"hi");
            assert_eq!(packet[0], ICMP_ECHO_REQUEST);
            assert_eq!(packet[1], 0); // code
            assert_eq!(u16::from_be_bytes([packet[4], packet[5]]), 0xABCD);
            assert_eq!(u16::from_be_bytes([packet[6], packet[7]]), 0x0042);
            assert_eq!(&packet[8..], b"hi");
        }

        fn ipv4_header(total_len: u16, protocol: u8, source: [u8; 4], dest: [u8; 4]) -> Vec<u8> {
            let mut header = vec![0u8; 20];
            header[0] = 0x45; // version 4, IHL 5 (20 bytes, no options)
            header[2..4].copy_from_slice(&total_len.to_be_bytes());
            header[8] = 64; // TTL
            header[9] = protocol;
            header[12..16].copy_from_slice(&source);
            header[16..20].copy_from_slice(&dest);
            header
        }

        #[test]
        fn parses_echo_reply_matching_our_probe() {
            let icmp = build_reply(ICMP_ECHO_REPLY, 0x1111, 5);
            let mut packet = ipv4_header(20 + icmp.len() as u16, 1, [93, 184, 216, 34], [10, 0, 0, 1]);
            packet.extend_from_slice(&icmp);

            let reply = parse_icmp_reply(&packet, 0x1111, 5).expect("should parse");
            assert!(reply.is_echo_reply);
            assert_eq!(reply.source_ip, Ipv4Addr::new(93, 184, 216, 34));
        }

        #[test]
        fn parses_time_exceeded_from_intermediate_router() {
            // The router's ICMP payload quotes back our original IP header +
            // echo request header (the classic traceroute matching trick).
            let mut quoted = ipv4_header(28, 1, [10, 0, 0, 1], [93, 184, 216, 34]);
            quoted.extend_from_slice(&build_echo_request(0x2222, 3, b"spex trace --icmp"));

            let mut icmp = vec![ICMP_TIME_EXCEEDED, 0, 0, 0, 0, 0, 0, 0];
            icmp.extend_from_slice(&quoted[..28]); // 8 bytes are enough per RFC, keep a bit more

            let mut packet = ipv4_header(20 + icmp.len() as u16, 1, [172, 16, 0, 1], [10, 0, 0, 1]);
            packet.extend_from_slice(&icmp);

            let reply = parse_icmp_reply(&packet, 0x2222, 3).expect("should parse");
            assert!(!reply.is_echo_reply);
            assert_eq!(reply.source_ip, Ipv4Addr::new(172, 16, 0, 1));
        }

        #[test]
        fn rejects_reply_to_a_different_probe() {
            let icmp = build_reply(ICMP_ECHO_REPLY, 0x1111, 5);
            let mut packet = ipv4_header(20 + icmp.len() as u16, 1, [93, 184, 216, 34], [10, 0, 0, 1]);
            packet.extend_from_slice(&icmp);

            // Right identifier, wrong sequence: not our probe, must be ignored.
            assert!(parse_icmp_reply(&packet, 0x1111, 6).is_none());
            // Wrong identifier entirely (e.g. another process's ping): ignored too.
            assert!(parse_icmp_reply(&packet, 0x9999, 5).is_none());
        }

        #[test]
        fn rejects_truncated_or_non_ipv4_packets() {
            assert!(parse_icmp_reply(&[], 1, 1).is_none());
            assert!(parse_icmp_reply(&[0x60, 0, 0, 0], 1, 1).is_none()); // version 6, not 4
        }

        fn build_reply(icmp_type: u8, identifier: u16, sequence: u16) -> Vec<u8> {
            let mut packet = build_echo_request(identifier, sequence, b"reply");
            packet[0] = icmp_type;
            packet[2..4].copy_from_slice(&[0, 0]);
            let sum = checksum(&packet);
            packet[2..4].copy_from_slice(&sum.to_be_bytes());
            packet
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
traceroute to www.de-cix.net.cdn.cloudflare.net (172.66.158.55), 30 hops max, 40 byte packets
 1  fritz.box (192.168.178.1)  73.288 ms  5.144 ms  5.404 ms
 2  100.98.64.1 (100.98.64.1)  6.612 ms  8.001 ms  6.624 ms
 3  * * 172.17.112.83 (172.17.112.83)  8.165 ms
 4  * * *
 7  109.104.61.254 (109.104.61.254)  8.299 ms  8.625 ms
    beber-mc02.hlkomm.net (109.104.59.180)  7.775 ms
";

    #[test]
    fn parses_sample_traceroute_output() {
        let hops = parse_traceroute(SAMPLE);
        assert_eq!(hops.len(), 5);

        assert_eq!(hops[0].number, 1);
        assert_eq!(hops[0].host.as_ref().unwrap().1, "192.168.178.1");
        assert_eq!(hops[0].rtts_ms.len(), 3);

        // Hop 3: two lost probes ("*") then one successful responder.
        assert_eq!(hops[2].number, 3);
        assert_eq!(hops[2].host.as_ref().unwrap().1, "172.17.112.83");
        assert_eq!(hops[2].rtts_ms.len(), 1);

        // Hop 4: fully timed out.
        assert_eq!(hops[3].number, 4);
        assert!(hops[3].host.is_none());
        assert!(hops[3].rtts_ms.is_empty());

        // Hop 7: continuation line adds another RTT sample (from a different responder,
        // but we only track the first host for the simple format).
        assert_eq!(hops[4].number, 7);
        assert_eq!(hops[4].host.as_ref().unwrap().1, "109.104.61.254");
        assert_eq!(hops[4].rtts_ms.len(), 3);
    }

    #[test]
    fn builds_chained_graph_with_synthetic_root() {
        let hops = parse_traceroute(SAMPLE);
        assert_eq!(hops.len(), 5);
        // Exercise the real, shared hop -> Graph conversion directly against
        // parsed hops, rather than `run`, which shells out to a real
        // traceroute (or opens a real raw socket for the ICMP path).
        let graph = hops_to_graph("www.de-cix.net.cdn.cloudflare.net", &hops);
        assert_eq!(graph.nodes.len(), 6);
        assert_eq!(graph.nodes[0].id, ROOT_ID);
        assert_eq!(graph.nodes[1].parent.as_deref(), Some(ROOT_ID));
        assert_eq!(graph.nodes[2].parent.as_deref(), Some("hop-1"));
        // Metric is the average RTT; hop 4 (fully timed out) has none.
        assert!(graph.nodes[4].metric.is_none());
        assert!(graph.nodes[1].metric.is_some());
    }

    #[test]
    fn hops_to_graph_shared_by_both_probing_mechanisms() {
        // The same conversion function used by both the UDP shell-out and
        // ICMP raw-socket paths (see `run`) — this is the one place hop data
        // becomes a `Graph`, regardless of how the hops were captured.
        let hops = vec![Hop {
            number: 1,
            host: Some(("router.local".to_string(), "10.0.0.1".to_string())),
            rtts_ms: vec![1.0, 2.0, 3.0],
        }];
        let graph = hops_to_graph("example.com", &hops);
        assert_eq!(graph.title.as_deref(), Some("traceroute to example.com"));
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[1].label, "router.local");
        assert_eq!(graph.nodes[1].metric, Some(2.0));
    }
}
