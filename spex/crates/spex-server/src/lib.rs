use anyhow::{Context, Result};
use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tower_http::services::ServeDir;

/// The viewer's built static assets (see `viewer/`), embedded into the binary
/// so `spex serve` is a single self-contained executable.
#[derive(RustEmbed)]
#[folder = "../../viewer/dist"]
struct ViewerAssets;

pub struct ServerConfig {
    pub tileset_dir: PathBuf,
    pub port: u16,
    pub open_browser: bool,
}

/// Starts the server and blocks until it exits. Spins up its own tokio
/// runtime so callers (the CLI) don't need to depend on tokio directly.
pub fn serve_blocking(config: ServerConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    rt.block_on(run(config))
}

/// Builds the router alone (no listener), so it can be exercised directly in
/// tests via `tower::ServiceExt::oneshot` without binding a real socket.
fn build_router(tileset_dir: &Path) -> Router {
    Router::new()
        .nest_service("/tileset", ServeDir::new(tileset_dir))
        .fallback(get(serve_viewer_asset))
}

async fn run(config: ServerConfig) -> Result<()> {
    let app = build_router(&config.tileset_dir);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding to {addr}"))?;
    let url = format!("http://{addr}");
    println!("spex serving {} at {url}", config.tileset_dir.display());

    if config.open_browser {
        let url = url.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            open_browser(&url);
        });
    }

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

/// A demos root: named tileset directories to serve side by side, with a
/// generated gallery page at `/` linking into each.
pub struct GalleryConfig {
    pub demos: Vec<(String, PathBuf)>,
    pub port: u16,
    pub open_browser: bool,
}

pub fn serve_gallery_blocking(config: GalleryConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    rt.block_on(run_gallery(config))
}

/// One `nest_service` per demo (the set is fixed at startup, so no dynamic
/// axum path params are needed) plus a pre-rendered gallery page at `/`.
/// Everything else — including `/d/<name>/` itself, which isn't separately
/// registered — falls through to the same embedded viewer SPA used by
/// `build_router`, unmodified.
fn build_gallery_router(demos: &[(String, PathBuf)]) -> Router {
    let mut router = Router::new();
    for (name, tileset_dir) in demos {
        router = router.nest_service(&format!("/d/{name}/tileset"), ServeDir::new(tileset_dir));
    }
    let gallery_html = render_gallery_html(demos);
    router
        .route("/", get(move || async move { Html(gallery_html) }))
        .fallback(get(serve_viewer_asset))
}

async fn run_gallery(config: GalleryConfig) -> Result<()> {
    let app = build_gallery_router(&config.demos);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding to {addr}"))?;
    let url = format!("http://{addr}");
    println!("spex serving {} demo(s) at {url}", config.demos.len());

    if config.open_browser {
        let url = url.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            open_browser(&url);
        });
    }

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

#[derive(Deserialize, Default)]
struct TilesetSummary {
    #[serde(rename = "pointCount")]
    point_count: Option<u64>,
}

#[derive(Deserialize, Default)]
struct MetaSummary {
    title: Option<String>,
    #[serde(rename = "nodeCount")]
    node_count: Option<usize>,
}

