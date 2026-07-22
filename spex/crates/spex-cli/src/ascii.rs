//! `spex ascii` — a colored ASCII-art snapshot of a tileset, inspired by
//! https://github.com/tronicum/aa-bb-blkstn-cc (sample a grid, map each
//! cell's luminance to a glyph in a dark->light ramp, color the glyph with
//! the cell's real RGB). Unlike that project — which samples an
//! already-rendered 2D image — this projects real 3D point data through a
//! simple pinhole camera first, so overlaps need resolving by depth.
use anyhow::Result;
use spex_core::{Aabb, Point};
use std::io::IsTerminal;
use std::path::Path;

/// Same ramp (dark -> light) and character-aspect-correction factor as the
/// reference project — a direct homage, and both are reasonable choices on
/// their own merits.
const RAMP: &[char] = &[' ', '.', '\u{b7}', ':', ';', '=', '+', '*', 'x', '%', '#', '@'];
const CHAR_ASPECT: f64 = 0.62;
const FOV_Y_DEGREES: f64 = 60.0;

pub fn run(tileset_dir: &Path, width: usize) -> Result<String> {
    let points = spex_tiler::read_points(tileset_dir)?;
    Ok(render(&points, width))
}

struct Camera {
    position: [f64; 3],
    right: [f64; 3],
    up: [f64; 3],
    forward: [f64; 3],
    tan_half_fov: f64,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn normalize(a: [f64; 3]) -> [f64; 3] {
    let len = dot(a, a).sqrt();
    if len < 1e-9 {
        [0.0, 0.0, 1.0]
    } else {
        [a[0] / len, a[1] / len, a[2] / len]
    }
}

/// Matches the viewer's default framing (`viewer/src/main.ts`'s initial
/// camera placement) so a terminal snapshot looks like the first thing
/// you'd see opening the same tileset in a browser: `position = center +
/// diagonal * 0.6` per axis, looking at `center`, world-up `(0,1,0)` (the
/// same convention three.js/OrbitControls default to).
fn default_camera(bounds: &Aabb) -> Camera {
    let center = bounds.center();
    let diag = bounds.diagonal().max(1.0);
    let position = [center[0] + diag * 0.6, center[1] + diag * 0.6, center[2] + diag * 0.6];
    let forward = normalize(sub(center, position));
    let world_up = [0.0, 1.0, 0.0];
    let right = normalize(cross(forward, world_up));
    let up = cross(right, forward);
    Camera {
        position,
        right,
        up,
        forward,
        tan_half_fov: (FOV_Y_DEGREES.to_radians() / 2.0).tan(),
    }
}

/// Projects `points` through `camera` into a `width`-column grid (rows
/// derived via `CHAR_ASPECT`), keeping the nearest point per cell (a simple
/// single-pass z-buffer — no need to depth-sort the whole point list).
fn project(points: &[Point], camera: &Camera, width: usize, height: usize) -> Vec<Option<[u8; 3]>> {
    let mut zbuffer: Vec<Option<(f64, [u8; 3])>> = vec![None; width * height];

    for p in points {
        let rel = sub(p.position, camera.position);
        let cz = dot(rel, camera.forward);
        if cz <= 1e-6 {
            continue; // behind the camera
        }
        let cx = dot(rel, camera.right);
        let cy = dot(rel, camera.up);
        let sx = cx / (cz * camera.tan_half_fov);
        let sy = cy / (cz * camera.tan_half_fov);
        if !(-1.0..=1.0).contains(&sx) || !(-1.0..=1.0).contains(&sy) {
            continue; // outside the field of view
        }
        let col = (((sx + 1.0) / 2.0) * width as f64) as usize;
        let row = (((1.0 - sy) / 2.0) * height as f64) as usize;
        let col = col.min(width - 1);
        let row = row.min(height - 1);
        let idx = row * width + col;
        let should_replace = match zbuffer[idx] {
            None => true,
            Some((depth, _)) => cz < depth,
        };
        if should_replace {
            zbuffer[idx] = Some((cz, p.color));
        }
    }

    zbuffer.into_iter().map(|cell| cell.map(|(_, color)| color)).collect()
}

fn luminance_to_char(color: [u8; 3]) -> char {
    let lum = 0.2126 * color[0] as f64 + 0.7152 * color[1] as f64 + 0.0722 * color[2] as f64;
    let t = (lum / 255.0).clamp(0.0, 1.0);
    let idx = (t * (RAMP.len() - 1) as f64).round() as usize;
    RAMP[idx.min(RAMP.len() - 1)]
}

/// The bounding box (inclusive) of every lit cell in `grid`, or `None` if
/// nothing was drawn at all.
fn content_bounds(grid: &[Option<[u8; 3]>], width: usize, height: usize) -> Option<(usize, usize, usize, usize)> {
    let (mut min_row, mut max_row, mut min_col, mut max_col) = (height, 0, width, 0);
    let mut any = false;
    for row in 0..height {
        for col in 0..width {
            if grid[row * width + col].is_some() {
                any = true;
                min_row = min_row.min(row);
                max_row = max_row.max(row);
                min_col = min_col.min(col);
                max_col = max_col.max(col);
            }
        }
    }
    any.then_some((min_row, max_row, min_col, max_col))
}

fn render(points: &[Point], width: usize) -> String {
    if points.is_empty() || width == 0 {
        return String::new();
    }
    let bounds = Aabb::from_points(points.iter().map(|p| p.position));
    let camera = default_camera(&bounds);
    let height = ((width as f64) * CHAR_ASPECT).round().max(1.0) as usize;
    let grid = project(points, &camera, width, height);

    // A sparse point cloud (a handful of blobs, say) only lights up a small
    // region of the full field-of-view grid — rendering the whole grid means
    // mostly blank space, which can scroll real content off a short terminal
    // entirely. Crop to what was actually drawn, plus a 1-cell margin.
    let Some((min_row, max_row, min_col, max_col)) = content_bounds(&grid, width, height) else {
        return String::new();
    };
    let min_row = min_row.saturating_sub(1);
    let max_row = (max_row + 1).min(height - 1);
    let min_col = min_col.saturating_sub(1);
    let max_col = (max_col + 1).min(width - 1);

    let use_color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let mut out = String::new();
    for row in min_row..=max_row {
        for col in min_col..=max_col {
            match grid[row * width + col] {
                None => out.push(' '),
                Some(color) => {
                    let ch = luminance_to_char(color);
                    if use_color {
                        out.push_str(&format!("\x1b[38;2;{};{};{}m{ch}\x1b[0m", color[0], color[1], color[2]));
                    } else {
                        out.push(ch);
                    }
                }
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_maps_black_to_lightest_and_darkest_ramp_ends() {
        assert_eq!(luminance_to_char([0, 0, 0]), RAMP[0]);
        assert_eq!(luminance_to_char([255, 255, 255]), RAMP[RAMP.len() - 1]);
    }

    #[test]
    fn a_point_at_the_look_at_target_lands_near_grid_center() {
        let bounds = Aabb { min: [-10.0, -10.0, -10.0], max: [10.0, 10.0, 10.0] };
        let camera = default_camera(&bounds);
        let center = bounds.center();
        let point = Point { position: center, color: [200, 50, 50] };

        let grid = project(&[point], &camera, 21, 13);
        let hit_count = grid.iter().filter(|c| c.is_some()).count();
        assert_eq!(hit_count, 1, "exactly one cell should be lit");

        let idx = grid.iter().position(|c| c.is_some()).unwrap();
        let (row, col) = (idx / 21, idx % 21);
        // Should land close to the middle of the grid, not off in a corner.
        assert!((7..=13).contains(&col), "col {col} not near center");
        assert!((3..=9).contains(&row), "row {row} not near center");
    }

    #[test]
    fn empty_input_renders_to_empty_string() {
        assert_eq!(render(&[], 80), "");
    }

    #[test]
    fn render_dimensions_follow_char_aspect_correction() {
        let points = vec![Point { position: [0.0, 0.0, 0.0], color: [255, 255, 255] }];
        // Uncropped, a width=20 render has round(20*CHAR_ASPECT) rows; a
        // single point crops down to a tiny few-line snippet, so assert
        // against `project()`'s raw grid instead of `render()`'s output.
        let bounds = Aabb::from_points(points.iter().map(|p| p.position));
        let camera = default_camera(&bounds);
        let height = (20.0_f64 * CHAR_ASPECT).round() as usize;
        let grid = project(&points, &camera, 20, height);
        assert_eq!(grid.len(), 20 * height);

        let text = render(&points, 20);
        assert!(!text.is_empty());
        assert!(text.lines().count() < height, "a single point should crop to far fewer than {height} rows");
    }

    #[test]
    fn render_crops_to_content_instead_of_mostly_blank_space() {
        // A real render (see the traveling-salesman demo) had content on only
        // 15 of 62 rows before cropping — exactly the kind of output that
        // scrolls off a short terminal and looks like nothing rendered at all.
        let points = vec![Point { position: [0.0, 0.0, 0.0], color: [255, 255, 255] }];
        let text = render(&points, 100);
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.len() <= 5, "expected a tight crop around one point, got {} lines:\n{text}", lines.len());
        assert!(lines.iter().any(|l| !l.trim().is_empty()), "cropped output should still contain the point");
    }
}
