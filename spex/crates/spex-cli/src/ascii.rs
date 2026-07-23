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

/// Same rendering as `run`, but as a small self-contained HTML page (colored
/// via inline `<span style="color:...">`, monospace `<pre>`) instead of an
/// ANSI-colored terminal string — so the same ASCII-art view is browsable
/// outside a terminal too. Written alongside a tileset's own files (see
/// `main.rs`'s `cmd_graph_layout`), so it's automatically present wherever
/// that tileset is served or copied — no special-casing needed in
/// `spex-server`/`spex export-static`, which already serve/copy a tileset
/// directory's contents verbatim.
pub fn run_html(tileset_dir: &Path, width: usize, title: &str) -> Result<String> {
    let points = spex_tiler::read_points(tileset_dir)?;
    Ok(render_html(&points, width, title))
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

/// A camera on a horizontal ring around `bounds`'s center, at a fixed
/// elevation, looking at the center — a turntable orbit. `angle` is the
/// azimuth in radians; `angle = PI/4` reproduces [`default_camera`]'s exact
/// position (same radius/height, just parameterized), which is what makes
/// `default_camera` a thin wrapper around this rather than separate code.
fn orbit_camera(bounds: &Aabb, angle: f64) -> Camera {
    let center = bounds.center();
    let diag = bounds.diagonal().max(1.0);
    let radius = diag * 0.6 * std::f64::consts::SQRT_2;
    let height = diag * 0.6;
    let position = [center[0] + radius * angle.cos(), center[1] + height, center[2] + radius * angle.sin()];
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

/// Matches the viewer's default framing (`viewer/src/main.ts`'s initial
/// camera placement) so a terminal snapshot looks like the first thing
/// you'd see opening the same tileset in a browser: `position = center +
/// diagonal * 0.6` per axis, looking at `center`, world-up `(0,1,0)` (the
/// same convention three.js/OrbitControls default to).
fn default_camera(bounds: &Aabb) -> Camera {
    orbit_camera(bounds, std::f64::consts::FRAC_PI_4)
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

/// Projects `points` and crops to content (see `content_bounds`), returning
/// the grid plus its real width and the cropped row/col bounds — shared by
/// `render` (ANSI terminal) and `render_html` (browser) so both draw from
/// the exact same projection instead of duplicating it.
#[allow(clippy::type_complexity)]
fn project_cropped(points: &[Point], width: usize) -> Option<(Vec<Option<[u8; 3]>>, usize, usize, usize, usize)> {
    if points.is_empty() || width == 0 {
        return None;
    }
    let bounds = Aabb::from_points(points.iter().map(|p| p.position));
    let camera = default_camera(&bounds);
    let height = ((width as f64) * CHAR_ASPECT).round().max(1.0) as usize;
    let grid = project(points, &camera, width, height);

    // A sparse point cloud (a handful of blobs, say) only lights up a small
    // region of the full field-of-view grid — rendering the whole grid means
    // mostly blank space, which can scroll real content off a short terminal
    // entirely. Crop to what was actually drawn, plus a 1-cell margin.
    let (min_row, max_row, min_col, max_col) = content_bounds(&grid, width, height)?;
    let min_row = min_row.saturating_sub(1);
    let max_row = (max_row + 1).min(height - 1);
    let min_col = min_col.saturating_sub(1);
    let max_col = (max_col + 1).min(width - 1);
    Some((grid, min_row, max_row, min_col, max_col))
}

/// Renders one already-projected grid slice as ANSI text — shared by the
/// single-frame `render` and the multi-frame animation renderer so both
/// produce byte-identical formatting for the same cells.
fn grid_slice_to_ansi(grid: &[Option<[u8; 3]>], width: usize, min_row: usize, max_row: usize, min_col: usize, max_col: usize, use_color: bool) -> String {
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

fn render(points: &[Point], width: usize) -> String {
    let Some((grid, min_row, max_row, min_col, max_col)) = project_cropped(points, width) else {
        return String::new();
    };
    let use_color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    grid_slice_to_ansi(&grid, width, min_row, max_row, min_col, max_col, use_color)
}

/// Projects `points` from `frame_count` camera angles evenly spaced around a
/// full turntable orbit (see [`orbit_camera`]), then crops every frame to
/// the *union* of all frames' real content bounds rather than each frame's
/// own — so the crop window is stable across the animation instead of the
/// content jumping around as different angles light up different regions
/// of the grid. Returns `(grids, width, min_row, max_row, min_col, max_col)`.
#[allow(clippy::type_complexity)]
fn project_frames_cropped(points: &[Point], width: usize, frame_count: usize) -> Option<(Vec<Vec<Option<[u8; 3]>>>, usize, usize, usize, usize, usize)> {
    if points.is_empty() || width == 0 || frame_count == 0 {
        return None;
    }
    let bounds = Aabb::from_points(points.iter().map(|p| p.position));
    let height = ((width as f64) * CHAR_ASPECT).round().max(1.0) as usize;

    let grids: Vec<Vec<Option<[u8; 3]>>> = (0..frame_count)
        .map(|i| {
            let angle = (i as f64 / frame_count as f64) * std::f64::consts::TAU;
            let camera = orbit_camera(&bounds, angle);
            project(points, &camera, width, height)
        })
        .collect();

    let (mut min_row, mut max_row, mut min_col, mut max_col) = (height, 0, width, 0);
    let mut any = false;
    for grid in &grids {
        if let Some((r0, r1, c0, c1)) = content_bounds(grid, width, height) {
            any = true;
            min_row = min_row.min(r0);
            max_row = max_row.max(r1);
            min_col = min_col.min(c0);
            max_col = max_col.max(c1);
        }
    }
    if !any {
        return None;
    }
    let min_row = min_row.saturating_sub(1);
    let max_row = (max_row + 1).min(height - 1);
    let min_col = min_col.saturating_sub(1);
    let max_col = (max_col + 1).min(width - 1);
    Some((grids, width, min_row, max_row, min_col, max_col))
}

/// One ANSI-colored string per frame of a turntable orbit animation — same
/// crop window for every frame (see [`project_frames_cropped`]).
pub fn render_frames(points: &[Point], width: usize, frame_count: usize) -> Vec<String> {
    let Some((grids, width, min_row, max_row, min_col, max_col)) = project_frames_cropped(points, width, frame_count) else {
        return Vec::new();
    };
    let use_color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    grids.iter().map(|g| grid_slice_to_ansi(g, width, min_row, max_row, min_col, max_col, use_color)).collect()
}

/// Plays a turntable orbit animation directly in the terminal: clears the
/// screen and redraws each frame in place (`\x1b[2J\x1b[H`), pacing frames
/// at `fps`, for `loops` full orbits (`loops == 0` means forever, until the
/// process is interrupted — e.g. Ctrl-C).
pub fn run_animated(tileset_dir: &Path, width: usize, frame_count: usize, fps: f64, loops: usize) -> Result<()> {
    let points = spex_tiler::read_points(tileset_dir)?;
    let frames = render_frames(&points, width, frame_count);
    if frames.is_empty() {
        return Ok(());
    }
    let frame_delay = std::time::Duration::from_secs_f64(1.0 / fps.max(0.1));
    let mut loop_count = 0;
    loop {
        for frame in &frames {
            print!("\x1b[2J\x1b[H{frame}");
            use std::io::Write;
            std::io::stdout().flush().ok();
            std::thread::sleep(frame_delay);
        }
        loop_count += 1;
        if loops != 0 && loop_count >= loops {
            break;
        }
    }
    Ok(())
}

/// Renders one already-projected grid slice as an HTML body (`<span
/// style="color:...">` per lit cell) — shared by the single-frame
/// `render_html` and the multi-frame animated HTML export.
fn grid_slice_to_html_body(grid: &[Option<[u8; 3]>], width: usize, min_row: usize, max_row: usize, min_col: usize, max_col: usize) -> String {
    let mut body = String::new();
    for row in min_row..=max_row {
        for col in min_col..=max_col {
            match grid[row * width + col] {
                None => body.push(' '),
                Some(color) => {
                    let ch = luminance_to_char(color);
                    body.push_str(&format!(
                        "<span style=\"color:rgb({},{},{})\">{}</span>",
                        color[0],
                        color[1],
                        color[2],
                        html_escape_char(ch)
                    ));
                }
            }
        }
        body.push('\n');
    }
    body
}

fn render_html(points: &[Point], width: usize, title: &str) -> String {
    let Some((grid, min_row, max_row, min_col, max_col)) = project_cropped(points, width) else {
        return format!("<!doctype html><title>{title}</title><body style=\"background:#0b0e12\"></body>");
    };
    let body = grid_slice_to_html_body(&grid, width, min_row, max_row, min_col, max_col);

    format!(
        "<!doctype html>\n\
<html><head><meta charset=\"UTF-8\"><title>{title} — ASCII</title>\n\
<style>\n\
  html, body {{ margin: 0; background: #0b0e12; color: #e6e6e6; }}\n\
  pre {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 14px; line-height: 1.15; padding: 16px; white-space: pre; }}\n\
</style>\n\
</head><body><pre>{body}</pre></body></html>\n"
    )
}

/// A self-contained, dependency-free animated version of `render_html`: every
/// frame of a turntable orbit (see [`project_frames_cropped`]) pre-rendered
/// to an HTML body string, embedded as a JS array (`serde_json::to_string`
/// handles all the escaping — no hand-rolled JS-string quoting), and cycled
/// by a small inline `<script>` via `setInterval` swapping one `<pre>`'s
/// `innerHTML`. No canvas/WebGL — this is meant to be viewable literally
/// anywhere an HTML file renders, matching the plain-ASCII spirit of the
/// static view it's an animated sibling of.
pub fn run_html_animated(tileset_dir: &Path, width: usize, frame_count: usize, fps: f64, title: &str) -> Result<String> {
    let points = spex_tiler::read_points(tileset_dir)?;
    Ok(render_html_animated(&points, width, frame_count, fps, title))
}

fn render_html_animated(points: &[Point], width: usize, frame_count: usize, fps: f64, title: &str) -> String {
    let Some((grids, width, min_row, max_row, min_col, max_col)) = project_frames_cropped(points, width, frame_count) else {
        return format!("<!doctype html><title>{title}</title><body style=\"background:#0b0e12\"></body>");
    };
    let bodies: Vec<String> = grids.iter().map(|g| grid_slice_to_html_body(g, width, min_row, max_row, min_col, max_col)).collect();
    let frames_json = serde_json::to_string(&bodies).unwrap_or_else(|_| "[]".to_string());
    let interval_ms = (1000.0 / fps.max(0.1)).round() as u64;

    format!(
        "<!doctype html>\n\
<html><head><meta charset=\"UTF-8\"><title>{title} — ASCII (animated)</title>\n\
<style>\n\
  html, body {{ margin: 0; background: #0b0e12; color: #e6e6e6; }}\n\
  pre {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 14px; line-height: 1.15; padding: 16px; white-space: pre; }}\n\
</style>\n\
</head><body><pre id=\"f\"></pre>\n\
<script>\n\
  const frames = {frames_json};\n\
  let i = 0;\n\
  const el = document.getElementById('f');\n\
  el.innerHTML = frames[0] ?? '';\n\
  setInterval(() => {{ i = (i + 1) % frames.length; el.innerHTML = frames[i]; }}, {interval_ms});\n\
</script>\n\
</body></html>\n"
    )
}

fn html_escape_char(c: char) -> String {
    match c {
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '&' => "&amp;".to_string(),
        _ => c.to_string(),
    }
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

    #[test]
    fn orbit_camera_at_45_degrees_matches_default_camera() {
        let bounds = Aabb { min: [-10.0, -10.0, -10.0], max: [10.0, 10.0, 10.0] };
        let default = default_camera(&bounds);
        let orbited = orbit_camera(&bounds, std::f64::consts::FRAC_PI_4);
        for i in 0..3 {
            assert!((default.position[i] - orbited.position[i]).abs() < 1e-9, "axis {i}: {} vs {}", default.position[i], orbited.position[i]);
        }
    }

    #[test]
    fn render_frames_produces_the_requested_frame_count() {
        // A point off-center so different orbit angles actually project it
        // to different screen positions, not the same cell every frame.
        let points = vec![Point { position: [3.0, 0.0, 0.0], color: [255, 255, 255] }];
        let frames = render_frames(&points, 60, 8);
        assert_eq!(frames.len(), 8);
        assert!(frames.iter().all(|f| !f.is_empty()), "every frame should render real content");
        // Not every frame should be byte-identical — the camera really is
        // moving around the point between frames.
        assert!(frames.windows(2).any(|w| w[0] != w[1]), "frames should differ as the camera orbits");
    }

    #[test]
    fn render_frames_share_a_stable_crop_window_across_the_orbit() {
        let points = vec![Point { position: [3.0, 0.0, 0.0], color: [255, 255, 255] }];
        let frames = render_frames(&points, 60, 8);
        let widths: Vec<usize> = frames.iter().map(|f| f.lines().next().map(str::chars).map(Iterator::count).unwrap_or(0)).collect();
        assert!(widths.iter().all(|w| *w == widths[0]), "every frame should share the same crop width: {widths:?}");
    }

    #[test]
    fn render_html_animated_embeds_every_frame_as_real_json() {
        let points = vec![Point { position: [3.0, 0.0, 0.0], color: [200, 50, 50] }];
        let html = render_html_animated(&points, 60, 6, 10.0, "test");
        assert!(html.contains("const frames ="));
        assert!(html.contains("setInterval"));
        // The embedded JSON array should parse back into exactly 6 real strings.
        let json_start = html.find("const frames = ").unwrap() + "const frames = ".len();
        let json_end = html[json_start..].find(";\n").unwrap() + json_start;
        let parsed: Vec<String> = serde_json::from_str(&html[json_start..json_end]).unwrap();
        assert_eq!(parsed.len(), 6);
    }
}