/// Pre-rendered gallery front page listing every demo, linking into each via
/// a *relative* href (`d/<name>/`, not `/d/<name>/`) — this resolves
/// correctly whether served at a domain root (`spex gallery`) or under a
/// subpath (a static export hosted on e.g. GitHub Pages at
/// `username.github.io/reponame/`). `pub` so `spex-cli`'s static-export
/// command can reuse it verbatim instead of duplicating the HTML.
pub fn render_gallery_html(demos: &[(String, PathBuf)]) -> String {
    let mut cards = String::new();
    for (name, tileset_dir) in demos {
        let tileset: TilesetSummary = std::fs::read_to_string(tileset_dir.join("tileset.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let meta: MetaSummary = std::fs::read_to_string(tileset_dir.join("meta.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let title = meta.title.unwrap_or_else(|| name.clone());
        let mut stats = Vec::new();
        if let Some(n) = meta.node_count {
            stats.push(format!("{n} nodes"));
        }
        if let Some(p) = tileset.point_count {
            stats.push(format!("{p} points"));
        }

        let name_escaped = escape_html(name);
        cards.push_str(&format!(
            r#"<div class="card-wrap">
<a class="card" href="d/{name_escaped}/">
  <h2>{name_escaped}</h2>
  <p class="title">{title_escaped}</p>
  <div class="stats">{stats_escaped}</div>
</a>
<a class="card-ascii" href="d/{name_escaped}/tileset/ascii.html" title="colored ASCII-art view, same as `spex ascii` in a terminal">ascii</a>
</div>
"#,
            title_escaped = escape_html(&title),
            stats_escaped = escape_html(&stats.join(" · ")),
        ));
    }

    let has_demos = !demos.is_empty();
    if cards.is_empty() {
        cards.push_str(r#"<p class="empty">No demos found yet — run <code>./scripts/walkthrough.sh</code>.</p>"#);
    }

    let cycle_link = if has_demos {
        r##"<a id="cycle-link" class="cycle-btn" href="javascript:void(0)">&#9654; cycle through demos</a>"##
    } else {
        ""
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<title>spex — demos</title>
<style>
  html, body {{ margin: 0; background: #0b0e12; color: #e6e6e6; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
  body {{ padding: 32px; }}
  .header {{ display: flex; align-items: center; justify-content: space-between; max-width: 1000px; margin-bottom: 24px; }}
  h1 {{ font-size: 20px; font-weight: 600; margin: 0; }}
  .cycle-btn {{ color: inherit; text-decoration: none; background: rgba(255,255,255,0.08); padding: 8px 14px; border-radius: 8px; font-size: 13px; transition: background 0.15s; }}
  .cycle-btn:hover {{ background: rgba(255,255,255,0.16); }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 16px; max-width: 1000px; }}
  .card-wrap {{ position: relative; }}
  .card {{ display: block; background: rgba(255,255,255,0.06); border-radius: 10px; padding: 16px; text-decoration: none; color: inherit; transition: background 0.15s; }}
  .card:hover {{ background: rgba(255,255,255,0.12); }}
  .card h2 {{ margin: 0 0 6px; font-size: 15px; font-weight: 600; }}
  .card .title {{ margin: 0 0 10px; font-size: 12px; opacity: 0.7; }}
  .card .stats {{ font-size: 11px; opacity: 0.5; }}
  .card-ascii {{ position: absolute; top: 10px; right: 10px; font-size: 10px; color: inherit; text-decoration: none; background: rgba(255,255,255,0.1); padding: 2px 7px; border-radius: 4px; opacity: 0.7; }}
  .card-ascii:hover {{ opacity: 1; background: rgba(255,255,255,0.2); }}
  .empty {{ opacity: 0.6; }}
  code {{ background: rgba(255,255,255,0.1); padding: 2px 6px; border-radius: 4px; }}
</style>
</head>
<body>
<div class="header">
  <h1>spex demos</h1>
  {cycle_link}
</div>
<div class="grid">
{cards}</div>
<script>
(function () {{
  var link = document.getElementById('cycle-link');
  if (!link) return;
  link.addEventListener('click', function (e) {{
    e.preventDefault();
    var cards = document.querySelectorAll('.card');
    if (!cards.length) return;
    var pick = cards[Math.floor(Math.random() * cards.length)];
    window.location.href = pick.getAttribute('href') + '?cycle=1';
  }});
}})();
</script>
</body>
</html>
"#
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let cmd = ("open", vec![url.to_string()]);
    #[cfg(target_os = "linux")]
    let cmd = ("xdg-open", vec![url.to_string()]);
    #[cfg(target_os = "windows")]
    let cmd = ("cmd", vec!["/C".to_string(), "start".to_string(), url.to_string()]);

    let _ = std::process::Command::new(cmd.0).args(cmd.1).status();
}

/// Writes every embedded viewer asset (the same `viewer/dist` this crate
/// serves at runtime via `serve_viewer_asset`) out to real files under
/// `output_dir` — used by `spex export-static` to give each demo folder in a
/// static export its own self-contained copy of the viewer (no server
/// needed to resolve them at request time).
pub fn write_viewer_assets(output_dir: &Path) -> Result<()> {
    for path in ViewerAssets::iter() {
        let Some(content) = ViewerAssets::get(&path) else { continue };
        let dest = output_dir.join(path.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, content.data.as_ref()).with_context(|| format!("writing {}", dest.display()))?;
    }
    Ok(())
}

/// Gallery mode serves every demo under `/d/<name>/`, but they all share one
/// embedded viewer bundle (unlike a static export, where each demo folder
/// gets its own copy) — the built `index.html` references assets via a
/// *relative* path (`./assets/...`, see `viewer/vite.config.ts`), which the
/// browser resolves against `/d/<name>/`, producing a request like
/// `/d/<name>/assets/index-HASH.js`. Strip that per-demo prefix so it still
/// resolves against the flat embedded keys (`assets/index-HASH.js`).
fn resolve_viewer_asset_path(raw: &str) -> &str {
    let path = raw
        .strip_prefix("d/")
        .and_then(|rest| rest.split_once('/'))
        .map(|(_name, rest)| rest)
        .unwrap_or(raw);
    if path.is_empty() {
        "index.html"
    } else {
        path
    }
}

async fn serve_viewer_asset(uri: Uri) -> Response {
    let raw = uri.path().trim_start_matches('/');
    let path = resolve_viewer_asset_path(raw);

    if let Some(content) = ViewerAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data.into_owned()))
            .unwrap();
    }
    // SPA fallback: unknown paths (or a missing build) serve index.html if present.
    match ViewerAssets::get("index.html") {
        Some(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data.into_owned()))
            .unwrap(),
        None => (StatusCode::NOT_FOUND, "viewer assets not built — run `npm run build` in viewer/").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;

    fn temp_tileset_dir(unique: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("spex-server-test-{unique}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tileset.json"), br#"{"version":1}"#).unwrap();
        dir
    }

    #[tokio::test]
    async fn serves_a_real_file_under_tileset_prefix() {
        let dir = temp_tileset_dir("real-file");
        let app = build_router(&dir);

        let response = app
            .oneshot(Request::builder().uri("/tileset/tileset.json").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], br#"{"version":1}"#);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn missing_file_under_tileset_prefix_is_a_real_404() {
        let dir = temp_tileset_dir("missing-file");
        let app = build_router(&dir);

        let response = app
            .oneshot(Request::builder().uri("/tileset/does-not-exist.bin").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // ServeDir's own 404, not the SPA fallback — a request for tileset
        // data that isn't there should look like a missing file, not a web page.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_viewer_asset_path_strips_the_per_demo_prefix() {
        assert_eq!(resolve_viewer_asset_path("d/bigmac/assets/index-HASH.js"), "assets/index-HASH.js");
        assert_eq!(resolve_viewer_asset_path("d/bigmac/"), "index.html");
        // No trailing slash: not a recognized demo-prefixed path, so it's
        // left alone — the caller's ViewerAssets lookup will miss and fall
        // through to its own SPA-fallback branch, still serving index.html
        // end to end, just not via this function's return value directly.
        assert_eq!(resolve_viewer_asset_path("d/bigmac"), "d/bigmac");
    }

    #[test]
    fn resolve_viewer_asset_path_leaves_non_demo_paths_alone() {
        assert_eq!(resolve_viewer_asset_path("assets/index-HASH.js"), "assets/index-HASH.js");
        assert_eq!(resolve_viewer_asset_path("some/spa/route"), "some/spa/route");
        assert_eq!(resolve_viewer_asset_path(""), "index.html");
        // A path that merely starts with "d" but not "d/" must not be mistaken for the demo prefix.
        assert_eq!(resolve_viewer_asset_path("decix-trace-notes.txt"), "decix-trace-notes.txt");
    }

    #[tokio::test]
    async fn a_demos_asset_request_resolves_to_the_shared_embedded_bundle() {
        let dir = temp_tileset_dir("demo-asset");
        let demos = vec![("bigmac".to_string(), dir.clone())];
        let app = build_gallery_router(&demos);

        // Whatever the real hashed asset filename is, a request for it under
        // /d/<name>/assets/... must resolve to the same bytes as requesting
        // it directly — proving the per-demo prefix strip actually works,
        // not just that *some* fallback fired.
        let Some(asset_path) = ViewerAssets::iter().find(|p| p.starts_with("assets/")) else {
            // dist/ not built in this environment — nothing to assert against.
            return;
        };

        let direct = app
            .clone()
            .oneshot(Request::builder().uri(format!("/{asset_path}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let nested = app
            .oneshot(Request::builder().uri(format!("/d/bigmac/{asset_path}")).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(direct.status(), StatusCode::OK);
        assert_eq!(nested.status(), StatusCode::OK);
        let direct_body = axum::body::to_bytes(direct.into_body(), usize::MAX).await.unwrap();
        let nested_body = axum::body::to_bytes(nested.into_body(), usize::MAX).await.unwrap();
        assert_eq!(direct_body, nested_body, "nested per-demo asset request should serve identical bytes");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_viewer_assets() {
        let dir = temp_tileset_dir("fallback");
        let app = build_router(&dir);

        let response = app
            .oneshot(Request::builder().uri("/some/spa/route").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Whether this is 200 (real dist/index.html embedded) or 404 (dist
        // never built) depends on build state, but it must go through the
        // fallback handler, not a raw framework 404 — a 500 here would mean
        // the handler panicked.
        assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_demo_dir(unique: &str, title: &str, node_count: usize, point_count: u64) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("spex-server-gallery-test-{unique}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tileset.json"), format!(r#"{{"pointCount":{point_count}}}"#)).unwrap();
        std::fs::write(dir.join("meta.json"), format!(r#"{{"title":"{title}","nodeCount":{node_count}}}"#)).unwrap();
        dir
    }

    #[tokio::test]
    async fn gallery_index_lists_each_demo() {
        let dir = temp_demo_dir("index", "a real traceroute", 9, 3540);
        let demos = vec![("decix-trace".to_string(), dir.clone())];
        let app = build_gallery_router(&demos);

        let response = app.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();

        assert!(html.contains("decix-trace"), "should list the demo name");
        assert!(html.contains("a real traceroute"), "should show its title");
        assert!(html.contains("9 nodes"), "should show its node count");
        assert!(
            html.contains("href=\"d/decix-trace/\""),
            "should link into the demo with a relative href (subpath-hosting safe)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn gallery_serves_each_demos_tileset_under_its_own_path() {
        let dir = temp_demo_dir("tileset", "some demo", 1, 100);
        let demos = vec![("my-demo".to_string(), dir.clone())];
        let app = build_gallery_router(&demos);

        let response = app
            .oneshot(Request::builder().uri("/d/my-demo/tileset/tileset.json").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(String::from_utf8(body.to_vec()).unwrap().contains("100"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
