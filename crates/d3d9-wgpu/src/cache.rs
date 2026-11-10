//! The driver's compiled pipelines kept between runs, so a place seen before is not compiled
//! again: wgpu's pipeline cache (Vulkan; other backends have none), read from a file of the
//! port's own when the device is made and written back after new pipelines were made. The
//! folder is the host's to choose (`host::wgpu_cache_path`); without one nothing is read or
//! written.
//!
//! The file is the cache's bytes in an envelope of ours: `NVWGPUPC`, a version, the length and
//! a hash of the bytes. A missing, truncated, altered or foreign file is ignored with one log line
//! and a new cache is started (wgpu then checks its own header: another wgpu version, adapter or
//! driver starts empty too). It is written to a temporary name and renamed, so a run that ends
//! while writing leaves the old file or the new one, never half of one.

use crate::fxhash::FxHasher;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"NVWGPUPC";
const VERSION: u32 = 1;
const HEADER: usize = 8 + 4 + 8 + 8;
/// Presents between two saves at most (about ten seconds of play), so a burst of new pipelines
/// is written once, after it.
const SAVE_EVERY: u64 = 600;

fn hash(bytes: &[u8]) -> u64 {
    let mut h = FxHasher::default();
    h.write(bytes);
    h.finish()
}

/// The cache's bytes in the file's envelope.
pub(crate) fn wrap(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + data.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(&hash(data).to_le_bytes());
    out.extend_from_slice(data);
    out
}

/// The cache's bytes from a file's contents, or why they are not usable.
pub(crate) fn unwrap(file: &[u8]) -> Result<&[u8], &'static str> {
    if file.len() < HEADER || &file[..8] != MAGIC {
        return Err("not a pipeline cache file of this port");
    }
    let word = |at: usize| u64::from_le_bytes(file[at..at + 8].try_into().expect("8 bytes"));
    if u32::from_le_bytes(file[8..12].try_into().expect("4 bytes")) != VERSION {
        return Err("another version of the file");
    }
    let data = &file[HEADER..];
    if word(12) != data.len() as u64 {
        return Err("truncated or too long");
    }
    if word(20) != hash(data) {
        return Err("its bytes do not match their hash");
    }
    Ok(data)
}

