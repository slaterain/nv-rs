//! Share 1b's unit tests (forms, cells, scripts, saves and sound). Only share 1b edits this file.

use engine_core::extend::stewie::Row;

/// Share 1b's rows' plan numbers, off keys and bits (`rows::forms::ROWS`), in the same order:
/// `super::each_rows_plan_number_key_and_bit_are_pinned` checks them with every other share's.
pub(super) const PINNED: &[(&str, &str, Row)] = &[];
