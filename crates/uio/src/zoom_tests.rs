//! UIO's text zoom, its HUD zoom removal and its FinalPostParse range, against its source.

use super::*;
use crate::post_parse::final_post_parse;
use std::cell::RefCell;

#[test]
fn the_scale_is_a_hundredth_of_a_positive_zoom_other_than_100() {
    let s = |x: f32| scale_of(x.to_bits());
    assert_eq!(s(200.0), Some(2.0));
    assert_eq!(s(50.0), Some(0.5));
    assert_eq!(s(150.0), Some(150.0f32 * 0.01f32));
    assert_eq!(s(100.0), None);
    assert_eq!(s(0.0), None);
    assert_eq!(s(-0.0), None);
    assert_eq!(s(-50.0), None);
    assert_eq!(s(f32::NEG_INFINITY), None);
    assert_eq!(s(f32::INFINITY), Some(f32::INFINITY));
    // Its bits above 0 as a signed word: a positive NaN scales (by NaN), a negative one not.
    assert!(scale_of(0x7fc0_0000).unwrap().is_nan());
    assert_eq!(scale_of(0xffc0_0000), None);
    // The smallest positive number: a hundredth of it.
    assert_eq!(scale_of(1), Some(f32::from_bits(1) * 0.01));
}

/// A tile whose sorted trait array holds values with `ids`, each with float `id as f32`.
fn tile_with(e: &mut Engine, ids: &[u32]) -> u32 {
    let tile = e.mem.alloc(0x38);
    let buffer = e.mem.alloc(4 * ids.len() as u32 + 4);
    for (i, &id) in ids.iter().enumerate() {
        let v = e.mem.alloc(0x14);
        e.mem.set_u32(v, id);
        e.mem.set_f32(v + 8, id as f32);
        e.mem.set_u32(buffer + 4 * i as u32, v);
    }
    e.mem.set_u32(tile + 0x14, buffer);
    e.mem.set_u32(tile + 0x18, ids.len() as u32);
    tile
}

#[test]
fn the_value_is_found_by_uios_binary_search() {
    let mut e = Engine::new();
    let ids = [0xfa1, 0xfa2, 0xfb7, 0xfb8, 0xfba, 0xff7, 0x2710];
    let tile = tile_with(&mut e, &ids);
    for id in ids {
        let v = tile_value(&e, tile, id);
        assert_ne!(v, 0, "{id:x}");
        assert_eq!(e.mem.u32(v), id);
    }
    for id in [0, 0xfa0, 0xfb9, 0x3000, u32::MAX] {
        assert_eq!(tile_value(&e, tile, id), 0, "{id:x}");
    }
    let empty = tile_with(&mut e, &[]);
    assert_eq!(tile_value(&e, empty, ZOOM), 0);
}

thread_local! {
    /// `Font::CreateText`'s calls: its words and the width and limit it was given.
    static CREATED: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
}

/// An engine whose `Font::CreateText` records its call, makes a shape and lays the text out at
/// `laid` (width, limit) when given.
fn engine(loaded: bool, laid: Option<(i32, i32)>) -> Engine {
    CREATED.with(|c| c.borrow_mut().clear());
    let mut e = Engine::new();
    e.ext::<Uio>().loaded = loaded;
    e.register_double(FONT_CREATE_TEXT, move |e, a| {
        let (w, l) = (e.mem.i32(a[2]), e.mem.i32(a[3]));
        CREATED.with(|c| c.borrow_mut().push((a.to_vec(), w, l)));
        let shape = e.mem.alloc(0x70);
        e.mem.set_f32(shape + 0x64, 1.0);
        e.mem.set_u32(a[9], shape);
        if let Some((w, l)) = laid {
            e.mem.set_i32(a[2], w);
            e.mem.set_i32(a[3], l);
        }
        Ret::default()
    });
    e
}

/// One `Font::CreateText` call: its words, the width and the limit it was given.
type Call = (Vec<u32>, i32, i32);

/// MakeNode's words at the seam for a tile with `zoom` (none for `None`), wrap width `width`
/// and limit `limit`: (the seam's answer, the width and limit after, the shape's scale, the call).
fn scale(
    loaded: bool,
    zoom: Option<f32>,
    width: i32,
    limit: i32,
    laid: Option<(i32, i32)>,
) -> (Option<u32>, i32, i32, f32, Option<Call>) {
    let mut e = engine(loaded, laid);
    let tile = match zoom {
        Some(z) => {
            let t = tile_with(&mut e, &[0xfa1, ZOOM, 0xfba]);
            let v = tile_value(&e, t, ZOOM);
            e.mem.set_f32(v + 8, z);
            t
        }
        None => tile_with(&mut e, &[0xfa1, 0xfba]),
    };
    let frame = e.mem.alloc(0x30);
    let (width_at, limit_at, shape_at, icons_at) = (frame, frame + 4, frame + 8, frame + 12);
    e.mem.set_i32(width_at, width);
    e.mem.set_i32(limit_at, limit);
    let words = [
        tile,
        0x0f00,
        0x0e00,
        width_at,
        limit_at,
        0x7fff_ffff,
        2,
        0x0d00,
        shape_at,
        icons_at,
    ];
    let answer = text_scale(&mut e, &words);
    let shape = e.mem.u32(shape_at);
    let shape_scale = if shape == 0 {
        0.0
    } else {
        e.mem.f32(shape + 0x64)
    };
    let call = CREATED.with(|c| c.borrow().first().cloned());
    (
        answer,
        e.mem.i32(width_at),
        e.mem.i32(limit_at),
        shape_scale,
        call,
    )
}

