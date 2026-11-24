//! Text-only paste support on Windows, Linux (X11/Wayland), and macOS.
//! Windows retains CF_TEXT bytes, including its clipboard locale conversion.
//! Other hosts expose Unicode; the existing NLS layer converts to the game's
//! ACP before applying the byte limit. Clipboard errors remain unavailable.
use engine_core::Engine;
#[cfg(any(not(windows), test))]
use engine_core::Ptr;

#[cfg(not(windows))]
pub(crate) fn read(e: &mut Engine, limit: usize) -> Option<Vec<u8>> {
    let text = arboard::Clipboard::new().ok()?.get_text().ok()?;
    encode_prefix(e, &text, limit, 0)
}

#[cfg(any(not(windows), test))]
fn encode_prefix(e: &mut Engine, text: &str, limit: usize, code_page: u32) -> Option<Vec<u8>> {
    if limit == 0 {
        return Some(Vec::new());
    }
    // At most one ANSI character per Unicode scalar is needed to obtain the
    // first `limit` bytes; retain an extra scalar, never half a surrogate pair.
    let wide: Vec<u16> = text
        .split('\0')
        .next()
        .unwrap_or_default()
        .chars()
        .take(limit.saturating_add(1))
        .flat_map(|c| {
            let mut units = [0; 2];
            c.encode_utf16(&mut units).to_vec()
        })
        .collect();
    if wide.is_empty() {
        return Some(Vec::new());
    }
    let capacity = wide.len().checked_mul(4)?.checked_add(1)?;
    let frame = wide.len().checked_mul(2)?.checked_add(capacity)?;
    e.with_stack(u32::try_from(frame).ok()?, |e, base| {
        for (i, unit) in wide.iter().enumerate() {
            e.mem.set_u16(base.addr() + i as u32 * 2, *unit);
        }
        let out = base.addr() + wide.len() as u32 * 2;
        let n = crate::nls::wide_char_to_multi_byte(
            e,
            code_page,
            0,
            base,
            wide.len() as i32,
            Ptr::new(out),
            capacity as i32,
            Ptr::NULL,
            Ptr::NULL,
        );
        if n == 0 {
            return None;
        }
        Some(e.mem.bytes(out, (n as usize).min(limit) as u32).to_vec())
    })
}

#[cfg(windows)]
pub(crate) fn read(_: &mut Engine, limit: usize) -> Option<Vec<u8>> {
    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(window: isize) -> i32;
        fn GetClipboardData(format: u32) -> isize;
        fn CloseClipboard() -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalLock(handle: isize) -> *const u8;
        fn GlobalUnlock(handle: isize) -> i32;
        fn GlobalSize(handle: isize) -> usize;
    }
    struct Open;
    impl Drop for Open {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    struct Locked(isize);
    impl Drop for Locked {
        fn drop(&mut self) {
            unsafe {
                GlobalUnlock(self.0);
            }
        }
    }
    // SAFETY: handles are obtained from the system, a locked block is bounded
    // by GlobalSize, and both resources are released on every return path.
    unsafe {
        if OpenClipboard(0) == 0 {
            return None;
        }
        let _open = Open;
        let handle = GetClipboardData(1); // CF_TEXT, as Tweaks' original helper.
        if handle == 0 {
            return None;
        }
        let data = GlobalLock(handle);
        if data.is_null() {
            return None;
        }
        let _locked = Locked(handle);
        let n = limit.min(GlobalSize(handle));
        let bytes = std::slice::from_raw_parts(data, n);
        Some(bytes.split(|&b| b == 0).next().unwrap_or_default().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_clipboard_uses_real_ansi_conversion_before_byte_limit_and_stops_at_nul() {
        let mut e = Engine::new();
        assert_eq!(
            encode_prefix(&mut e, "café\0ignored", 200, 1252),
            Some(b"caf\xe9".to_vec())
        );
        assert_eq!(
            encode_prefix(&mut e, "café", 3, 1252),
            Some(b"caf".to_vec())
        );
        assert_eq!(encode_prefix(&mut e, "", 200, 1252), Some(Vec::new()));
        // UTF-8 demonstrates a BYTE limit, including an intentionally split
        // final character, exactly as a native CF_TEXT 200-byte paste limit.
        #[cfg(windows)]
        assert_eq!(
            encode_prefix(&mut e, "éé", 3, 65001),
            Some(vec![0xc3, 0xa9, 0xc3])
        );
        assert_eq!(encode_prefix(&mut e, "ignored", 0, 1252), Some(Vec::new()));
    }
}
