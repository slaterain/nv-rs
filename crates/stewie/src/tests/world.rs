//! Share 1a's unit tests (the engines and the console). Only share 1a edits this file.

use engine_core::extend::stewie::Row;

/// Share 1a's rows' plan numbers, off keys and bits (`rows::world::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[];
