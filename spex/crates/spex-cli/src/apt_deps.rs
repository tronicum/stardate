use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::process::Command;

/// Runs the real `apt-cache show <package>` (Debian/Ubuntu APT package-index
/// lookup) on `package` and its direct dependencies, converting them into a
/// `spex_graph::Graph`: the package as root, one child per direct
/// dependency, real installed size (KB) driving color. Start-small scope,
/// matching `deb-deps`'s architecture: one level of real direct deps (the
/// `Depends:` field), not a full recursive apt tree.
///
/// ## Command choice: `apt-cache show`, not `apt-cache depends` + `dpkg-query`
///
/// `apt-cache show <package>` prints the package's full control-file stanza
/// straight from the local APT package index — the same
/// `Package:`/`Installed-Size:`/`Depends:` field shape `dpkg -s` uses, and
/// the very same single-line, comma-separated, `|`-alternative `Depends:`
/// grammar `deb_deps.rs` already parses (both ultimately come from the same
/// Debian control-file format: the APT `Packages` index and dpkg's local
/// status database share the field syntax by design). That means *one* real
/// subprocess call yields both the dependency list and the installed-size
/// metric, with one real grammar to parse.
///
/// `apt-cache depends <package>` is real too, and is arguably the more
/// "idiomatic" dependency-listing command, but it emits a different,
/// multi-line grammar — one `  Depends: pkg (ver)` / ` |Depends: pkg (ver)`
/// line per dependency, with a leading `|` marking "this is ORed with the
/// next line" instead of an inline `|` — and it carries no size information
/// at all, which would mean bolting on a second real command
/// (`dpkg-query -W -f='${Installed-Size}\n' <pkg>`) to get it. That second
/// command only works for packages that are actually *installed* on the
/// local system (it reads dpkg's status database, exactly like `deb-deps`'s
/// `dpkg -s`), which would silently narrow `apt-deps` down to the same
/// "must already be installed" constraint `deb-deps` already has — defeating
/// the point of using `apt-cache` (the package *index*) in the first place.
///
/// `apt-cache show` doesn't have that limitation: because it reads the local
/// APT cache (populated by `apt-get update`), not the local dpkg
/// install-status database, `apt-deps` works against *available* packages —
/// installed or not — unlike `deb-deps`'s `dpkg -s`, which only works
/// against *installed* ones. That's a real, meaningful difference in what
/// the two adapters can introspect, and worth having both for.
///
/// Only runs on a real Debian/Ubuntu system with a populated APT cache —
/// `apt-cache` isn't present, or has no data, elsewhere.
///
/// **Live-verification status**: code-complete and unit-tested against a
/// realistic hand-written `apt-cache show` fixture (modeled on the real,
/// published Debian bookworm `wget` package — see the module's tests), but
/// NOT verified against a real live `apt-cache` on a real Debian/Ubuntu
/// system. This development machine is macOS with no `apt`/`dpkg` on PATH,
/// and `colima`'s Docker VM can't reach the internet through this machine's
/// active VPN tunnels to pull a Debian/Ubuntu image to test against — the
/// same standing, host-level blocker already documented for `deb-deps`
/// (issue #5) and the RPM adapter (issue #7). See `TODOs.md`.
pub fn run(package: &str) -> Result<Graph> {
    let root = package_info(package).with_context(|| {
        format!("running `apt-cache show {package}` (is this a Debian/Ubuntu system with `apt-get update` run?)")
    })?;

    let mut nodes = Vec::with_capacity(root.depends.len() + 1);
    let mut metadata = Map::new();
    metadata.insert("dependsRaw".to_string(), Value::from(root.depends.clone()));
    nodes.push(GraphNode {
        id: package.to_string(),
        label: package.to_string(),
        parent: None,
        metric: root.installed_size_kb,
        metadata,
        ..Default::default()
    });

    for dep in &root.depends {
        // Some direct deps are virtual packages/provides that `apt-cache
        // show` won't resolve to a concrete stanza (or simply aren't in the
        // local cache) — skip gracefully rather than failing the whole
        // tree, same spirit as `deb-deps` tolerating parse gaps.
        let Ok(info) = package_info(dep) else { continue };
        nodes.push(GraphNode {
            id: dep.clone(),
            label: dep.clone(),
            parent: Some(package.to_string()),
            metric: info.installed_size_kb,
            metadata: Map::new(),
            ..Default::default()
        });
    }

    Ok(Graph {
        title: Some(format!("apt-cache direct dependencies: {package}")),
        metric_label: Some("installed size (KB)".to_string()),
        nodes,
    })
}

struct PackageInfo {
    installed_size_kb: Option<f64>,
    depends: Vec<String>,
}