/// Writes `bytes` to `path` through a temporary file in the same folder, renamed over it.
pub(crate) fn write_replacing(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// The pipeline cache of the device and its file.
pub(crate) struct PipelineCacheFile {
    pub(crate) cache: wgpu::PipelineCache,
    path: PathBuf,
    /// Pipelines were made since the last save.
    dirty: std::cell::Cell<bool>,
    saved_at: u64,
    /// A write failed (a read-only folder): no more writes this run, so it is said once.
    failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PipelineCacheFile {
    /// The cache for `device` (made with `Features::PIPELINE_CACHE`), filled from the file in
    /// `dir` for this adapter when there is a usable one. None where the backend has no
    /// pipeline cache (wgpu names a file only for Vulkan).
    pub(crate) fn open(
        device: &wgpu::Device,
        info: &wgpu::AdapterInfo,
        dir: &Path,
    ) -> Option<PipelineCacheFile> {
        if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
            return None;
        }
        let name = wgpu::util::pipeline_cache_key(info)?;
        // A folder that cannot be made (a read-only place, a path that is no folder) means no
        // cache, with one line: never a failed start. A relative path is taken from the working
        // folder, as any other path the host is given.
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!(
                "NV_D3D9 wgpu: pipeline cache folder {}: {e}: no cache this run",
                dir.display()
            );
            return None;
        }
        let path = dir.join(format!("{name}.bin"));
        let file = std::fs::read(&path);
        let data = match &file {
            Ok(bytes) => match unwrap(bytes) {
                Ok(d) => Some(d),
                Err(why) => {
                    eprintln!(
                        "NV_D3D9 wgpu: pipeline cache {}: {why}: ignored, a new one is started",
                        path.display()
                    );
                    None
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!(
                        "NV_D3D9 wgpu: pipeline cache {}: none yet, a new one is started",
                        path.display()
                    );
                } else {
                    eprintln!(
                        "NV_D3D9 wgpu: pipeline cache {}: {e}: ignored, a new one is started",
                        path.display()
                    );
                }
                None
            }
        };
        // SAFETY: the bytes are ones `PipelineCache::get_data` gave (the envelope's hash says
        // they arrived as written); wgpu checks its own header and the driver its own, and with
        // `fallback` a cache they refuse starts empty.
        let cache = unsafe {
            device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                label: Some("d3d9-wgpu pipelines"),
                data,
                fallback: true,
            })
        };
        if let Some(d) = data {
            eprintln!(
                "NV_D3D9 wgpu: pipeline cache {}: {} KB read",
                path.display(),
                d.len() / 1024
            );
        }
        Some(PipelineCacheFile {
            cache,
            path,
            dirty: std::cell::Cell::new(false),
            saved_at: 0,
            failed: Default::default(),
        })
    }

    /// A pipeline was made with the cache.
    pub(crate) fn touched(&self) {
        self.dirty.set(true);
    }

    /// At a `Present`: the cache written when pipelines were made since the last save and the
    /// last save is [`SAVE_EVERY`] presents old (or `now` is forced). The bytes are taken here;
    /// the file is written on a thread of its own, so the render thread does not wait for the
    /// disk.
    pub(crate) fn save_if_due(&mut self, presents: u64, now: bool) {
        if self.failed.load(std::sync::atomic::Ordering::Relaxed)
            || !self.dirty.get()
            || (!now && presents.saturating_sub(self.saved_at) < SAVE_EVERY) {
            return;
        }
        self.dirty.set(false);
        self.saved_at = presents;
        let t = std::time::Instant::now();
        let Some(data) = self.cache.get_data() else {
            return;
        };
        let taken = t.elapsed();
        let path = self.path.clone();
        let failed = self.failed.clone();
        let write = move || match write_replacing(&path, &wrap(&data)) {
            Ok(()) => eprintln!(
                "NV_D3D9 wgpu: pipeline cache {}: {} KB written (taken in {:.1} ms)",
                path.display(),
                data.len() / 1024,
                taken.as_secs_f64() * 1000.0
            ),
            Err(e) => {
                failed.store(true, std::sync::atomic::Ordering::Relaxed);
                eprintln!(
                    "NV_D3D9 wgpu: pipeline cache {}: not written: {e}: none written this run",
                    path.display()
                );
            }
        };
        if now {
            write();
        } else {
            std::thread::spawn(write);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_envelope_takes_back_only_what_it_wrote() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7) as u8).collect();
        let file = wrap(&data);
        assert_eq!(unwrap(&file), Ok(&data[..]));
        // Truncated, altered, foreign, empty, another version.
        assert!(unwrap(&file[..file.len() - 1]).is_err());
        let mut altered = file.clone();
        altered[HEADER + 10] ^= 1;
        assert!(unwrap(&altered).is_err());
        assert!(unwrap(b"WGPUPLCH and the rest of something else").is_err());
        assert!(unwrap(&[]).is_err());
        let mut other = file.clone();
        other[8] = 2;
        assert!(unwrap(&other).is_err());
    }

    #[test]
    fn a_replaced_file_is_whole_and_leaves_no_temporary() {
        // Beside the test binary (the build's own folder), not the user's temporary folder.
        let exe = std::env::current_exe().unwrap();
        let dir = exe
            .parent()
            .unwrap()
            .join(format!("d3d9-wgpu-cache-{}", std::process::id()));
        let path = dir.join("c.bin");
        write_replacing(&path, b"first").unwrap();
        write_replacing(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert!(!path.with_extension("tmp").exists());
        // A "folder" that is a file: an error to log, not a panic.
        assert!(write_replacing(&path.join("c.bin"), b"x").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
