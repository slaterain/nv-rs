//! Share 2's unit tests (AI, actors and the player). Only share 2 edits this file.

use engine_core::extend::stewie::Row;

/// Share 2's rows' plan numbers, off keys and bits (`rows::ai::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[];
