use anyhow::{Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::process::Command;

/// Runs `ps -axo pid,ppid,%cpu,%mem,comm` and converts the REAL process tree
/// on this machine into a `spex_graph::Graph` (parent/child from pid/ppid,
/// %mem driving color). Real system state, but only pid/ppid/cpu/mem/executable
/// name — no command-line arguments or usernames — the "pstree" input adapter,
/// using genuine data instead of the earlier fabricated example.
///
/// If `root_pid` is given, only that pid and its descendants are kept — a
/// real system's process tree is very wide (one process can easily have
/// hundreds of direct children), so scoping to a subtree of interest (e.g.
/// your shell session, found with `pgrep <name>` or `echo $$`) gives a much
/// smaller, more legible result than the whole system.
pub fn run(root_pid: Option<u32>) -> Result<Graph> {
    let output = Command::new("ps")
        .args(["-axo", "pid,ppid,%cpu,%mem,comm"])
        .output()
        .context("running `ps` (is it on PATH?)")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut procs = parse_ps(&stdout);
    if let Some(root) = root_pid {
        procs = filter_subtree(procs, root);
    }
    let mut graph = build_graph(procs);
    graph.title = Some(match root_pid {
        Some(pid) => format!("process tree (pid {pid} + descendants)"),
        None => "process tree (full system)".to_string(),
    });
    graph.metric_label = Some("% memory".to_string());
    Ok(graph)
}

/// Keeps only `root_pid` and everything reachable from it by following
/// ppid -> pid edges downward (BFS).
fn filter_subtree(procs: Vec<Proc>, root_pid: u32) -> Vec<Proc> {
    let mut children_of: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for p in &procs {
        children_of.entry(p.ppid).or_default().push(p.pid);
    }

    let mut keep: HashSet<u32> = HashSet::new();
    let mut queue = VecDeque::new();
    if procs.iter().any(|p| p.pid == root_pid) {
        keep.insert(root_pid);
        queue.push_back(root_pid);
    }
    while let Some(pid) = queue.pop_front() {
        if let Some(kids) = children_of.get(&pid) {
            for &kid in kids {
                if keep.insert(kid) {
                    queue.push_back(kid);
                }
            }
        }
    }

    procs.into_iter().filter(|p| keep.contains(&p.pid)).collect()
}

fn build_graph(procs: Vec<Proc>) -> Graph {
    let pids: HashSet<u32> = procs.iter().map(|p| p.pid).collect();

    let nodes = procs
        .iter()
        .map(|p| {
            let label = Path::new(&p.comm)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.comm.clone());

            let mut metadata = Map::new();
            metadata.insert("pid".to_string(), Value::from(p.pid));
            metadata.insert("ppid".to_string(), Value::from(p.ppid));
            metadata.insert("cpuPercent".to_string(), Value::from(p.cpu));
            metadata.insert("memPercent".to_string(), Value::from(p.mem));
            metadata.insert("comm".to_string(), Value::from(p.comm.clone()));

            let parent = if p.ppid != p.pid && pids.contains(&p.ppid) {
                Some(format!("pid-{}", p.ppid))
            } else {
                None
            };

            GraphNode {
                id: format!("pid-{}", p.pid),
                label,
                parent,
                metric: Some(p.mem),
                metadata,
            }
        })
        .collect();

    Graph { nodes, ..Default::default() }
}

struct Proc {
    pid: u32,
    ppid: u32,
    cpu: f64,
    mem: f64,
    comm: String,
}

/// Parses `ps -axo pid,ppid,%cpu,%mem,comm` output. The first four columns are
/// always plain numbers; everything after them is the command, rejoined with
/// single spaces since some macOS app executable names contain spaces
/// (e.g. ".../MacOS/Google Chrome") and a naive whitespace split would truncate them.
fn parse_ps(output: &str) -> Vec<Proc> {
    let mut procs = Vec::new();
    for line in output.lines().skip(1) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 5 {
            continue;
        }
        let (Ok(pid), Ok(ppid), Ok(cpu), Ok(mem)) =
            (tokens[0].parse::<u32>(), tokens[1].parse::<u32>(), tokens[2].parse::<f64>(), tokens[3].parse::<f64>())
        else {
            continue;
        };
        let comm = tokens[4..].join(" ");
        procs.push(Proc { pid, ppid, cpu, mem, comm });
    }
    procs
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
  PID  PPID  %CPU %MEM COMM
    1     0   0.1  0.1 /sbin/launchd
  343     1   0.5  0.2 /usr/libexec/logd
  621   343   2.0  1.5 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome
";

    #[test]
    fn parses_pid_ppid_and_comm_with_spaces() {
        let procs = parse_ps(SAMPLE);
        assert_eq!(procs.len(), 3);
        assert_eq!(procs[0].pid, 1);
        assert_eq!(procs[0].ppid, 0);
        assert_eq!(procs[2].pid, 621);
        assert_eq!(procs[2].ppid, 343);
        assert_eq!(procs[2].comm, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
    }

    #[test]
    fn builds_tree_with_basename_labels_and_no_root_parent() {
        let graph = build_graph(parse_ps(SAMPLE));
        let launchd = graph.nodes.iter().find(|n| n.id == "pid-1").unwrap();
        assert_eq!(launchd.label, "launchd");
        assert_eq!(launchd.parent, None); // ppid 0 not in pid set -> root

        let chrome = graph.nodes.iter().find(|n| n.id == "pid-621").unwrap();
        assert_eq!(chrome.label, "Google Chrome");
        assert_eq!(chrome.parent.as_deref(), Some("pid-343"));
    }

    const WIDE_SAMPLE: &str = "\
  PID  PPID  %CPU %MEM COMM
    1     0   0.1  0.1 launchd
  100     1   0.0  0.0 daemonA
  200     1   0.0  0.0 shell
  300   200   0.0  0.0 editor
  400   300   0.0  0.0 plugin
";

    #[test]
    fn filter_subtree_keeps_only_root_and_descendants() {
        let procs = parse_ps(WIDE_SAMPLE);
        let scoped = filter_subtree(procs, 200);
        let mut pids: Vec<u32> = scoped.iter().map(|p| p.pid).collect();
        pids.sort();
        assert_eq!(pids, vec![200, 300, 400]); // shell, editor, plugin — not launchd or daemonA

        let graph = build_graph(scoped);
        let shell = graph.nodes.iter().find(|n| n.id == "pid-200").unwrap();
        assert_eq!(shell.parent, None); // its real parent (launchd) is out of scope, so it's a root here
    }
}
