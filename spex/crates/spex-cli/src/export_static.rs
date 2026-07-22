//! `spex export-static` — writes a fully static, server-free copy of a
//! demos directory: same shape `spex gallery` serves dynamically
//! (`/` gallery index, `/d/<name>/` per-demo viewer + tileset), but every
//! byte lives on disk so it can be hosted by a plain static file host (e.g.
//! GitHub Pages) with no backend at all. All links/asset references are
//! relative, so the output works whether hosted at a domain root or under
//! a project-pages subpath.
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn run(demos: &[(String, PathBuf)], output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    for (name, tileset_dir) in demos {
        let demo_dir = output_dir.join("d").join(name);
        copy_dir_recursive(tileset_dir, &demo_dir.join("tileset"))
            .with_context(|| format!("copying tileset for demo {name}"))?;
        spex_server::write_viewer_assets(&demo_dir)
            .with_context(|| format!("writing viewer assets for demo {name}"))?;
    }

    let gallery_html = spex_server::render_gallery_html(demos);
    std::fs::write(output_dir.join("index.html"), gallery_html)?;

    Ok(())
}

/// Hand-rolled recursive copy — the source trees here (a tileset directory:
/// `tileset.json`, `octree/*.bin`, `nodes.json`, `meta.json`) are shallow
/// enough that pulling in a crate for this isn't worth it.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
