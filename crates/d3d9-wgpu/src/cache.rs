//! The driver's compiled pipelines kept between runs: wgpu's pipeline cache (Vulkan; wgpu has
//! none for the other backends), read from a file of the port's own when the device is made and
//! written back after a burst of new pipelines and at the end of the run. The folder is the
//! host's to choose (`NV_WGPU_CACHE`, which the launcher points into its `userdata\machine`);
//! without one nothing is read or written.
//!
//! What it gains: a start whose graphics driver has nothing cached for this program (its own
//! shader cache cleared or turned off, the exe moved or renamed, a driver with no cache of its
//! own) makes its pipelines from this file instead of compiling them again: Doc's house on
//! Vulkan about 5.8 s with nothing, 0.5 s with the file (#416's review). Where the driver's own
//! cache already holds them it changes little.
//!
//! The file is the cache's bytes in an envelope of ours: `NVWGPUPC`, a version, the length and a
//! fixed hash of the bytes ([`hash`], FNV-1a, not the map hasher, so the format never moves with
//! it). A missing, oversized, truncated, altered or foreign file (another adapter, backend or
//! driver: wgpu's own header says so) is one log line and a new, empty cache: never a failed
//! start. It is written to a temporary name of its own and renamed, so a run that ends while
//! writing leaves the old file or the new one, never half of one; a failed write removes its
//! temporary.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

const MAGIC: &[u8; 8] = b"NVWGPUPC";
const VERSION: u32 = 2;
const HEADER: usize = 8 + 4 + 8 + 8;
/// A file larger than this is not read (no driver's cache of one game comes near it).
const MAX_FILE: u64 = 256 << 20;
/// Presents after the last new pipeline before the cache is written: a burst of new pipelines
/// (a new place) is written once, after it.
const QUIET_PRESENTS: u64 = 300;
/// And at least this often while pipelines keep coming.
const SAVE_EVERY: u64 = 3600;

/// The envelope's hash: 64-bit FNV-1a, fixed by its definition.
pub(crate) fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
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

/// Writes `bytes` to `path` through a temporary file of this writer's own in the same folder
/// (another instance of the game, or this one's other writer, never shares it), renamed over it;
/// the temporary is removed when the write or the rename fails.
pub(crate) fn write_replacing(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
    let r = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// Removes the temporaries of writes that never finished (a process killed while writing), older
/// than ten minutes (another instance's write in progress is younger), of the files named `stems`
/// only: [`write_replacing`]'s own names, `STEM.PID-N.tmp`. The folder may be shared
/// (`NV_WGPU_CACHE` can name any folder), so another program's temporaries are left alone.
pub(crate) fn remove_stale_temporaries(dir: &Path, stems: &[&str]) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let ours = e
            .file_name()
            .to_str()
            .is_some_and(|name| own_temporary(name, stems));
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > 600);
        if ours && old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Whether `name` is a temporary [`write_replacing`] makes for a file whose name without its
/// extension is one of `stems`: `STEM.PID-N.tmp`, the process id and the writer's count in digits.
fn own_temporary(name: &str, stems: &[&str]) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let Some((stem, id)) = name.strip_suffix(".tmp").and_then(|r| r.rsplit_once('.')) else {
        return false;
    };
    let Some((pid, n)) = id.split_once('-') else {
        return false;
    };
    stems.contains(&stem) && digits(pid) && digits(n)
}

/// What writes the cache's file: shared by the device and the writer threads, and found by
/// [`save_all_now`] at the end of the run.
struct Saver {
    cache: wgpu::PipelineCache,
    path: PathBuf,
    /// The hash of the bytes last written or read: the same bytes are not written again.
    last: AtomicU64,
    /// One write at a time.
    writing: Mutex<()>,
    /// A write failed (a read-only folder): no more writes this run, so it is said once.
    failed: AtomicBool,
}

