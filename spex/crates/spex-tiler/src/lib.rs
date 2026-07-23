use anyhow::{bail, Result};
use rand::rngs::StdRng;
use rand::seq::index::sample;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use spex_core::{child_id, Aabb, Point, ROOT_ID};
use std::collections::HashSet;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct TilerConfig {
    /// Max points sampled to represent a single node's LOD level.
    pub max_points_per_node: usize,
    /// Hard cap on octree depth, as a safety net against pathological inputs.
    pub max_depth: usize,
}

impl Default for TilerConfig {
    fn default() -> Self {
        TilerConfig {
            max_points_per_node: 50_000,
            max_depth: 16,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NodeManifestEntry {
    id: String,
    bounds: Aabb,
    #[serde(rename = "pointCount")]
    point_count: usize,
}

#[derive(Serialize, Deserialize)]
struct TilesetManifest {
    version: u32,
    offset: [f64; 3],
    bounds: Aabb,
    #[serde(rename = "pointCount")]
    point_count: usize,
    nodes: Vec<NodeManifestEntry>,
}

/// Builds a non-overlapping octree tileset from `points` and writes it to `out_dir`
/// as `tileset.json` + `octree/<node-id>.bin`. Each point is stored in exactly one
/// node (the node whose LOD level it was sampled to represent), so per-node point
/// counts sum to the total input point count.
/// Builds the tileset and returns the coordinate offset it subtracted from
/// every point (root bounds' min corner) — callers that have other data
/// positioned in the same original coordinate space (e.g. spex-graph's
/// per-node layout metadata) need this to keep everything aligned.
pub fn build(points: Vec<Point>, out_dir: &Path, config: &TilerConfig) -> Result<[f64; 3]> {
    build_with_offset(points, out_dir, config, None)
}

/// Same as `build`, but lets a caller force a specific coordinate offset
/// instead of always deriving one from this call's own bounding box.
/// Needed whenever several independently-built tilesets must share one
/// coordinate frame — e.g. a real multi-frame point-cloud animation (see
/// `unibrick/gen_monolith_assembly.py`), where each frame is tiled
/// separately but has to line up in the same world space so the viewer
/// can swap between them without every frame's origin jumping around.
/// `build` is the common case (`offset_override: None`) and is unaffected.
pub fn build_with_offset(
    points: Vec<Point>,
    out_dir: &Path,
    config: &TilerConfig,
    offset_override: Option<[f64; 3]>,
) -> Result<[f64; 3]> {
    if points.is_empty() {
        bail!("no points to tile");
    }
    let total_points = points.len();
    let global_bounds = Aabb::from_points(points.iter().map(|p| p.position));
    let offset = offset_override.unwrap_or(global_bounds.min);

    let octree_dir = out_dir.join("octree");
    fs::create_dir_all(&octree_dir)?;

    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    let mut nodes = Vec::new();
    build_node(
        ROOT_ID.to_string(),
        global_bounds,
        points,
        &offset,
        0,
        config,
        &octree_dir,
        &mut rng,
        &mut nodes,
    )?;

    let manifest = TilesetManifest {
        version: 1,
        offset,
        bounds: shift(&global_bounds, &offset),
        point_count: total_points,
        nodes,
    };
    let manifest_path = out_dir.join("tileset.json");
    let f = fs::File::create(&manifest_path)?;
    serde_json::to_writer_pretty(f, &manifest)?;
    Ok(offset)
}

/// Reads a tileset back into a flat `Vec<Point>`, in the tileset's own
/// offset-relative frame (the same frame `octree/*.bin` and `nodes.json`
/// already use) — the read-side counterpart to `build()`. Works on any
/// tileset, literal point clouds included, since it only knows the tileset
/// format itself, not where the points originally came from.
pub fn read_points(tileset_dir: &Path) -> Result<Vec<Point>> {
    let manifest_path = tileset_dir.join("tileset.json");
    let manifest: TilesetManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

    let mut points = Vec::with_capacity(manifest.point_count);
    for node in &manifest.nodes {
        let bin_path = tileset_dir.join("octree").join(format!("{}.bin", node.id));
        let data = fs::read(&bin_path)?;
        if data.len() < 4 {
            bail!("{} is too short to contain a point count", bin_path.display());
        }
        let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let mut offset = 4;
        for _ in 0..count {
            if offset + 15 > data.len() {
                bail!("{} is truncated", bin_path.display());
            }
            let x = f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as f64;
            let y = f32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()) as f64;
            let z = f32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap()) as f64;
            let color = [data[offset + 12], data[offset + 13], data[offset + 14]];
            points.push(Point { position: [x, y, z], color });
            offset += 15;
        }
    }
    Ok(points)
}

fn shift(b: &Aabb, offset: &[f64; 3]) -> Aabb {
    Aabb {
        min: sub(&b.min, offset),
        max: sub(&b.max, offset),
    }
}

fn sub(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[allow(clippy::too_many_arguments)]
fn build_node(
    id: String,
    bounds: Aabb,
    points: Vec<Point>,
    offset: &[f64; 3],
    depth: usize,
    config: &TilerConfig,
    octree_dir: &Path,
    rng: &mut StdRng,
    nodes: &mut Vec<NodeManifestEntry>,
) -> Result<()> {
    let n = points.len();
    let (sample_points, remaining): (Vec<Point>, Vec<Point>) = if n <= config.max_points_per_node {
        (points, Vec::new())
    } else {
        let chosen: HashSet<usize> = sample(rng, n, config.max_points_per_node).iter().collect();
        let mut sample_points = Vec::with_capacity(config.max_points_per_node);
        let mut remaining = Vec::with_capacity(n - config.max_points_per_node);
        for (i, p) in points.into_iter().enumerate() {
            if chosen.contains(&i) {
                sample_points.push(p);
            } else {
                remaining.push(p);
            }
        }
        (sample_points, remaining)
    };

    write_node_bin(octree_dir, &id, &sample_points, offset)?;
    nodes.push(NodeManifestEntry {
        id: id.clone(),
        bounds: shift(&bounds, offset),
        point_count: sample_points.len(),
    });

    if remaining.is_empty() || depth >= config.max_depth {
        return Ok(());
    }

    let mut buckets: [Vec<Point>; 8] = Default::default();
    for p in remaining {
        let octant = bounds.octant_index(&p.position);
        buckets[octant as usize].push(p);
    }

    for (octant, bucket) in buckets.into_iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }
        let child_bounds = bounds.octant_bounds(octant as u8);
        let cid = child_id(&id, octant as u8);
        build_node(cid, child_bounds, bucket, offset, depth + 1, config, octree_dir, rng, nodes)?;
    }
    Ok(())
}