fn package_info(package: &str) -> Result<PackageInfo> {
    let output = Command::new("apt-cache")
        .args(["show", package])
        .output()
        .context("running `apt-cache` (is it on PATH?)")?;
    if !output.status.success() {
        bail!("apt-cache show {package} failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let stdout = output.stdout;
    let text = String::from_utf8_lossy(&stdout);
    if text.trim().is_empty() {
        bail!("apt-cache show {package} returned no data (package not in the APT cache — try `apt-get update`?)");
    }
    Ok(parse_show(&text))
}

/// Parses `apt-cache show` output: the package's real control-file stanza,
/// straight from the APT index (RFC 822-style fields, the same shape as
/// `dpkg -s`).
///
/// `apt-cache show` can print *multiple* stanzas back to back, separated by
/// a blank line, when more than one version of a package is cached (e.g.
/// available from more than one suite/pocket, or for more than one
/// architecture) — only the first stanza is parsed, matching apt's own
/// convention that the first (highest-priority) stanza is the install
/// candidate.
///
/// `Depends:` is a comma-separated list where each entry may have a version
/// constraint in parens (`libc6 (>= 2.34)`) and/or `|`-separated
/// alternatives (`libssl3 (>= 3.0.0) | libssl1.1`) — only the first
/// alternative is kept per entry (same "pick one, `Graph` is tree-only"
/// tradeoff as `deb_deps.rs` and `sql_schema.rs`'s first-FK-only rule).
fn parse_show(output: &str) -> PackageInfo {
    let mut installed_size_kb = None;
    let mut depends = Vec::new();

    for line in first_stanza(output).lines() {
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

/// `apt-cache show` separates multiple stanzas with a blank line; returns
/// just the first (the install candidate).
fn first_stanza(output: &str) -> &str {
    output.split("\n\n").next().unwrap_or(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A realistic `apt-cache show wget` stanza, modeled on the real
    /// published Debian bookworm `wget` package
    /// (<https://packages.debian.org/bookworm/wget>): real direct
    /// dependency names and version constraints (`libc6`, `libgnutls30`,
    /// `libidn2-0`, `libnettle8`, `libpcre2-8-0`, `libpsl5`, `libuuid1`,
    /// `zlib1g`), real amd64 installed size (3521 kB) as published on that
    /// page. Cosmetic fields (maintainer, hashes, filename) are filler —
    /// only the fields this parser reads were checked against the real
    /// package page.
    const WGET_SAMPLE: &str = "\
Package: wget
Version: 1.21.3-1+deb12u1
Installed-Size: 3521
Maintainer: Debian wget Maintainers <team+wget@tracker.debian.org>
Architecture: amd64
Replaces: wget-udeb
Depends: libc6 (>= 2.34), libgnutls30 (>= 3.7.5), libidn2-0 (>= 0.6), libnettle8, libpcre2-8-0 (>= 10.22), libpsl5 (>= 0.16.0), libuuid1 (>= 2.16), zlib1g (>= 1:1.1.4)
Recommends: ca-certificates
Suggests: gpgv
Breaks: wget-udeb
Description-en: retrieves files from the web
 Wget is a network utility to retrieve files from the Web using HTTP(S) and
 FTP, the two most widely used Internet protocols. It works
 non-interactively, so it will work in the background after having logged
 off.
Homepage: https://www.gnu.org/software/wget/
Section: web
Priority: optional
Filename: pool/main/w/wget/wget_1.21.3-1+deb12u1_amd64.deb
Size: 981012
MD5sum: 00000000000000000000000000000000
SHA256: 0000000000000000000000000000000000000000000000000000000000000000
";

    #[test]
    fn parses_installed_size_and_depends_from_real_wget_stanza() {
        let info = parse_show(WGET_SAMPLE);
        assert_eq!(info.installed_size_kb, Some(3521.0));
        assert_eq!(
            info.depends,
            vec!["libc6", "libgnutls30", "libidn2-0", "libnettle8", "libpcre2-8-0", "libpsl5", "libuuid1", "zlib1g"]
        );
    }

    /// `|`-separated alternatives: modeled on the real Debian OpenSSL 1.1 ->
    /// 3.0 transition pattern used by several packages' `Depends:` fields
    /// during the bullseye→bookworm migration (the same real-world pattern
    /// `deb_deps.rs`'s own test fixture uses). Only the first alternative
    /// should be kept.
    #[test]
    fn keeps_first_alternative_for_or_separated_depends() {
        let sample = "\
Package: example-ssl-using-pkg
Installed-Size: 100
Depends: libc6 (>= 2.34), libssl3 (>= 3.0.0) | libssl1.1
";
        let info = parse_show(sample);
        assert_eq!(info.depends, vec!["libc6", "libssl3"]);
    }

    #[test]
    fn parses_only_first_stanza_when_apt_cache_show_prints_multiple() {
        let two_stanzas = "\
Package: wget
Version: 1.21.3-1+deb12u1
Installed-Size: 3521
Depends: libc6 (>= 2.34), zlib1g (>= 1:1.1.4)

Package: wget
Version: 1.21.3-1
Installed-Size: 3400
Depends: libc6 (>= 2.31)
";
        let info = parse_show(two_stanzas);
        assert_eq!(info.installed_size_kb, Some(3521.0));
        assert_eq!(info.depends, vec!["libc6", "zlib1g"]);
    }

    #[test]
    fn missing_fields_are_handled_gracefully() {
        let info = parse_show("Package: foo\n");
        assert_eq!(info.installed_size_kb, None);
        assert!(info.depends.is_empty());
    }
}
