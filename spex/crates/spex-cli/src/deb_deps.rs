use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::process::Command;

/// Runs the real `dpkg -s` (Debian/Ubuntu package status) on `package` and
/// its direct dependencies, converting them into a `spex_graph::Graph`: the
/// package as root, one child per direct dependency, real installed size
/// (KB) driving color. Start-small scope, matching `brew-deps`'s
/// architecture: one level of real direct deps (`Depends:` field), not a
/// full recursive apt/dnf tree. Only runs on a real Debian/Ubuntu system —
/// `dpkg` isn't present elsewhere.
pub fn run(package: &str) -> Result<Graph> {
    let root = package_info(package).with_context(|| format!("running `dpkg -s {package}` (is this a Debian/Ubuntu system?)"))?;

    let mut nodes = Vec::with_capacity(root.depends.len() + 1);
    let mut metadata = Map::new();
    metadata.insert("dependsRaw".to_string(), Value::from(root.depends.clone()));
    nodes.push(GraphNode {
        id: package.to_string(),
        label: package.to_string(),
        parent: None,
        metric: root.installed_size_kb,
        metadata,
    });

    for dep in &root.depends {
        // Some direct deps are virtual packages/alternatives that may not
        // themselves be queryable — skip gracefully rather than failing the
        // whole tree, same spirit as brew-deps tolerating parse gaps.
        let Ok(info) = package_info(dep) else { continue };
        nodes.push(GraphNode {
            id: dep.clone(),
            label: dep.clone(),
            parent: Some(package.to_string()),
            metric: info.installed_size_kb,
            metadata: Map::new(),
        });
    }

    Ok(Graph {
        title: Some(format!("dpkg direct dependencies: {package}")),
        metric_label: Some("installed size (KB)".to_string()),
        nodes,
    })
}

struct PackageInfo {
    installed_size_kb: Option<f64>,
    depends: Vec<String>,
}

fn package_info(package: &str) -> Result<PackageInfo> {
    let output = Command::new("dpkg").args(["-s", package]).output().context("running `dpkg` (is it on PATH?)")?;
    if !output.status.success() {
        bail!("dpkg -s {package} failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(parse_status(&String::from_utf8_lossy(&output.stdout)))
}

/// Parses `dpkg -s` output (RFC 822-style control file fields). `Depends:`
/// is a comma-separated list where each entry may have a version constraint
/// in parens (`libc6 (>= 2.34)`) and/or `|`-separated alternatives
/// (`libssl3 (>= 3.0.0) | libssl1.1`) — only the first alternative is kept
/// per entry (same "pick one, `Graph` is tree-only" tradeoff as `sql-schema`
/// picking a table's first foreign key).
fn parse_status(output: &str) -> PackageInfo {
    let mut installed_size_kb = None;
    let mut depends = Vec::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("Installed-Size:") {
            installed_size_kb = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("Depends:") {
            depends = rest
                .split(',')
                .filter_map(|entry| {
                    let first_alt = entry.split('|').next()?;
                    let name = first_alt.split('(').next()?.trim();
                    (!name.is_empty()).then(|| name.to_string())
                })
                .collect();
        }
    }

    PackageInfo { installed_size_kb, depends }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Package: curl
Status: install ok installed
Priority: optional
Section: web
Installed-Size: 386
Maintainer: Ubuntu Developers <ubuntu-devel-discuss@lists.ubuntu.com>
Architecture: arm64
Multi-Arch: foreign
Source: curl
Version: 7.88.1-10+deb12u5
Depends: libc6 (>= 2.34), libcurl4 (= 7.88.1-10+deb12u5), libssl3 (>= 3.0.0) | libssl1.1
Description: command line tool for transferring data with URL syntax
 curl is a command line tool for transferring data with URL syntax.
";

    #[test]
    fn parses_installed_size_and_depends_with_versions_and_alternatives() {
        let info = parse_status(SAMPLE);
        assert_eq!(info.installed_size_kb, Some(386.0));
        assert_eq!(info.depends, vec!["libc6", "libcurl4", "libssl3"]);
    }

    #[test]
    fn missing_fields_are_handled_gracefully() {
        let info = parse_status("Package: foo\nStatus: install ok installed\n");
        assert_eq!(info.installed_size_kb, None);
        assert!(info.depends.is_empty());
    }
}
