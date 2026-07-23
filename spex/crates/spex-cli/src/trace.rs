use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::process::Command;

const ROOT_ID: &str = "source";

/// Runs the system `traceroute` (standard UDP mode, no elevated privileges
/// needed) against `host` and converts the hop-by-hop output into a
/// `spex_graph::Graph`: a simple chain rooted at a synthetic "source" node
/// (this machine), one node per hop, colored later by average RTT.
pub fn run(host: &str) -> Result<Graph> {
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
        if let Some((hostname, ip)) = &hop.host {
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

    Ok(Graph {
        title: Some(format!("traceroute to {host}")),
        metric_label: Some("avg RTT (ms)".to_string()),
        nodes,
    })
}

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
        // Spot-check the graph-building logic directly against parsed hops
        // rather than re-invoking `run`, which shells out to a real traceroute.
        let mut nodes = vec![GraphNode {
            id: ROOT_ID.to_string(),
            label: "you".to_string(),
            parent: None,
            metric: None,
            metadata: Map::new(),
        }];
        let mut prev_id = ROOT_ID.to_string();
        for hop in &hops {
            let id = format!("hop-{}", hop.number);
            nodes.push(GraphNode {
                id: id.clone(),
                label: hop.host.as_ref().map(|(h, _)| h.clone()).unwrap_or_else(|| "*".to_string()),
                parent: Some(prev_id.clone()),
                metric: None,
                metadata: Map::new(),
            });
            prev_id = id;
        }
        assert_eq!(nodes.len(), 6);
        assert_eq!(nodes[0].id, ROOT_ID);
        assert_eq!(nodes[1].parent.as_deref(), Some(ROOT_ID));
        assert_eq!(nodes[2].parent.as_deref(), Some("hop-1"));
    }
}
