mod ascii;
mod brew_deps;
mod cargo_deps;
mod deb_deps;
mod disk_usage;
mod export_static;
mod graph_diff;
mod molecule;
mod nav;
mod npm_deps;
mod ps_tree;
mod pstree_demo;
mod sql_schema;
mod trace;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use spex_core::Aabb;
use spex_graph::Graph;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "spex",
    version,
    about = "Point cloud explorer: convert to a streamable octree tileset and view it in the browser"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print point count and bounds for an input file without converting it.
    Info { input: PathBuf },

    /// Convert an input point cloud (.ply, .xyz, .csv, .txt) into an octree tileset.
    Convert {
        input: PathBuf,

        /// Output tileset directory.
        #[arg(short, long)]
        out: PathBuf,

        /// Max points sampled to represent a single octree node's LOD level.
        #[arg(long, default_value_t = 50_000)]
        max_points_per_node: usize,

        /// Hard cap on octree depth.
        #[arg(long, default_value_t = 16)]
        max_depth: usize,
    },

    /// Serve a tileset directory and open the browser viewer.
    Serve {
        tileset_dir: PathBuf,

        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Don't automatically open the default browser.
        #[arg(long)]
        no_open: bool,
    },

    /// Run a traceroute to a host and write it as a spex-graph JSON file.
    Trace {
        host: String,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Lay out a spex-graph JSON file in 3D and build an octree tileset from it
    /// (the "web view"). Also writes nodes.json alongside the tileset: each
    /// node's layout position + label/metric/metadata, for the viewer to render
    /// as on-screen labels.
    GraphLayout {
        graph: PathBuf,

        /// Output tileset directory.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Print a spex-graph JSON file as a human-readable ASCII tree in the
    /// terminal (the "terminal view" — labels, metrics, and metadata inline).
    GraphPrint { graph: PathBuf },

    /// Compare two spex-graph JSON captures of the same kind of tree by node
    /// id — which nodes appeared, disappeared, or changed metric (e.g. two
    /// `ps-tree` snapshots a few seconds apart, or `disk-usage` before/after
    /// a build). Prints to the terminal, no new file written.
    GraphDiff { old: PathBuf, new: PathBuf },

    /// List available demos (subdirectories of `demos/` containing a
    /// graph.json), showing the terminal/web view command for each — so you
    /// can pick which demo and which representation to look at.
    Demos {
        #[arg(default_value = "demos")]
        dir: PathBuf,
    },

    /// Serve a web gallery listing every demo under `dir` (front page at `/`,
    /// each demo browsable at `/d/<name>/`) — like `spex demos`, but a
    /// clickable browser index instead of a terminal listing.
    Gallery {
        #[arg(default_value = "demos")]
        dir: PathBuf,

        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Don't automatically open the default browser.
        #[arg(long)]
        no_open: bool,
    },

    /// Write a fully static, server-free copy of a demos gallery — same
    /// shape `spex gallery` serves dynamically, but every byte lives on disk
    /// (relative links/asset paths throughout) so it can be hosted by a
    /// plain static host like GitHub Pages with no backend at all.
    ExportStatic {
        #[arg(default_value = "demos")]
        dir: PathBuf,

        /// Output directory for the static site.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Interactively browse demos: move through the list, view a demo's
    /// tree inline, or press `w` to launch its web view — a k9s-style
    /// navigator instead of one-shot commands per demo.
    Nav {
        #[arg(default_value = "demos")]
        dir: PathBuf,
    },

    /// Render a tileset as a colored ASCII-art snapshot in the terminal —
    /// projects the real points through a simple camera (matching the
    /// viewer's default view) and maps luminance to a light/dark glyph ramp,
    /// colored by each cell's actual RGB. Works on any tileset, literal
    /// point clouds included.
    Ascii {
        tileset_dir: PathBuf,

        #[arg(long, default_value_t = 100)]
        width: usize,
    },

    /// Capture the REAL process tree on this machine (via `ps`) as a
    /// spex-graph JSON file: pid/ppid/%cpu/%mem/executable name only — no
    /// command-line arguments or usernames.
    PsTree {
        /// Only include this pid and its descendants. A real system's process
        /// tree is very wide (a process can have hundreds of direct children);
        /// scoping to a subtree gives a much smaller, more legible result.
        /// Find one with `pgrep <name>`, `ps aux | grep <name>`, or `echo $$`
        /// for your current shell.
        #[arg(long)]
        root: Option<u32>,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Write a fabricated example process tree (a minimal freshly-booted Linux
    /// system) as a spex-graph JSON file. Not read from any real machine —
    /// kept as a synthetic/offline fallback; prefer `ps-tree` for real data.
    PstreeDemo {
        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Run `brew deps --tree <formula>` and write it as a spex-graph JSON file.
    BrewDeps {
        formula: String,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Run `dpkg -s <package>` (Debian/Ubuntu) on a package and its direct
    /// dependencies and write them as a spex-graph JSON file: one level of
    /// real direct deps, not a full recursive apt tree. Only works on a real
    /// Debian/Ubuntu system.
    DebDeps {
        package: String,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Run `cargo tree -p <package>` (from the current directory, which
    /// must be inside a real Cargo project) and write it as a spex-graph
    /// JSON file: real subtree size (crate count) drives color.
    CargoDeps {
        package: String,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Run `npm ls --json --all` (from the current directory, which must be
    /// a real npm project with node_modules installed) and write it as a
    /// spex-graph JSON file: real subtree size (package count) drives color.
    NpmDeps {
        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Run `du -a -k <path>` and write the real filesystem tree as a
    /// spex-graph JSON file: size in KB drives color. The layout's fan-out
    /// cap keeps huge directories (build output, node_modules, ...) legible.
    DiskUsage {
        path: PathBuf,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Introspect a SQLite database's real schema (via the `sqlite3` CLI) and
    /// write it as a spex-graph JSON file: one node per table, row count
    /// driving color, columns + foreign keys as metadata. A table's first
    /// foreign key becomes its parent; tables with none are forest roots.
    SqlSchema {
        db: PathBuf,

        /// Output graph JSON path.
        #[arg(short, long)]
        out: PathBuf,
    },

    /// Parse a real SMILES string (or a known molecule name — run with no
    /// argument to list them) into a spex-graph JSON file: one node per
    /// atom, atomic number drives color, bonds become tree edges (a ring
    /// closure bond is kept as `ring_bond_to` metadata rather than a second
    /// tree parent, since `Graph` is tree-only).
    Molecule {
        /// A SMILES string (e.g. "c1ccccc1") or one of the known names
        /// (run with no argument to list them).
        smiles_or_name: Option<String>,

        /// Output graph JSON path. Required unless listing known molecules.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    reset_sigpipe();
    let cli = Cli::parse();
    match cli.command {
        Command::Info { input } => cmd_info(&input),
        Command::Convert {
            input,
            out,
            max_points_per_node,
            max_depth,
        } => cmd_convert(&input, &out, max_points_per_node, max_depth),
        Command::Serve {
            tileset_dir,
            port,
            no_open,
        } => cmd_serve(&tileset_dir, port, !no_open),
        Command::Trace { host, out } => cmd_trace(&host, &out),
        Command::GraphLayout { graph, out } => cmd_graph_layout(&graph, &out),
        Command::GraphPrint { graph } => cmd_graph_print(&graph),
        Command::GraphDiff { old, new } => cmd_graph_diff(&old, &new),
        Command::Demos { dir } => cmd_demos(&dir),
        Command::Gallery { dir, port, no_open } => cmd_gallery(&dir, port, !no_open),
        Command::ExportStatic { dir, out } => cmd_export_static(&dir, &out),
        Command::Nav { dir } => nav::run(&dir),
        Command::Ascii { tileset_dir, width } => cmd_ascii(&tileset_dir, width),
        Command::PsTree { root, out } => cmd_ps_tree(root, &out),
        Command::PstreeDemo { out } => cmd_pstree_demo(&out),
        Command::BrewDeps { formula, out } => cmd_brew_deps(&formula, &out),
        Command::DebDeps { package, out } => cmd_deb_deps(&package, &out),
        Command::CargoDeps { package, out } => cmd_cargo_deps(&package, &out),
        Command::NpmDeps { out } => cmd_npm_deps(&out),
        Command::DiskUsage { path, out } => cmd_disk_usage(&path, &out),
        Command::SqlSchema { db, out } => cmd_sql_schema(&db, &out),
        Command::Molecule { smiles_or_name, out } => cmd_molecule(smiles_or_name, out),
    }
}

/// Rust ignores SIGPIPE by default, turning "the reader closed the pipe"
/// (e.g. piping `spex graph-print` into `head`) into a write error that
/// `println!` then panics on. Restore the normal Unix behavior (the process
/// just exits) instead.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn cmd_info(input: &Path) -> Result<()> {
    let points = spex_io::read_points(input).with_context(|| format!("reading {}", input.display()))?;
    if points.is_empty() {
        println!("0 points");
        return Ok(());
    }
    let bounds = Aabb::from_points(points.iter().map(|p| p.position));
    let size = [
        bounds.max[0] - bounds.min[0],
        bounds.max[1] - bounds.min[1],
        bounds.max[2] - bounds.min[2],
    ];
    println!("points: {}", points.len());
    println!("bounds min: {:?}", bounds.min);
    println!("bounds max: {:?}", bounds.max);
    println!("size:       {:?}", size);
    Ok(())
}

fn cmd_convert(input: &Path, out: &Path, max_points_per_node: usize, max_depth: usize) -> Result<()> {
    println!("reading {}...", input.display());
    let points = spex_io::read_points(input).with_context(|| format!("reading {}", input.display()))?;
    println!("read {} points, building octree tileset...", points.len());
    let config = spex_tiler::TilerConfig {
        max_points_per_node,
        max_depth,
    };
    spex_tiler::build(points, out, &config)?;
    println!("wrote tileset to {}", out.display());
    Ok(())
}

fn cmd_serve(tileset_dir: &Path, port: u16, open_browser: bool) -> Result<()> {
    if !tileset_dir.join("tileset.json").exists() {
        bail!(
            "{} does not look like a tileset directory (no tileset.json found) — did you run `spex convert`?",
            tileset_dir.display()
        );
    }
    let config = spex_server::ServerConfig {
        tileset_dir: tileset_dir.to_path_buf(),
        port,
        open_browser,
    };
    spex_server::serve_blocking(config)
}

fn cmd_trace(host: &str, out: &Path) -> Result<()> {
    println!("running traceroute to {host}...");
    let graph = trace::run(host)?;
    println!("captured {} hops", graph.nodes.len() - 1);
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_ps_tree(root: Option<u32>, out: &Path) -> Result<()> {
    println!("running `ps` to capture the real process tree...");
    let graph = ps_tree::run(root)?;
    println!("captured {} processes", graph.nodes.len());
    if root.is_none() && graph.nodes.len() > 200 {
        println!(
            "note: that's a lot of processes for one view — consider `--root <pid>` to scope down to a subtree \
             (find one with `pgrep <name>` or `echo $$` for your shell)"
        );
    }
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_pstree_demo(out: &Path) -> Result<()> {
    let graph = pstree_demo::generate();
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote {} nodes to {}", graph.nodes.len(), out.display());
    Ok(())
}

fn cmd_brew_deps(formula: &str, out: &Path) -> Result<()> {
    println!("running `brew deps --tree {formula}`...");
    let graph = brew_deps::run(formula)?;
    println!("captured {} packages", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_deb_deps(package: &str, out: &Path) -> Result<()> {
    println!("running `dpkg -s {package}` and its direct dependencies...");
    let graph = deb_deps::run(package)?;
    println!("captured {} packages", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_cargo_deps(package: &str, out: &Path) -> Result<()> {
    println!("running `cargo tree -p {package}`...");
    let graph = cargo_deps::run(package)?;
    println!("captured {} crates", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_npm_deps(out: &Path) -> Result<()> {
    println!("running `npm ls --json --all`...");
    let graph = npm_deps::run()?;
    println!("captured {} packages", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_disk_usage(path: &Path, out: &Path) -> Result<()> {
    println!("running `du` on {}...", path.display());
    let graph = disk_usage::run(path)?;
    println!("captured {} filesystem entries", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_sql_schema(db: &Path, out: &Path) -> Result<()> {
    println!("running `sqlite3` against {}...", db.display());
    let graph = sql_schema::run(db)?;
    println!("captured {} tables", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_molecule(smiles_or_name: Option<String>, out: Option<PathBuf>) -> Result<()> {
    let Some(arg) = smiles_or_name else {
        println!("known molecules:");
        for (name, smiles) in molecule::KNOWN_MOLECULES {
            println!("  {name:<10} {smiles}");
        }
        println!("\nusage: spex molecule <name-or-smiles> -o <graph.json>");
        return Ok(());
    };
    let out = out.context("--out <graph.json> is required when parsing a molecule")?;
    let smiles = molecule::KNOWN_MOLECULES
        .iter()
        .find(|(name, _)| *name == arg)
        .map(|(_, smiles)| *smiles)
        .unwrap_or(&arg);
    println!("parsing SMILES {smiles:?}...");
    let graph = molecule::parse_smiles(smiles)?;
    println!("parsed {} atoms", graph.nodes.len());
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    graph.write_json(&out)?;
    println!("wrote graph to {}", out.display());
    Ok(())
}

fn cmd_graph_layout(graph_path: &Path, out: &Path) -> Result<()> {
    let graph = Graph::read_json(graph_path)?;
    let demo_title = graph.title.clone().unwrap_or_else(|| "spex demo".to_string());
    println!("laying out {} nodes...", graph.nodes.len());
    let layout = spex_graph::build(&graph);
    println!("generated {} points, building octree tileset...", layout.points.len());
    let offset = spex_tiler::build(layout.points, out, &spex_tiler::TilerConfig::default())?;

    // Shift node centers into the same offset-relative frame the tileset's
    // points are stored in, so the viewer can position labels correctly.
    let mut nodes = layout.nodes;
    for n in &mut nodes {
        n.center[0] -= offset[0];
        n.center[1] -= offset[1];
        n.center[2] -= offset[2];
    }
    let f = std::fs::File::create(out.join("nodes.json"))?;
    serde_json::to_writer_pretty(f, &nodes)?;

    let (mut metric_min, mut metric_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for n in &graph.nodes {
        if let Some(m) = n.metric {
            metric_min = metric_min.min(m);
            metric_max = metric_max.max(m);
        }
    }
    let has_metric = metric_max >= metric_min;
    let meta = GraphMeta {
        title: graph.title,
        metric_label: graph.metric_label,
        node_count: graph.nodes.len(),
        metric_min: has_metric.then_some(metric_min),
        metric_max: has_metric.then_some(metric_max),
    };
    let f = std::fs::File::create(out.join("meta.json"))?;
    serde_json::to_writer_pretty(f, &meta)?;

    // A static ASCII-art view (colored HTML, mirrors `spex ascii`'s terminal
    // rendering) written directly into the tileset directory — automatically
    // served/copied wherever the tileset itself is (spex-server's ServeDir,
    // `spex export-static`'s directory copy), no special-casing needed
    // anywhere else. Non-fatal if it fails; the tileset itself is already written.
    if let Ok(ascii_html) = ascii::run_html(out, 100, &demo_title) {
        let _ = std::fs::write(out.join("ascii.html"), ascii_html);
    }

    println!("wrote tileset to {} (+ nodes.json, meta.json, ascii.html)", out.display());
    Ok(())
}

/// Written alongside the tileset for the browser viewer's persistent
/// header/legend — the same "what am I looking at" info `graph-print` shows
/// in the terminal (see `spex_graph::format_tree`), just as small JSON.
#[derive(serde::Serialize)]
struct GraphMeta {
    title: Option<String>,
    #[serde(rename = "metricLabel")]
    metric_label: Option<String>,
    #[serde(rename = "nodeCount")]
    node_count: usize,
    #[serde(rename = "metricMin")]
    metric_min: Option<f64>,
    #[serde(rename = "metricMax")]
    metric_max: Option<f64>,
}

fn cmd_graph_print(graph_path: &Path) -> Result<()> {
    let graph = Graph::read_json(graph_path)?;
    print!("{}", spex_graph::format_tree(&graph));
    Ok(())
}

fn cmd_graph_diff(old: &Path, new: &Path) -> Result<()> {
    print!("{}", graph_diff::run(old, new)?);
    Ok(())
}

fn cmd_ascii(tileset_dir: &Path, width: usize) -> Result<()> {
    print!("{}", ascii::run(tileset_dir, width)?);
    Ok(())
}

/// A demo found under a demos root — shared by the terminal listing
/// (`spex demos`) and the web gallery (`spex gallery`).
struct DemoEntry {
    name: String,
    title: Option<String>,
    graph_path: PathBuf,
    tileset_dir: PathBuf,
    node_count: usize,
    web_ready: bool,
}

fn discover_demos(dir: &Path) -> Result<Vec<DemoEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut demos = Vec::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let graph_path = entry.path().join("graph.json");
        let tileset_dir = entry.path().join("tileset");
        if !graph_path.exists() {
            continue;
        }
        let graph = Graph::read_json(&graph_path).ok();
        let node_count = graph.as_ref().map(|g| g.nodes.len()).unwrap_or(0);
        let title = graph.and_then(|g| g.title);
        let web_ready = tileset_dir.join("tileset.json").exists();
        demos.push(DemoEntry { name, title, graph_path, tileset_dir, node_count, web_ready });
    }
    Ok(demos)
}

fn cmd_demos(dir: &Path) -> Result<()> {
    let demos = discover_demos(dir)?;
    if demos.is_empty() {
        println!("no demos found in {} yet", dir.display());
        println!("try: ./scripts/walkthrough.sh  (generates a handful of example demos)");
        return Ok(());
    }

    println!("available demos in {}:", dir.display());
    for demo in demos {
        println!("\n{}  ({} nodes)", demo.name, demo.node_count);
        println!("  json:     {}", demo.graph_path.display());
        println!("  terminal: spex graph-print {}", demo.graph_path.display());
        if demo.web_ready {
            println!("  web:      spex serve {}", demo.tileset_dir.display());
        } else {
            println!(
                "  web:      spex graph-layout {} -o {}   (then `spex serve` that dir)",
                demo.graph_path.display(),
                demo.tileset_dir.display()
            );
        }
    }
    Ok(())
}

fn cmd_gallery(dir: &Path, port: u16, open_browser: bool) -> Result<()> {
    let demos = discover_demos(dir)?;
    let ready: Vec<(String, PathBuf)> = demos
        .into_iter()
        .filter(|d| d.web_ready)
        .map(|d| (d.name, d.tileset_dir))
        .collect();
    if ready.is_empty() {
        bail!(
            "no web-ready demos found in {} — run `spex graph-layout` on a captured graph.json first, \
             or try ./scripts/walkthrough.sh",
            dir.display()
        );
    }
    spex_server::serve_gallery_blocking(spex_server::GalleryConfig {
        demos: ready,
        port,
        open_browser,
    })
}

fn cmd_export_static(dir: &Path, out: &Path) -> Result<()> {
    let demos = discover_demos(dir)?;
    let ready: Vec<(String, PathBuf)> = demos
        .into_iter()
        .filter(|d| d.web_ready)
        .map(|d| (d.name, d.tileset_dir))
        .collect();
    if ready.is_empty() {
        bail!(
            "no web-ready demos found in {} — run `spex graph-layout` on a captured graph.json first, \
             or try ./scripts/walkthrough.sh",
            dir.display()
        );
    }
    println!("exporting {} demo(s) to {}...", ready.len(), out.display());
    export_static::run(&ready, out)?;
    println!("wrote a static site to {} — serve it with any static file host, e.g.:", out.display());
    println!("  cd {} && python3 -m http.server 8000", out.display());
    Ok(())
}
