//! Share 3's unit tests (menus and the HUD). Only share 3 edits this file.

use engine_core::extend::stewie::Row;

/// Share 3's rows' plan numbers, off keys and bits (`rows::menus::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[];
