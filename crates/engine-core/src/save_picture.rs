//! Port-added (not in the original): a save's picture read back after the save's body,
//! `NV_SAVE_PICTURE_LATER=1` (off until it is checked in play).
//!
//! The game's save (`00856ca0`) writes its header first (`TESSaveLoadGame::SaveHeader`, `00861130`),
//! whose picture (`ScreenShot::TakeSaveGameScreenShot`, `00878f60`) draws the scene into a render target
//! and copies it into a system-memory surface (`GetRenderTargetData`): main waits there until the render
//! thread has carried out everything before the copy, the picture's scene among it, and only then writes
//! the save's body. With the switch the copy's reply is taken at the end of the body instead: the header
//! is written with the picture's bytes as they are, the place of those bytes in the file is kept, and at
//! the end of the body (before the game seeks back to write its two positions) the copied rows are turned
//! into the picture's bytes as `00878f60` turns them and written over them. The file's bytes are the same
//! as without the switch; main does not wait for the render thread while it writes the body.
//!
//! The steps, all on the thread that saves (the game's main thread), each one a no-op unless the one
//! before it was taken: [`begin`] (SaveHeader, before the picture; nothing unless the switch is on),
//! [`allow`] (the picture, before its copy, when the rows it converts are all copied rows), [`deferred`]
//! (the platform's copy, when it asked for its reply later), [`shot`] (the picture, its size: it converts
//! nothing), [`end`] (SaveHeader, after the picture), [`at`] (SaveHeader, the file position of the
//! picture's bytes and those bytes), [`take`] (the save, at the end of its body: the rows from the
//! platform's [`install`]ed function, made into the bytes to write there).

use crate::Engine;
use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

/// The picture's place in the save, from [`at`] to [`take`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    /// The file position of the picture's bytes (`0x004720a0` on the save's file, just before they are
    /// written).
    pub offset: u32,
    /// The bytes to write there: the picture's 24-bit pixels, as `00878f60` converts them.
    pub bytes: Vec<u8>,
}

#[derive(Default)]
enum Step {
    #[default]
    Off,
    /// [`begin`]: a save's header is being written with the switch on.
    Begun,
    /// [`allow`]: the picture may take its copy's reply later.
    Allowed,
    /// [`deferred`]: the copy's reply is on its way; the destination's rows are `pitch` bytes apart.
    Deferred { pitch: u32 },
    /// [`shot`]: the picture of `width` by `height` pixels converted nothing.
    Shot { width: u32, height: u32, pitch: u32 },
    /// [`at`]: its bytes are in the file at `offset` (`bytes`: what was written there).
    Placed {
        width: u32,
        height: u32,
        pitch: u32,
        offset: u32,
        bytes: Vec<u8>,
    },
}

thread_local! {
    static STEP: RefCell<Step> = const { RefCell::new(Step::Off) };
    /// Tests: the switch for this thread (0: the run's, 1: off, 2: on).
    static ON_HERE: Cell<u8> = const { Cell::new(0) };
}

/// The rows of the copy asked for later: the row length and the rows (`None`: no reply, as the copy's
/// own `false`).
pub type TakeRows = fn(&mut Engine) -> Option<(u32, Vec<u8>)>;

static TAKE: OnceLock<TakeRows> = OnceLock::new();

/// The platform's function that takes the copy's rows (the first call wins).
pub fn install(take: TakeRows) {
    let _ = TAKE.set(take);
}

/// `NV_SAVE_PICTURE_LATER=1`: a save's picture is read back after the save's body (see the module).
pub fn on() -> bool {
    static RUN: OnceLock<bool> = OnceLock::new();
    match ON_HERE.with(Cell::get) {
        1 => false,
        2 => true,
        _ => *RUN
            .get_or_init(|| std::env::var("NV_SAVE_PICTURE_LATER").is_ok_and(|v| v.trim() == "1")),
    }
}

/// Tests: the switch on this thread, whatever the run says (`None`: the run's again).
#[doc(hidden)]
pub fn on_for_tests(on: Option<bool>) {
    ON_HERE.with(|c| c.set(on.map_or(0, |on| 1 + u8::from(on))));
}