impl Saver {
    /// Takes the cache's bytes and writes them when they changed (on the calling thread).
    fn save(&self) {
        if self.failed.load(Ordering::Relaxed) {
            return;
        }
        let _one = self.writing.lock().unwrap_or_else(|e| e.into_inner());
        let t = std::time::Instant::now();
        let Some(data) = self.cache.get_data() else {
            return;
        };
        let h = hash(&data);
        if self.last.load(Ordering::Relaxed) == h {
            return;
        }
        match write_replacing(&self.path, &wrap(&data)) {
            Ok(()) => {
                self.last.store(h, Ordering::Relaxed);
                eprintln!(
                    "NV_D3D9 wgpu: pipeline cache {}: {} KB written ({:.1} ms, off the render thread)",
                    self.path.display(),
                    data.len() / 1024,
                    t.elapsed().as_secs_f64() * 1000.0
                );
            }
            Err(e) => {
                self.failed.store(true, Ordering::Relaxed);
                eprintln!(
                    "NV_D3D9 wgpu: pipeline cache {}: not written: {e}: none written this run",
                    self.path.display()
                );
            }
        }
    }
}

/// The devices' savers, for [`save_all_now`].
static SAVERS: Mutex<Vec<Weak<Saver>>> = Mutex::new(Vec::new());

/// Writes every live device's pipeline cache now, when it changed: for the end of the run (the
/// game's process ends with `std::process::exit`, so the devices are not dropped; the host calls
/// this from its exit hooks).
pub fn save_all_now() {
    let savers: Vec<Arc<Saver>> = SAVERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter_map(Weak::upgrade)
        .collect();
    for s in savers {
        s.save();
    }
}

/// The pipeline cache of the device and its file.
pub(crate) struct PipelineCacheFile {
    pub(crate) cache: wgpu::PipelineCache,
    saver: Arc<Saver>,
    /// What [`PipelineCacheFile::open`] made of the file (a log line's words; tests read it).
    pub(crate) note: String,
    /// The file was read and wgpu took it (the same adapter, backend and driver): the driver's
    /// compiled pipelines and libraries come back from it.
    pub(crate) warm: bool,
    /// Pipelines were made since the last save.
    dirty: std::cell::Cell<bool>,
    /// The present of the last new pipeline, and of the last save.
    touched_at: std::cell::Cell<u64>,
    /// The present of the first pipeline the file does not have yet.
    first_touched_at: std::cell::Cell<u64>,
}

