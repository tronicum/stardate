//! `spex frame-sequence` — tiles N real point-cloud files (each one frame of
//! a real animation, e.g. `spex brick-assembly`'s per-frame
//! `.xyz` snapshots of parts converging into a stacked assembly) into N real
//! octree tilesets that all share *one* coordinate offset, plus a small
//! `sequence.json` manifest the viewer can play back — real point clouds
//! through the *same* `spex-tiler`/`spex-server`/viewer pipeline every other
//! demo already uses, not a bespoke renderer.
//!
//! Sharing one offset across every frame's tileset is the entire reason this
//! is its own command rather than N plain `spex convert` calls: `build()`
//! derives its offset from that single call's own bounding box, so two
//! independently-converted point clouds — the same real object, at
//! different real positions — would each get a different local origin, and
//! the viewer would see the point cloud's coordinate frame silently jump
//! every time it switched frames. Computing one shared bounding box across
//! *all* frames first, then tiling every frame with `spex_tiler::build_with_offset`
//! forcing that same offset, keeps every frame in one consistent world space.
use anyhow::{Context, Result};
use serde::Serialize;
use spex_core::{Aabb, Point};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct SequenceManifest {
    version: u32,
    #[serde(rename = "frameCount")]
    frame_count: usize,
    fps: f64,
    frames: Vec<String>,
}

pub fn run(inputs: &[PathBuf], out_dir: &Path, fps: f64, max_points_per_node: usize, max_depth: usize) -> Result<()> {
    if inputs.is_empty() {
        anyhow::bail!("frame-sequence needs at least one input point-cloud file");
    }

    let mut per_frame_points: Vec<Vec<Point>> = Vec::with_capacity(inputs.len());
    for input in inputs {
        let points =
            spex_io::read_points(input).with_context(|| format!("reading frame {}", input.display()))?;
        per_frame_points.push(points);
    }

    let config = spex_tiler::TilerConfig {
        max_points_per_node,
        max_depth,
    };
    run_from_frames(per_frame_points, out_dir, fps, &config)
}

/// The real shared-offset-tiling core, taking already-in-memory frames
/// directly rather than file paths — used by `run` above (file-based
/// `spex frame-sequence`) and by `brick::cmd_brick_assembly` (which
/// generates its frames in memory via `spex-ldraw`, no intermediate files
/// at all). See this module's own doc comment for why every frame needs
/// to share one coordinate offset in the first place.
pub fn run_from_frames(frames: Vec<Vec<Point>>, out_dir: &Path, fps: f64, config: &spex_tiler::TilerConfig) -> Result<()> {
    if frames.is_empty() {
        anyhow::bail!("frame-sequence needs at least one frame");
    }

    let mut global_bounds = Aabb::empty();
    for points in &frames {
        for p in points {
            global_bounds.expand(&p.position);
        }
    }
    let shared_offset = global_bounds.min;

    std::fs::create_dir_all(out_dir)?;

    let mut frame_names = Vec::with_capacity(frames.len());
    for (i, points) in frames.into_iter().enumerate() {
        let frame_name = format!("frame-{i:03}");
        let frame_dir = out_dir.join(&frame_name);
        spex_tiler::build_with_offset(points, &frame_dir, config, Some(shared_offset))
            .with_context(|| format!("tiling {frame_name}"))?;
        frame_names.push(frame_name);
    }

    let manifest = SequenceManifest {
        version: 1,
        frame_count: frame_names.len(),
        fps,
        frames: frame_names,
    };
    let manifest_path = out_dir.join("sequence.json");
    let f = std::fs::File::create(&manifest_path)?;
    serde_json::to_writer_pretty(f, &manifest)?;

    println!(
        "wrote a {}-frame real sequence (shared offset {:?}) to {}",
        manifest.frame_count,
        shared_offset,
        out_dir.display()
    );
    Ok(())
}
