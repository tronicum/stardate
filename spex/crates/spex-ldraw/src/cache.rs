//! Real LDraw fetching + on-disk caching — the network/I/O layer only. No
//! LDraw file-format parsing lives here (see `geometry.rs`/`scene.rs`).
use anyhow::{Context, Result};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const LDRAW_BASE: &str = "https://library.ldraw.org/library/official";
const LIBRARY_ZIP_URL: &str = "https://library.ldraw.org/library/updates/complete.zip";
const USER_AGENT: &str = "spex-brick/1.0 (educational project, github.com/tronicum/stardate)";

/// Fetches real LDraw files (`https://library.ldraw.org/library/official/...`),
/// preferring — in order — a per-file disk cache, a local `complete.zip`
/// full-library mirror (if downloaded via `download_library_zip`), and
/// finally a live HTTP fetch (retried with real exponential backoff on a
/// real HTTP 429 — ldraw.org genuinely rate-limits a burst of per-file
/// requests, e.g. resolving a real multi-part model needs a fetch per
/// distinct part plus its own subpart/primitive tree).
pub struct LdrawCache {
    pub cache_dir: PathBuf,
}

impl LdrawCache {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        LdrawCache {
            cache_dir: cache_dir.into(),
        }
    }

    fn cache_path(&self, path: &str) -> PathBuf {
        self.cache_dir.join(path)
    }

    fn library_zip_path(&self) -> PathBuf {
        self.cache_dir.join("complete.zip")
    }

    /// Fetches one real file's text content. Cached to disk at the same
    /// relative path regardless of which source (disk cache, zip mirror,
    /// or a live fetch) actually answered it, so a later call never cares
    /// which one it was.
    pub fn fetch(&self, path: &str) -> Result<String> {
        let cache_path = self.cache_path(path);
        if cache_path.exists() {
            return fs::read_to_string(&cache_path)
                .with_context(|| format!("reading cached {}", cache_path.display()));
        }

        let text = match self.read_from_library_zip(path)? {
            Some(text) => text,
            None => self.fetch_live(path)?,
        };
        self.write_cache(&cache_path, &text)?;
        Ok(text)
    }

    /// Reads one real file straight out of a local `complete.zip` mirror,
    /// if one has been downloaded — `None` if there's no mirror, or if this
    /// particular path isn't present in it (falls through to a live fetch).
    fn read_from_library_zip(&self, path: &str) -> Result<Option<String>> {
        let zip_path = self.library_zip_path();
        if !zip_path.exists() {
            return Ok(None);
        }
        let file = fs::File::open(&zip_path)
            .with_context(|| format!("opening {}", zip_path.display()))?;
        let mut archive = zip::ZipArchive::new(file)
            .with_context(|| format!("reading zip archive {}", zip_path.display()))?;
        let entry_name = format!("ldraw/{path}");
        let result = match archive.by_name(&entry_name) {
            Ok(mut entry) => {
                let mut text = String::new();
                entry.read_to_string(&mut text)?;
                Ok(Some(text))
            }
            Err(zip::result::ZipError::FileNotFound) => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {entry_name} from library mirror")),
        };
        result
    }

    fn fetch_live(&self, path: &str) -> Result<String> {
        let url = format!("{LDRAW_BASE}/{path}");
        let retries = 6;
        for attempt in 0..retries {
            match ureq::get(&url).header("User-Agent", USER_AGENT).call() {
                Ok(mut response) => {
                    return response
                        .body_mut()
                        .read_to_string()
                        .with_context(|| format!("reading response body for {url}"));
                }
                Err(ureq::Error::StatusCode(429)) if attempt + 1 < retries => {
                    let wait = 2u64.pow(attempt as u32);
                    eprintln!("  {path:?}: HTTP 429 (rate limited), retrying in {wait}s...");
                    thread::sleep(Duration::from_secs(wait));
                }
                Err(e) => return Err(e).with_context(|| format!("fetching {url}")),
            }
        }
        anyhow::bail!("exhausted retries fetching {url}")
    }

    fn write_cache(&self, cache_path: &Path, text: &str) -> Result<()> {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(cache_path, text)?;
        Ok(())
    }

    /// Downloads the real, official `complete.zip` (~136MB) once, so
    /// subsequent `fetch()` calls read straight out of it with zero
    /// network requests — the real fix for ldraw.org's real rate limit
    /// when resolving a multi-part model needs dozens of distinct file
    /// fetches in quick succession. Not automatic — an explicit opt-in
    /// step (a real ~136MB transfer shouldn't happen as a side effect of
    /// resolving one part).
    pub fn download_library_zip(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)?;
        let mut response = ureq::get(LIBRARY_ZIP_URL)
            .header("User-Agent", USER_AGENT)
            .config()
            .timeout_global(Some(Duration::from_secs(300)))
            .build()
            .call()
            .with_context(|| format!("fetching {LIBRARY_ZIP_URL}"))?;
        let zip_path = self.library_zip_path();
        let mut file = fs::File::create(&zip_path)
            .with_context(|| format!("creating {}", zip_path.display()))?;
        std::io::copy(&mut response.body_mut().as_reader(), &mut file)
            .with_context(|| format!("writing {}", zip_path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_reads_from_an_existing_disk_cache_without_any_network() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LdrawCache::new(dir.path());
        fs::create_dir_all(dir.path().join("parts")).unwrap();
        fs::write(dir.path().join("parts/3005.dat"), "0 Brick  1 x  1\n").unwrap();

        let text = cache.fetch("parts/3005.dat").unwrap();
        assert_eq!(text, "0 Brick  1 x  1\n");
    }

    #[test]
    fn fetch_reads_from_a_local_zip_mirror_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("complete.zip");
        {
            let file = fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer
                .start_file::<_, ()>("ldraw/parts/3005.dat", zip::write::FileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"0 Brick  1 x  1 (from mirror)\n").unwrap();
            writer.finish().unwrap();
        }

        let cache = LdrawCache::new(dir.path());
        let text = cache.fetch("parts/3005.dat").unwrap();
        assert_eq!(text, "0 Brick  1 x  1 (from mirror)\n");

        // Also proves the result got written through to the per-file disk
        // cache, same as a live fetch would.
        assert!(dir.path().join("parts/3005.dat").exists());
    }

    #[test]
    fn fetch_prefers_the_disk_cache_over_the_zip_mirror() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("complete.zip");
        {
            let file = fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            writer
                .start_file::<_, ()>("ldraw/parts/3005.dat", zip::write::FileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"from mirror\n").unwrap();
            writer.finish().unwrap();
        }
        fs::create_dir_all(dir.path().join("parts")).unwrap();
        fs::write(dir.path().join("parts/3005.dat"), "from disk cache\n").unwrap();

        let cache = LdrawCache::new(dir.path());
        let text = cache.fetch("parts/3005.dat").unwrap();
        assert_eq!(text, "from disk cache\n");
    }
}