fn step<R>(f: impl FnOnce(&mut Step) -> R) -> R {
    STEP.with(|s| f(&mut s.borrow_mut()))
}

/// SaveHeader, before its picture: with the switch on, the picture may be read back later.
pub fn begin() {
    step(|s| *s = if on() { Step::Begun } else { Step::Off });
}

/// The picture, before its copy: a save's picture may take its copy's reply later when `rows_suffice`
/// (every pixel it converts is in the copied rows).
pub fn allow(rows_suffice: bool) {
    step(|s| {
        if matches!(s, Step::Begun) {
            *s = if rows_suffice {
                Step::Allowed
            } else {
                Step::Off
            };
        }
    });
}

/// The platform's copy: whether it may ask for its reply later.
pub fn allowed() -> bool {
    step(|s| matches!(s, Step::Allowed))
}

/// The platform's copy asked for its reply later (the destination's memory is not written; its rows are
/// `pitch` bytes apart).
pub fn deferred(pitch: u32) {
    step(|s| {
        if matches!(s, Step::Allowed) {
            *s = Step::Deferred { pitch };
        }
    });
}

/// The picture, after its copy: whether the copy's reply comes later (its pixels are then not converted).
pub fn is_deferred() -> bool {
    step(|s| matches!(s, Step::Deferred { .. }))
}

/// The picture of `width` by `height` pixels converted nothing: its bytes are made at [`take`].
pub fn shot(width: u32, height: u32) {
    step(|s| {
        if let Step::Deferred { pitch } = *s {
            *s = Step::Shot {
                width,
                height,
                pitch,
            };
        }
    });
}

/// SaveHeader, after its picture: whether its bytes are to be made later ([`at`] then keeps their place).
pub fn end() -> bool {
    step(|s| {
        let shot = matches!(s, Step::Shot { .. });
        if !shot {
            *s = Step::Off;
        }
        shot
    })
}

/// SaveHeader: the picture's bytes, `bytes`, are written at the file position `offset`.
pub fn at(offset: u32, bytes: Vec<u8>) {
    step(|s| {
        if let Step::Shot {
            width,
            height,
            pitch,
        } = *s
        {
            *s = Step::Placed {
                width,
                height,
                pitch,
                offset,
                bytes,
            };
        }
    });
}

/// The save, at the end of its body: the picture's place and its bytes, made from the copy's rows
/// (`None` when no picture waits). A copy that gave no rows (a backend without surfaces) leaves the
/// destination's memory as it was made, zero: a black picture, and the save goes on (the game does
/// not look at `GetRenderTargetData`'s result either; review of #1037).
pub fn take(e: &mut Engine) -> Option<Picture> {
    let Step::Placed {
        width,
        height,
        pitch,
        offset,
        bytes,
    } = step(std::mem::take)
    else {
        return None;
    };
    let rows = TAKE.get().and_then(|take| take(e));
    Some(picture(width, height, pitch, offset, bytes, rows))
}

/// The picture of `width` by `height` pixels whose bytes `bytes` are at `offset`, made from the copy's
/// rows (`row` bytes each, `pitch` apart in the destination; none: zero). As many of the
/// destination's rows as the conversion reads (`width * height` pixels, the rows packed) are placed.
pub fn picture(
    width: u32,
    height: u32,
    pitch: u32,
    offset: u32,
    mut bytes: Vec<u8>,
    rows: Option<(u32, Vec<u8>)>,
) -> Picture {
    let (row, rows) = rows.unwrap_or((0, Vec::new()));
    let read = u64::from(width) * u64::from(height) * 4;
    let surface_rows = read
        .div_ceil(u64::from(pitch.max(1)))
        .min(u64::from(u32::MAX)) as u32;
    convert(
        &placed(&rows, row, pitch, surface_rows),
        width,
        height,
        &mut bytes,
    );
    Picture { offset, bytes }
}