#[test]
fn before_uioload_the_seam_answers_nothing_and_makes_no_call() {
    let (answer, w, l, _, call) = scale(false, Some(200.0), 300, 50, None);
    assert_eq!((answer, w, l), (None, 300, 50));
    assert!(call.is_none());
}

#[test]
fn the_call_is_the_games_with_its_own_words() {
    let (answer, w, l, s, call) = scale(true, None, 300, 50, Some((280, 40)));
    assert_eq!(answer, Some(1));
    let (words, given_w, given_l) = call.unwrap();
    let frame = words[2];
    assert_eq!(
        words,
        vec![
            0x0f00,
            0x0e00,
            frame,
            frame + 4,
            0,
            0x7fff_ffff,
            2,
            0xa,
            0x0d00,
            frame + 8,
            frame + 12
        ]
    );
    // No zoom: the width and limit as given and as laid out, the shape not scaled.
    assert_eq!((given_w, given_l, w, l, s), (300, 50, 280, 40, 1.0));
    // A zoom of 100: the same.
    let (_, w, l, s, call) = scale(true, Some(100.0), 300, 50, Some((280, 40)));
    assert_eq!((call.unwrap().1, w, l, s), (300, 280, 40, 1.0));
}

#[test]
fn a_zoom_lays_the_text_out_smaller_and_scales_the_shape() {
    // 200: laid out at 150 x 25, the shape doubled, the laid-out size doubled back.
    let (_, w, l, s, call) = scale(true, Some(200.0), 300, 50, Some((140, 33)));
    let (_, given_w, given_l) = call.unwrap();
    assert_eq!((given_w, given_l), (150, 25));
    assert_eq!((w, l, s), (280, 66, 2.0));
    // 300: 100 / 3 = 33.3 -> 33, 10 / 3 -> 3; back: 33 * 3 = 99, 3 * 3 = 9 (rounded to nearest).
    let (_, w, l, _, call) = scale(true, Some(300.0), 100, 10, None);
    let c = call.unwrap();
    assert_eq!((c.1, c.2, w, l), (33, 3, 99, 9));
    // Ties go to even: 5 / 2 = 2.5 -> 2.
    let (_, _, _, _, call) = scale(true, Some(200.0), 5, 7, None);
    let c = call.unwrap();
    assert_eq!((c.1, c.2), (2, 4));
    // 50: the text laid out twice as wide, the shape halved.
    let (_, w, l, s, call) = scale(true, Some(50.0), 300, 0, Some((500, 21)));
    let c = call.unwrap();
    assert_eq!((c.1, c.2, w, l, s), (600, 0, 250, 10, 0.5));
}

#[test]
fn a_wide_text_is_not_narrowed_but_still_scaled_back() {
    // A width of 0x800 or more (unsigned: negatives too) is passed as it is.
    let (_, w, l, s, call) = scale(true, Some(150.0), 0x800, 40, Some((1000, 30)));
    let c = call.unwrap();
    assert_eq!((c.1, c.2), (0x800, 40));
    assert_eq!((w, l, s), (1500, 45, 1.5));
    let (_, _, _, _, call) = scale(true, Some(200.0), -1, 40, Some((10, 10)));
    let c = call.unwrap();
    assert_eq!((c.1, c.2), (-1, 40));
    // 0x7FF is narrowed.
    let (_, _, _, _, call) = scale(true, Some(200.0), 0x7ff, 40, None);
    assert_eq!(call.unwrap().1, 0x400);
}

#[test]
fn odd_zooms_negative_zero_and_nan() {
    // Negative and zero zooms: no scale.
    for z in [0.0, -0.0, -200.0] {
        let (_, w, l, s, _) = scale(true, Some(z), 300, 50, Some((280, 40)));
        assert_eq!((w, l, s), (280, 40, 1.0), "{z}");
    }
    // A positive NaN: the width and limit become the indefinite integer, the shape's scale NaN.
    let (_, w, l, s, call) = scale(true, Some(f32::NAN), 300, 50, None);
    let c = call.unwrap();
    assert_eq!((c.1, c.2, w, l), (i32::MIN, i32::MIN, i32::MIN, i32::MIN));
    assert!(s.is_nan());
    // A huge zoom: laid out at 0, multiplied back to the indefinite integer (inf * 0 is NaN).
    let (_, w, _, _, call) = scale(true, Some(f32::INFINITY), 300, 50, None);
    assert_eq!(call.unwrap().1, 0);
    assert_eq!(w, i32::MIN);
}

#[test]
fn the_hud_zooms_are_skipped_once_uioload_ran() {
    let mut e = Engine::new();
    assert_eq!(hud_zoom(&mut e, &[0x164]), None);
    e.ext::<Uio>().loaded = true;
    for field in [0x164, 0x16c, 0x168, 0x178, 0x17c] {
        assert_eq!(hud_zoom(&mut e, &[field]), Some(1));
    }
}

#[test]
fn final_post_parse_takes_uios_range_once_uioload_ran() {
    let mut e = Engine::new();
    assert_eq!(final_post_parse(&mut e, &[0xfb8]), None);
    e.ext::<Uio>().loaded = true;
    for t in 0xfb7..=0xfbe {
        assert_eq!(final_post_parse(&mut e, &[t]), Some(1), "{t:x}");
    }
    assert_eq!(final_post_parse(&mut e, &[0xff7]), Some(1));
    for t in [0, 0xfa1, 0xfb6, 0xfbf, 0xff6, 0xff8, 0x2710, u32::MAX] {
        assert_eq!(final_post_parse(&mut e, &[t]), None, "{t:x}");
    }
}