impl PipelineCacheFile {
    /// The cache for `device` (made with `Features::PIPELINE_CACHE`), filled from the file in
    /// `dir` for this adapter when there is a usable one. None where the backend has no
    /// pipeline cache (wgpu names a file only for Vulkan) or the folder cannot be made.
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
        remove_stale_temporaries(dir, &[name.as_str()]);
        let path = dir.join(format!("{name}.bin"));
        let file = match std::fs::metadata(&path) {
            Ok(m) if m.len() > MAX_FILE => Err(format!("{} MB, too large", m.len() >> 20)),
            Ok(_) => std::fs::read(&path).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(String::new()),
            Err(e) => Err(e.to_string()),
        };
        let data = match &file {
            Ok(bytes) => unwrap(bytes).map_err(str::to_string),
            Err(e) => Err(e.clone()),
        };
        let empty = || {
            // SAFETY: no data: an empty cache.
            unsafe {
                device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                    label: Some("d3d9-wgpu pipelines"),
                    data: None,
                    fallback: true,
                })
            }
        };
        let (cache, note, read) = match data {
            Ok(d) => {
                // wgpu checks its own header (another adapter, backend or driver: refused) under
                // an error scope, so a refusal is said here instead of ending the run; then an
                // empty cache. SAFETY: the bytes are ones `PipelineCache::get_data` gave (the
                // envelope's hash says they arrived as written); wgpu checks its header, the
                // driver its own.
                let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
                let cache = unsafe {
                    device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                        label: Some("d3d9-wgpu pipelines"),
                        data: Some(d),
                        fallback: false,
                    })
                };
                match crate::device::block_on(scope.pop()) {
                    None => (cache, format!("{} KB read", d.len() / 1024), hash(d)),
                    Some(e) => {
                        // wgpu's error and its causes on one line ("... made for a different
                        // device").
                        let why = std::iter::successors(Some(&e as &dyn std::error::Error), |e| {
                            e.source()
                        })
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(": ");
                        let why = why.split_whitespace().collect::<Vec<_>>().join(" ");
                        (empty(), format!("{why}: ignored, a new one is started"), 0)
                    }
                }
            }
            Err(why) if why.is_empty() => (empty(), "none yet, a new one is started".into(), 0),
            Err(why) => (empty(), format!("{why}: ignored, a new one is started"), 0),
        };
        eprintln!("NV_D3D9 wgpu: pipeline cache {}: {note}", path.display());
        let saver = Arc::new(Saver {
            cache: cache.clone(),
            path,
            last: AtomicU64::new(read),
            writing: Mutex::new(()),
            failed: AtomicBool::new(false),
        });
        let mut all = SAVERS.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|w| w.strong_count() > 0);
        all.push(Arc::downgrade(&saver));
        Some(PipelineCacheFile {
            cache,
            saver,
            note,
            warm: read != 0,
            dirty: std::cell::Cell::new(false),
            touched_at: std::cell::Cell::new(0),
            first_touched_at: std::cell::Cell::new(0),
        })
    }

    /// A pipeline was made with the cache at present `presents`.
    pub(crate) fn touched(&self, presents: u64) {
        if !self.dirty.get() {
            self.first_touched_at.set(presents);
        }
        self.dirty.set(true);
        self.touched_at.set(presents);
    }

    /// At a `Present`: the cache written [`QUIET_PRESENTS`] after the last new pipeline (or
    /// [`SAVE_EVERY`] after the last save while they keep coming; at once with `now`). Both the
    /// bytes and the file are taken and written on a thread of their own: the render thread only
    /// starts it.
    pub(crate) fn save_if_due(&mut self, presents: u64, now: bool) {
        if !self.dirty.get() {
            return;
        }
        let quiet = presents.saturating_sub(self.touched_at.get()) >= QUIET_PRESENTS;
        let long = presents.saturating_sub(self.first_touched_at.get()) >= SAVE_EVERY;
        if !(now || quiet || long) {
            return;
        }
        self.dirty.set(false);
        let saver = self.saver.clone();
        if now {
            saver.save();
        } else {
            std::thread::spawn(move || saver.save());
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
        other[8] = 1;
        assert!(unwrap(&other).is_err());
    }

    #[test]
    fn the_envelope_hash_is_fnv_1a() {
        // FNV-1a 64's published values: the file format does not move with the map hasher.
        assert_eq!(hash(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hash(b"foobar"), 0x8594_4171_f739_67e8);
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
        // A "folder" that is a file: an error to log, not a panic; a target that is a folder:
        // the rename fails, and no temporary is left either way.
        assert!(write_replacing(&path.join("c.bin"), b"x").is_err());
        std::fs::create_dir_all(dir.join("d.bin")).unwrap();
        assert!(write_replacing(&dir.join("d.bin"), b"x").is_err());
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        let mut left = left;
        left.sort();
        assert_eq!(
            left,
            vec!["c.bin".to_string(), "d.bin".to_string()],
            "{left:?}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Only this port's own stale temporaries go: an old `STEM.PID-N.tmp` of a named stem. Another
    /// program's `.tmp` in a shared folder stays, so do a young one of ours (a write in progress)
    /// and the files themselves.
    #[test]
    fn only_our_own_stale_temporaries_are_removed() {
        let exe = std::env::current_exe().unwrap();
        let dir = exe
            .parent()
            .unwrap()
            .join(format!("d3d9-wgpu-temps-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(1200);
        let names = [
            ("pipelines.4242-0.tmp", true, false),
            (
                "wgpu_pipeline_cache_vulkan_4318_10137.77-3.tmp",
                true,
                false,
            ),
            ("pipelines.4242-1.tmp", false, true),
            ("foreign.tmp", true, true),
            ("other.4242-0.tmp", true, true),
            ("pipelines.tmp", true, true),
            ("pipelines.4242.tmp", true, true),
            ("pipelines.x-0.tmp", true, true),
            ("pipelines.txt", true, true),
        ];
        for (name, aged, _) in names {
            let f = std::fs::File::create(dir.join(name)).unwrap();
            if aged {
                f.set_modified(old).unwrap();
            }
        }
        remove_stale_temporaries(
            &dir,
            &["pipelines", "wgpu_pipeline_cache_vulkan_4318_10137"],
        );
        for (name, _, stays) in names {
            assert_eq!(dir.join(name).exists(), stays, "{name}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
