mod ply;
mod xyz;

use anyhow::{bail, Context, Result};
use spex_core::Point;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Reads a point cloud file, dispatching on extension. Supported: `.ply`, `.xyz`, `.csv`, `.txt`.
pub fn read_points(path: &Path) -> Result<Vec<Point>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::new(file);

    match ext.as_str() {
        "ply" => ply::read(reader),
        "xyz" | "csv" | "txt" => xyz::read(reader),
        other => bail!("unsupported point cloud format '.{other}' (supported: .ply, .xyz, .csv, .txt)"),
    }
}
