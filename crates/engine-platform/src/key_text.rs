//! Layout-aware characters observed on the native window thread.
//!
//! ShowOff's ToANSIChar consumer in MCM asks about the key whose real input
//! event just arrived. Windows measures that event with its original keyboard
//! APIs; Linux/macOS use winit's actual layout text and modifier-free key.
//! Unobserved keys and unsupported/dead-key combinations return unavailable.
//! No US-layout fallback is supplied.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Default)]
struct Text {
    normal: Option<Vec<u16>>,
    unshifted: Option<Vec<u16>>,
}

#[derive(Default)]
struct Store(HashMap<u32, Text>);
impl Store {
    fn record(&mut self, scan: u32, normal: Option<Vec<u16>>, unshifted: Option<Vec<u16>>) {
        self.0.insert(scan, Text { normal, unshifted });
    }
    fn get(&self, scan: u32, ignore_shift: bool) -> Option<Vec<u16>> {
        self.0.get(&scan).and_then(|text| {
            if ignore_shift {
                text.unshifted.clone()
            } else {
                text.normal.clone()
            }
        })
    }
}
fn state() -> &'static Mutex<Store> {
    static STATE: OnceLock<Mutex<Store>> = OnceLock::new();
    STATE.get_or_init(Mutex::default)
}

#[cfg(not(windows))]
pub(crate) fn record(
    scan: u32,
    normal: Option<&str>,
    unshifted: Option<&str>,
    dead: bool,
    alt: bool,
) {
    let text = observed_text(normal, unshifted, dead, alt);
    record_wide(scan, text.normal, text.unshifted);
}

#[cfg(any(not(windows), test))]
fn observed_text(normal: Option<&str>, unshifted: Option<&str>, dead: bool, alt: bool) -> Text {
    let wide = |s: Option<&str>| {
        s.map(|s| s.encode_utf16().take(16).collect::<Vec<_>>())
            .filter(|s| !s.is_empty())
    };
    Text {
        normal: wide(if dead || alt { None } else { normal }),
        // winit supplies this from the layout with Shift/Caps/Ctrl removed.
        // A shifted dead key can therefore have an ordinary unshifted digit.
        unshifted: wide(unshifted),
    }
}

fn record_wide(scan: u32, normal: Option<Vec<u16>>, unshifted: Option<Vec<u16>>) {
    state().lock().unwrap().record(scan, normal, unshifted);
}

pub(crate) fn clear() {
    state().lock().unwrap().0.clear();
}

pub(crate) fn get(scan: u32, ignore_shift: bool) -> Option<Vec<u16>> {
    state().lock().unwrap().get(scan, ignore_shift)
}

/// Runs on the native Win32 window thread, where GetKeyboardState has the
/// correct message-queue state. Matches ShowOff SO_fn_Misc.cpp::ToANSIChar.
#[cfg(windows)]
pub(crate) fn observe_windows(scan: u32) {
    #[link(name = "user32")]
    extern "system" {
        fn GetKeyboardState(keys: *mut u8) -> i32;
        fn GetKeyboardLayout(thread: u32) -> isize;
        fn MapVirtualKeyExW(code: u32, kind: u32, layout: isize) -> u32;
        fn ToUnicodeEx(
            vk: u32,
            scan: u32,
            keys: *const u8,
            out: *mut u16,
            count: i32,
            flags: u32,
            layout: isize,
        ) -> i32;
    }
    let mut keys = [0u8; 256];
    // SAFETY: fixed-size writable keyboard buffer; the window thread owns the
    // native input queue. Handles are used only for this synchronous query.
    let (normal, unshifted) = unsafe {
        if GetKeyboardState(keys.as_mut_ptr()) == 0 {
            record_wide(scan, None, None);
            return;
        }
        let layout = GetKeyboardLayout(0);
        for vk in [0x11, 0xa2, 0xa3, 0x12, 0xa4, 0xa5] {
            keys[vk] = 0;
        }
        let vk = MapVirtualKeyExW(scan, 1, layout);
        let convert = |keys: &[u8; 256]| {
            if vk == 0 {
                return None;
            }
            let mut out = [0u16; 16];
            let n = ToUnicodeEx(vk, scan, keys.as_ptr(), out.as_mut_ptr(), 16, 4, layout);
            if n <= 0 {
                None
            } else {
                Some(out[..(n as usize).min(16)].to_vec())
            }
        };
        let normal = convert(&keys);
        for vk in [0x10, 0xa0, 0xa1, 0x14] {
            keys[vk] = 0;
        }
        (normal, convert(&keys))
    };
    record_wide(scan, normal, unshifted);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_normal_key_preserves_the_observed_modifier_free_fallback() {
        let mut keys = Store::default();
        let text = observed_text(Some("^"), Some("6"), true, false);
        keys.record(7, text.normal, text.unshifted);
        assert_eq!(
            keys.get(7, false),
            None,
            "dead normal text remains unavailable"
        );
        assert_eq!(keys.get(7, true), Some(vec![b'6' as u16]));
        let text = observed_text(None, None, true, false);
        keys.record(7, text.normal, text.unshifted);
        assert_eq!(
            keys.get(7, true),
            None,
            "missing base replaces the prior fallback"
        );
    }

    #[test]
    fn observed_layout_text_is_not_a_physical_us_key_guess() {
        let mut keys = Store::default();
        assert_eq!(keys.get(3, false), None);
        keys.record(
            3,
            Some("2".encode_utf16().collect()),
            Some("é".encode_utf16().collect()),
        );
        assert_eq!(keys.get(3, false), Some(vec![b'2' as u16]));
        assert_eq!(keys.get(3, true), Some(vec![0xe9]));
        assert_eq!(keys.get(4, false), None, "unobserved key is unavailable");
        keys.record(3, None, None);
        assert_eq!(
            keys.get(3, false),
            None,
            "dead/unsupported event replaces stale text"
        );
        assert_eq!(keys.get(3, true), None);
        keys.record(3, Some(vec![0x03a9]), Some(vec![0x03c9]));
        let mut copy = keys.get(3, false).unwrap();
        copy[0] = 0;
        assert_eq!(
            keys.get(3, false),
            Some(vec![0x03a9]),
            "query owns its snapshot"
        );
        keys.0.clear();
        assert_eq!(keys.get(3, false), None);
    }
}
