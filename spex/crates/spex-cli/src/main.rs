mod ascii;
mod brew_deps;
mod brick;
mod cargo_deps;
mod deb_deps;
mod disk_usage;
mod export_static;
mod frame_sequence;
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

    /// Tile N real point-cloud files (one per animation frame) into a real
    /// multi-tileset sequence that all share one coordinate offset, plus a
    /// sequence.json manifest — e.g. real parts converging into an assembly
    /// (see `spex brick-assembly`, which builds its frames in memory and
    /// calls this same shared-offset tiling core directly).
    FrameSequence {
        /// Input point-cloud files, in playback order.
        inputs: Vec<PathBuf>,

        /// Output directory (gets frame-000/, frame-001/, ..., sequence.json).
        #[arg(short, long)]
        out: PathBuf,

        /// Playback rate the viewer should advance frames at.
        #[arg(long, default_value_t = 6.0)]
        fps: f64,

        /// Max points sampled to represent a single octree node's LOD level.
        #[arg(long, default_value_t = 50_000)]
        max_points_per_node: usize,

        /// Hard cap on octree depth.
        #[arg(long, default_value_t = 16)]
        max_depth: usize,
    },

    /// Render one real Klemmbaustein/LEGO-compatible part (fetched live
    /// from https://ldraw.org, or read from a local LDraw library mirror)
    /// straight into an octree tileset — no intermediate point-cloud file.
    BrickPart {
        /// A known alias (run with no argument to list them) or a literal
        /// real LDraw part filename (e.g. "3005.dat").
        part: Option<String>,

        /// Real LDraw color code (see LDConfig.ldr) — default 4 = Red.
        #[arg(long, default_value_t = 4)]
        color: u32,

        #[arg(long, default_value_t = 3000)]
        points: usize,

        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Local cache directory for fetched real LDraw files.
        #[arg(long, default_value = ".ldraw-cache")]
        cache_dir: PathBuf,
    },

    /// Render a full real multi-part LDraw scene (a named official model
    /// fetched from ldraw.org's models/ folder, or a local .ldr file)
    /// straight into an octree tileset. Resolves each distinct real part
    /// exactly once no matter how many times the scene places it.
    BrickModel {
        /// A known model name (run with no argument to list them) or a
        /// local .ldr file path (e.g. "ldraw-scenes/monolith.ldr").
        model: Option<String>,

        #[arg(long, default_value_t = 20_000)]
        points: usize,

        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Local cache directory for fetched real LDraw files.
        #[arg(long, default_value = ".ldraw-cache")]
        cache_dir: PathBuf,
    },

    /// Animate any real LDraw scene's placements converging from a real,
    /// deliberately stylized scattered start into their real final
    /// positions — a real point-cloud animation (`spex serve` plays it
    /// back), not a hardcoded special case: any scene `spex brick-model`
    /// can render, this can animate.
    BrickAssembly {
        /// A known model name or a local .ldr file path (see `brick-model`).
        model: Option<String>,

        #[arg(long, default_value_t = 6_000)]
        points: usize,

        #[arg(long, default_value_t = 30)]
        frames: usize,

        #[arg(long, default_value_t = 6.0)]
        fps: f64,

        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Local cache directory for fetched real LDraw files.
        #[arg(long, default_value = ".ldraw-cache")]
        cache_dir: PathBuf,
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
    /// a build). Prints to the terminal, no new file written — unless
    /// `--merge -o <graph.json>` is given, which instead writes a single
    /// renderable graph with every node tagged added/removed/changed/
    /// unchanged (colored distinctly by `graph-layout`, viewable in 3D).
    GraphDiff {
        old: PathBuf,
        new: PathBuf,

        /// Write a merged, colorable graph.json instead of printing the
        /// terminal diff — the viewer half of diff/temporal mode.
        #[arg(long)]
        merge: bool,

        /// Output path for `--merge`'s graph JSON.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

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

        /// Play a turntable-orbit ASCII animation instead of one static
        /// frame — in the terminal, or (with --out) as a self-contained
        /// animated HTML file.
        #[arg(long)]
        animate: bool,

        /// Frames per full orbit, only used with --animate.
        #[arg(long, default_value_t = 24)]
        frames: usize,

        /// Playback speed in frames/sec, only used with --animate.
        #[arg(long, default_value_t = 10.0)]
        fps: f64,

        /// How many full orbits to play in the terminal before exiting (0 =
        /// forever, until interrupted). Ignored with --out.
        #[arg(long, default_value_t = 3)]
        loops: usize,

        /// Write an animated HTML file here instead of playing in the
        /// terminal (implies --animate).
        #[arg(long)]
        out: Option<PathBuf>,
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
        Command::FrameSequence {
            inputs,
            out,
            fps,
            max_points_per_node,
            max_depth,
        } => frame_sequence::run(&inputs, &out, fps, max_points_per_node, max_depth),
        Command::BrickPart {
            part,
            color,
            points,
            out,
            cache_dir,
        } => cmd_brick_part(part, color, points, out, &cache_dir),
        Command::BrickModel {
            model,
            points,
            out,
            cache_dir,
        } => cmd_brick_model(model, points, out, &cache_dir),
        Command::BrickAssembly {
            model,
            points,
            frames,
            fps,
            out,
            cache_dir,
        } => cmd_brick_assembly(model, points, frames, fps, out, &cache_dir),
        Command::Serve {
            tileset_dir,
            port,
            no_open,
        } => cmd_serve(&tileset_dir, port, !no_open),
        Command::Trace { host, out } => cmd_trace(&host, &out),
        Command::GraphLayout { graph, out } => cmd_graph_layout(&graph, &out),
        Command::GraphPrint { graph } => cmd_graph_print(&graph),
        Command::GraphDiff { old, new, merge, out } => cmd_graph_diff(&old, &new, merge, out),
        Command::Demos { dir } => cmd_demos(&dir),
        Command::Gallery { dir, port, no_open } => cmd_gallery(&dir, port, !no_open),
        Command::ExportStatic { dir, out } => cmd_export_static(&dir, &out),
        Command::Nav { dir } => nav::run(&dir),
        Command::Ascii { tileset_dir, width, animate, frames, fps, loops, out } => cmd_ascii(&tileset_dir, width, animate, frames, fps, loops, out),
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

fn cmd_brick_part(part: Option<String>, color: u32, points: usize, out: Option<PathBuf>, cache_dir: &Path) -> Result<()> {
    let Some(part) = part else {
        println!("known real LDraw part aliases:");
        for (alias, part_file) in brick::KNOWN_PARTS {
            println!("  {alias:<12} {part_file}");
        }
        println!("\nusage: spex brick-part <alias-or-part.dat> -o <tileset-dir>");
        return Ok(());
    };
    let out = out.context("--out <tileset-dir> is required when rendering a part")?;
    let part_file = brick::resolve_part_alias(&part);

    println!("resolving real LDraw part {part_file:?}...");
    let cache = spex_ldraw::LdrawCache::new(cache_dir);
    let cloud = brick::render_part_to_points(&cache, part_file, color, points, 0xC0FFEE)?;
    println!("sampled {} real points, building octree tileset...", cloud.len());

    spex_tiler::build(cloud, &out, &spex_tiler::TilerConfig::default())?;
    println!("wrote tileset to {}", out.display());
    Ok(())
}

fn cmd_brick_model(model: Option<String>, points: usize, out: Option<PathBuf>, cache_dir: &Path) -> Result<()> {
    let Some(model) = model else {
        println!("known real official LDraw models (or pass a local .ldr file path):");
        for name in brick::KNOWN_MODELS {
            println!("  {name}");
        }
        println!("\nusage: spex brick-model <name-or-path.ldr> -o <tileset-dir>");
        return Ok(());
    };
    let out = out.context("--out <tileset-dir> is required when rendering a model")?;
    let source = brick::resolve_model_source(&model);

    println!("parsing real LDraw scene {model:?}...");
    let cache = spex_ldraw::LdrawCache::new(cache_dir);
    let scene = spex_ldraw::parse_scene(&cache, source.as_model_source())?;
    let distinct_parts: std::collections::HashSet<&str> = scene.placements.iter().map(|p| p.part_file.as_str()).collect();
    println!(
        "parsed {} real placements ({} distinct real parts) from {:?} (author: {:?})",
        scene.placements.len(),
        distinct_parts.len(),
        scene.source_description,
        scene.source_author
    );

    let cloud = brick::render_scene_to_points(&cache, &scene, points, 0xC0FFEE)?;
    println!("sampled {} real points, building octree tileset...", cloud.len());

    spex_tiler::build(cloud, &out, &spex_tiler::TilerConfig::default())?;
    println!("wrote tileset to {}", out.display());
    Ok(())
}

fn cmd_brick_assembly(model: Option<String>, points: usize, frames: usize, fps: f64, out: Option<PathBuf>, cache_dir: &Path) -> Result<()> {
    let Some(model) = model else {
        println!("known real official LDraw models (or pass a local .ldr file path):");
        for name in brick::KNOWN_MODELS {
            println!("  {name}");
        }
        println!("\nusage: spex brick-assembly <name-or-path.ldr> -o <sequence-dir>");
        return Ok(());
    };
    let out = out.context("--out <sequence-dir> is required when animating an assembly")?;
    let source = brick::resolve_model_source(&model);

    println!("parsing real LDraw scene {model:?}...");
    let cache = spex_ldraw::LdrawCache::new(cache_dir);
    let scene = spex_ldraw::parse_scene(&cache, source.as_model_source())?;
    println!(
        "sampling {points} real points once across {} real placements, building {frames} real assembly frames...",
        scene.placements.len()
    );

    let point_frames = brick::build_assembly_frames(&cache, &scene, points, frames, 1337)?;
    let config = spex_tiler::TilerConfig::default();
    frame_sequence::run_from_frames(point_frames, &out, fps, &config)?;
    Ok(())
}

fn cmd_serve(tileset_dir: &Path, port: u16, open_browser: bool) -> Result<()> {
    // A real `spex frame-sequence` output has no root-level tileset.json —
    // just sequence.json plus a frame-NNN/ subdirectory per frame (each one
    // its own real tileset) — so accept either shape rather than assuming
    // every servable directory is a single plain tileset.
    if !tileset_dir.join("tileset.json").exists() && !tileset_dir.join("sequence.json").exists() {
        bail!(
            "{} does not look like a tileset directory (no tileset.json or sequence.json found) — did you run `spex convert` or `spex frame-sequence`?",
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
    // 140 columns at the HTML views' 14px monospace font (~8.4px/char for
    // Menlo/SF Mono-family fonts) renders to roughly 1180px wide plus
    // padding — comfortably fits a 1280px-wide viewport with a little
    // margin and fills a 1440/1920px one nicely too, without the
    // scroll-heavy sparseness a narrower default left on wider screens.
    const HTML_ASCII_WIDTH: usize = 140;
    if let Ok(ascii_html) = ascii::run_html(out, HTML_ASCII_WIDTH, &demo_title) {
        let _ = std::fs::write(out.join("ascii.html"), ascii_html);
    }
    // Same reasoning as the static ascii.html above — generated once here so
    // it rides along wherever the tileset itself goes, no special-casing in
    // spex-server/export-static. 24 frames at this width is still cheap for
    // a few-thousand-point demo tileset (projection is the only per-frame
    // cost, and it's a simple O(points) pass); non-fatal if it fails.
    if let Ok(ascii_animated_html) = ascii::run_html_animated(out, HTML_ASCII_WIDTH, 24, 8.0, &demo_title) {
        let _ = std::fs::write(out.join("ascii-animated.html"), ascii_animated_html);
    }

    println!("wrote tileset to {} (+ nodes.json, meta.json, ascii.html, ascii-animated.html)", out.display());
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

fn cmd_graph_diff(old: &Path, new: &Path, merge: bool, out: Option<PathBuf>) -> Result<()> {
    if merge {
        let out = out.context("--out <graph.json> is required with --merge")?;
        let old_graph = Graph::read_json(old)?;
        let new_graph = Graph::read_json(new)?;
        let merged = graph_diff::merge_for_viz(&old_graph, &new_graph);
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        merged.write_json(&out)?;
        println!("wrote merged diff graph ({} nodes) to {}", merged.nodes.len(), out.display());
        return Ok(());
    }
    print!("{}", graph_diff::run(old, new)?);
    Ok(())
}

fn cmd_ascii(tileset_dir: &Path, width: usize, animate: bool, frames: usize, fps: f64, loops: usize, out: Option<PathBuf>) -> Result<()> {
    if let Some(out) = out {
        let title = out.file_stem().and_then(|s| s.to_str()).unwrap_or("spex demo");
        let html = ascii::run_html_animated(tileset_dir, width, frames, fps, title)?;
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(&out, html)?;
        println!("wrote animated ASCII ({frames} frames) to {}", out.display());
        return Ok(());
    }
    if animate {
        return ascii::run_animated(tileset_dir, width, frames, fps, loops);
    }
    print!("{}", ascii::run(tileset_dir, width)?);
    Ok(())
}

/// What kind of real demo a `demos/<name>/` directory holds — determines
/// what `cmd_demos`'s terminal printout suggests, but not how it's served
/// (`cmd_gallery`/`cmd_export_static`/`render_gallery_html` are already
/// entirely tileset-dir-driven and need no kind-specific handling at all).
#[derive(Clone, Copy, PartialEq, Eq)]
enum DemoKind {
    /// A real `graph.json` + `graph-layout`'d tileset (the original,
    /// still-most-common shape).
    Graph,
    /// A plain point cloud (`spex convert`/`spex brick-part`/`brick-model`) —
    /// no graph, no tree, just points.
    PointCloud,
    /// A real multi-frame animation (`spex frame-sequence`/`brick-assembly`/
    /// `brick-cinematic`) — `sequence.json` + `frame-NNN/` subdirectories.
    Sequence,
}

/// A demo found under a demos root — shared by the terminal listing
/// (`spex demos`) and the web gallery (`spex gallery`).
struct DemoEntry {
    name: String,
    title: Option<String>,
    kind: DemoKind,
    graph_path: Option<PathBuf>,
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

        if graph_path.exists() {
            let graph = Graph::read_json(&graph_path).ok();
            let node_count = graph.as_ref().map(|g| g.nodes.len()).unwrap_or(0);
            let title = graph.and_then(|g| g.title);
            let web_ready = tileset_dir.join("tileset.json").exists();
            demos.push(DemoEntry {
                name,
                title,
                kind: DemoKind::Graph,
                graph_path: Some(graph_path),
                tileset_dir,
                node_count,
                web_ready,
            });
        } else if tileset_dir.join("sequence.json").exists() {
            demos.push(DemoEntry {
                name,
                title: None,
                kind: DemoKind::Sequence,
                graph_path: None,
                tileset_dir,
                node_count: 0,
                web_ready: true,
            });
        } else if tileset_dir.join("tileset.json").exists() {
            demos.push(DemoEntry {
                name,
                title: None,
                kind: DemoKind::PointCloud,
                graph_path: None,
                tileset_dir,
                node_count: 0,
                web_ready: true,
            });
        }
        // else: not a recognized demo shape (yet) — skip.
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
        match demo.kind {
            DemoKind::Graph => {
                let graph_path = demo.graph_path.expect("Graph demos always have a graph_path");
                println!("\n{}  ({} nodes)", demo.name, demo.node_count);
                println!("  json:     {}", graph_path.display());
                println!("  terminal: spex graph-print {}", graph_path.display());
                if demo.web_ready {
                    println!("  web:      spex serve {}", demo.tileset_dir.display());
                } else {
                    println!(
                        "  web:      spex graph-layout {} -o {}   (then `spex serve` that dir)",
                        graph_path.display(),
                        demo.tileset_dir.display()
                    );
                }
            }
            DemoKind::PointCloud => {
                println!("\n{}  (point cloud)", demo.name);
                println!("  web:      spex serve {}", demo.tileset_dir.display());
            }
            DemoKind::Sequence => {
                println!("\n{}  (animation)", demo.name);
                println!("  web:      spex serve {}", demo.tileset_dir.display());
            }
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

#[cfg(test)]
mod discover_demos_tests {
    use super::*;

    #[test]
    fn discover_demos_recognizes_all_three_real_demo_kinds() {
        let dir = tempfile::tempdir().unwrap();

        // A real graph demo: graph.json + tileset/tileset.json.
        let graph_demo = dir.path().join("a-graph-demo");
        std::fs::create_dir_all(graph_demo.join("tileset")).unwrap();
        std::fs::write(
            graph_demo.join("graph.json"),
            r#"{"title":"a real graph","nodes":[{"id":"n0","label":"root","parent":null}]}"#,
        )
        .unwrap();
        std::fs::write(graph_demo.join("tileset").join("tileset.json"), "{}").unwrap();

        // A real point-cloud demo: tileset/tileset.json only, no graph.json.
        let point_cloud_demo = dir.path().join("a-point-cloud-demo");
        std::fs::create_dir_all(point_cloud_demo.join("tileset")).unwrap();
        std::fs::write(point_cloud_demo.join("tileset").join("tileset.json"), "{}").unwrap();

        // A real sequence demo: tileset/sequence.json, no graph.json, no
        // top-level tileset.json (a sequence dir never has one).
        let sequence_demo = dir.path().join("a-sequence-demo");
        std::fs::create_dir_all(sequence_demo.join("tileset")).unwrap();
        std::fs::write(sequence_demo.join("tileset").join("sequence.json"), "{}").unwrap();

        // A directory that's neither — must be silently skipped, not error.
        std::fs::create_dir_all(dir.path().join("not-a-demo-at-all")).unwrap();

        let demos = discover_demos(dir.path()).unwrap();
        assert_eq!(demos.len(), 3, "the unrecognized directory must be skipped, not counted");

        let by_name = |name: &str| demos.iter().find(|d| d.name == name).unwrap_or_else(|| panic!("missing demo {name}"));

        let graph = by_name("a-graph-demo");
        assert!(graph.kind == DemoKind::Graph);
        assert!(graph.graph_path.is_some());
        assert_eq!(graph.title.as_deref(), Some("a real graph"));
        assert!(graph.web_ready);

        let point_cloud = by_name("a-point-cloud-demo");
        assert!(point_cloud.kind == DemoKind::PointCloud);
        assert!(point_cloud.graph_path.is_none());
        assert!(point_cloud.web_ready);

        let sequence = by_name("a-sequence-demo");
        assert!(sequence.kind == DemoKind::Sequence);
        assert!(sequence.graph_path.is_none());
        assert!(sequence.web_ready);
    }
}
