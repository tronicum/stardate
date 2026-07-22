use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};

struct Proc {
    id: &'static str,
    label: &'static str,
    parent: Option<&'static str>,
    pid: u32,
    mem_mb: f64,
}

/// A fabricated process tree for a minimal, freshly-booted Linux system.
/// Not read from any real machine — purely illustrative demo data for the
/// "generic tree explorer" pstree use case (branching, unlike traceroute's chain).
const PROCS: &[Proc] = &[
    Proc { id: "systemd", label: "systemd", parent: None, pid: 1, mem_mb: 3.2 },
    Proc { id: "journald", label: "systemd-journald", parent: Some("systemd"), pid: 112, mem_mb: 8.4 },
    Proc { id: "udevd", label: "systemd-udevd", parent: Some("systemd"), pid: 130, mem_mb: 4.1 },
    Proc { id: "networkd", label: "systemd-networkd", parent: Some("systemd"), pid: 345, mem_mb: 3.0 },
    Proc { id: "resolved", label: "systemd-resolved", parent: Some("systemd"), pid: 346, mem_mb: 2.8 },
    Proc { id: "dbus", label: "dbus-daemon", parent: Some("systemd"), pid: 400, mem_mb: 2.1 },
    Proc { id: "cron", label: "cron", parent: Some("systemd"), pid: 410, mem_mb: 1.2 },
    Proc { id: "getty", label: "getty (tty1)", parent: Some("systemd"), pid: 550, mem_mb: 1.0 },
    Proc { id: "sshd", label: "sshd", parent: Some("systemd"), pid: 500, mem_mb: 4.4 },
    Proc { id: "sshd-session", label: "sshd: user [priv]", parent: Some("sshd"), pid: 620, mem_mb: 5.6 },
    Proc { id: "bash", label: "bash", parent: Some("sshd-session"), pid: 621, mem_mb: 3.3 },
    Proc { id: "pstree", label: "pstree", parent: Some("bash"), pid: 700, mem_mb: 1.8 },
];

pub fn generate() -> Graph {
    let nodes = PROCS
        .iter()
        .map(|p| {
            let mut metadata = Map::new();
            metadata.insert("pid".to_string(), Value::from(p.pid));
            metadata.insert("memMb".to_string(), Value::from(p.mem_mb));
            GraphNode {
                id: p.id.to_string(),
                label: p.label.to_string(),
                parent: p.parent.map(|s| s.to_string()),
                metric: Some(p.mem_mb),
                metadata,
            }
        })
        .collect();
    Graph {
        title: Some("fabricated example process tree (not real data)".to_string()),
        metric_label: Some("memory (MB)".to_string()),
        nodes,
    }
}
