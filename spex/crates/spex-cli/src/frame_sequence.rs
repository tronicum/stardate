//! `spex frame-sequence` — tiles N real point-cloud files (each one frame of
//! a real animation, e.g. `unibrick/gen_monolith_assembly.py`'s per-frame
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
    let mut global_bounds = Aabb::empty();
    for input in inputs {
        let points =
            spex_io::read_points(input).with_context(|| format!("reading frame {}", input.display()))?;
        for p in &points {
            global_bounds.expand(&p.position);
        }
        per_frame_points.push(points);
    }
    let shared_offset = global_bounds.min;

    std::fs::create_dir_all(out_dir)?;
    let config = spex_tiler::TilerConfig {
        max_points_per_node,
        max_depth,
    };

    let mut frame_names = Vec::with_capacity(inputs.len());
    for (i, points) in per_frame_points.into_iter().enumerate() {
        let frame_name = format!("frame-{i:03}");
        let frame_dir = out_dir.join(&frame_name);
        spex_tiler::build_with_offset(points, &frame_dir, &config, Some(shared_offset))
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