/// The destination's first `height` rows `pitch` bytes apart, as the copy writes them into its memory
/// (each row of `row` bytes at its place; a part no row reaches is zero, as the new surface's memory).
pub fn placed(rows: &[u8], row: u32, pitch: u32, height: u32) -> Vec<u8> {
    let mut surface = vec![0u8; pitch as usize * height as usize];
    if row != 0 {
        for (y, r) in rows.chunks(row as usize).enumerate() {
            let at = y * pitch as usize;
            if at >= surface.len() {
                break;
            }
            let n = r.len().min(surface.len() - at);
            surface[at..at + n].copy_from_slice(&r[..n]);
        }
    }
    surface
}

/// The picture's conversion of `width * height` pixels of `surface` (blue, green, red, alpha, the rows
/// packed) into `into` as red, green, blue, as `00878f60` converts them (the count a signed 32-bit
/// product); bytes past `into`'s end are not written (the caller's picture holds exactly them).
pub fn convert(surface: &[u8], width: u32, height: u32, into: &mut [u8]) {
    let limit = (width as i32).wrapping_mul(height as i32).wrapping_mul(4);
    if limit <= 0 {
        return;
    }
    for (pixel, out) in surface[..(limit as usize).min(surface.len())]
        .chunks_exact(4)
        .zip(into.chunks_mut(3))
    {
        let rgb = [pixel[2], pixel[1], pixel[0]];
        out.copy_from_slice(&rgb[..out.len()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_go_in_order_and_only_with_the_switch_on() {
        on_for_tests(Some(false));
        begin();
        allow(true);
        assert!(!allowed(), "off: nothing allowed");
        on_for_tests(Some(true));
        begin();
        allow(false);
        assert!(!allowed(), "rows not enough: not allowed");
        begin();
        allow(true);
        assert!(allowed());
        deferred(8);
        assert!(is_deferred() && !allowed());
        shot(2, 1);
        assert!(end());
        at(40, vec![9; 6]);
        install(|_| Some((8, vec![1, 2, 3, 4, 5, 6, 7, 8])));
        let mut e = Engine::new();
        let p = take(&mut e).expect("the picture");
        assert_eq!(p.offset, 40);
        assert_eq!(p.bytes, vec![3, 2, 1, 7, 6, 5]);
        assert!(take(&mut e).is_none(), "taken once");
        // Not deferred (the platform could not): the header writes the converted picture as before.
        begin();
        allow(true);
        shot(2, 1);
        assert!(!end());
        at(40, vec![]);
        assert!(take(&mut e).is_none());
        on_for_tests(None);
    }

    #[test]
    fn a_copy_without_rows_is_a_black_picture_and_every_row_read_is_placed() {
        // No rows (a backend without surfaces): black, the save goes on.
        let p = picture(2, 1, 8, 40, vec![9; 6], None);
        assert_eq!((p.offset, p.bytes), (40, vec![0; 6]));
        // A picture 4 pixels wide of a destination 2 pixels wide (8 bytes a row): its one row of pixels
        // is the destination's first two rows, as the picture's conversion reads them.
        let rows: Vec<u8> = (1..=16).collect();
        let p = picture(4, 1, 8, 0, vec![9; 12], Some((8, rows)));
        assert_eq!(p.bytes, vec![3, 2, 1, 7, 6, 5, 11, 10, 9, 15, 14, 13]);
    }

    #[test]
    fn rows_are_placed_pitch_apart_and_the_count_is_signed() {
        assert_eq!(placed(&[1, 2, 3, 4], 2, 3, 2), vec![1, 2, 0, 3, 4, 0]);
        assert_eq!(placed(&[1, 2, 3, 4], 2, 2, 1), vec![1, 2]);
        let mut into = vec![7; 6];
        convert(&[1, 2, 3, 4, 5, 6, 7, 8], u32::MAX, 2, &mut into);
        assert_eq!(into, vec![7; 6], "a negative count converts nothing");
        convert(&[1, 2, 3, 4, 5, 6, 7, 8], 2, 1, &mut into[..4]);
        assert_eq!(
            into,
            vec![3, 2, 1, 7, 7, 7],
            "the second pixel cut at the end"
        );
    }
}