/// Node file layout: u32 LE point count, then per point 3x f32 LE position
/// (relative to the tileset offset) + 3x u8 RGB — 15 bytes/point.
fn write_node_bin(dir: &Path, id: &str, points: &[Point], offset: &[f64; 3]) -> Result<()> {
    let path = dir.join(format!("{id}.bin"));
    let mut f = BufWriter::new(fs::File::create(&path)?);
    f.write_all(&(points.len() as u32).to_le_bytes())?;
    for p in points {
        let rel = [
            (p.position[0] - offset[0]) as f32,
            (p.position[1] - offset[1]) as f32,
            (p.position[2] - offset[2]) as f32,
        ];
        f.write_all(&rel[0].to_le_bytes())?;
        f.write_all(&rel[1].to_le_bytes())?;
        f.write_all(&rel[2].to_le_bytes())?;
        f.write_all(&p.color)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn synthetic_points(n: usize) -> Vec<Point> {
        (0..n)
            .map(|i| {
                let t = i as f64;
                Point {
                    position: [t % 100.0, (t * 1.3) % 100.0, (t * 0.7) % 100.0],
                    color: [(i % 256) as u8, 100, 150],
                }
            })
            .collect()
    }

    #[test]
    fn builds_tileset_with_conserved_point_count() {
        let dir = tempdir("conserved-count");
        let points = synthetic_points(200_000);
        let config = TilerConfig {
            max_points_per_node: 10_000,
            max_depth: 16,
        };
        build(points, &dir, &config).unwrap();

        let manifest_str = fs::read_to_string(dir.join("tileset.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
        assert_eq!(manifest["pointCount"].as_u64().unwrap(), 200_000);

        let nodes = manifest["nodes"].as_array().unwrap();
        let sum: u64 = nodes.iter().map(|n| n["pointCount"].as_u64().unwrap()).sum();
        assert_eq!(sum, 200_000);

        // Every node's .bin file should report the same point count as the manifest.
        for node in nodes {
            let id = node["id"].as_str().unwrap();
            let expected = node["pointCount"].as_u64().unwrap() as u32;
            let mut f = fs::File::open(dir.join("octree").join(format!("{id}.bin"))).unwrap();
            let mut count_buf = [0u8; 4];
            f.read_exact(&mut count_buf).unwrap();
            assert_eq!(u32::from_le_bytes(count_buf), expected);
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_points_round_trips_a_small_tileset() {
        let dir = tempdir("round-trip");
        let points = synthetic_points(50);
        let config = TilerConfig {
            max_points_per_node: 1000,
            max_depth: 16,
        };
        build(points.clone(), &dir, &config).unwrap();

        let read_back = read_points(&dir).unwrap();
        assert_eq!(read_back.len(), points.len());

        let mut expected: Vec<[f64; 3]> = points.iter().map(|p| p.position).collect();
        let mut actual: Vec<[f64; 3]> = read_back.iter().map(|p| p.position).collect();
        expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
        actual.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (e, a) in expected.iter().zip(actual.iter()) {
            for i in 0..3 {
                assert!((e[i] - a[i]).abs() < 1e-4, "expected {e:?} got {a:?}");
            }
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn empty_points_is_an_error() {
        let dir = tempdir("empty-points");
        let result = build(vec![], &dir, &TilerConfig::default());
        assert!(result.is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn exactly_at_budget_stays_a_single_leaf() {
        let dir = tempdir("exact-budget");
        let config = TilerConfig {
            max_points_per_node: 100,
            max_depth: 16,
        };
        let points = synthetic_points(100); // == max_points_per_node: should not split
        build(points, &dir, &config).unwrap();

        let manifest_str = fs::read_to_string(dir.join("tileset.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
        let nodes = manifest["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1, "100 points at a 100-point budget should be a single root leaf");
        assert_eq!(nodes[0]["id"].as_str().unwrap(), "r");
        assert_eq!(nodes[0]["pointCount"].as_u64().unwrap(), 100);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn build_with_offset_forces_the_given_offset_instead_of_deriving_one() {
        let dir = tempdir("forced-offset");
        let points = synthetic_points(50);
        let config = TilerConfig {
            max_points_per_node: 1000,
            max_depth: 16,
        };
        // Deliberately NOT this data's own bounding-box min, so a passing
        // test can only mean the override actually took effect.
        let forced_offset = [-1000.0, -2000.0, -3000.0];
        let returned_offset =
            build_with_offset(points.clone(), &dir, &config, Some(forced_offset)).unwrap();
        assert_eq!(returned_offset, forced_offset);

        let manifest_str = fs::read_to_string(dir.join("tileset.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
        let stored_offset: Vec<f64> = manifest["offset"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        assert_eq!(stored_offset, forced_offset.to_vec());

        // read_points() returns raw offset-relative positions (it doesn't
        // add the offset back) — so a correct forced offset means every
        // stored point equals (original position - forced_offset), not the
        // original position itself. Checking against that, rather than the
        // original coordinates directly, is what actually proves the
        // *forced* offset was subtracted, not some offset derived from the
        // data's own bounds (which would fail this check unless it happened
        // to coincide with forced_offset).
        let read_back = read_points(&dir).unwrap();
        let mut expected: Vec<[f64; 3]> = points.iter().map(|p| sub(&p.position, &forced_offset)).collect();
        let mut actual: Vec<[f64; 3]> = read_back.iter().map(|p| p.position).collect();
        expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
        actual.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (e, a) in expected.iter().zip(actual.iter()) {
            for i in 0..3 {
                assert!((e[i] - a[i]).abs() < 1e-4, "expected {e:?} got {a:?}");
            }
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    fn tempdir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("spex-tiler-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
