//! Share 1a's rows (the engines and the console: Havok, Gamebryo, the environment, magic, pathing,
//! face-gen, the engine-bs files no other share holds, the interface manager, the console and the
//! INI settings). Only share 1a adds rows here; the other shares add theirs in `super::forms`,
//! `super::ai` and `super::menus`, so their PRs never touch the same lines. A row is
//! `Row::share(WORLD_FIXES, WORLD_FIXES_END, n)` (or the tweaks' pair), which does not compile past
//! the share's last bit.

/// Share 1a's fix rows: bits `128..214` (row S1016's `CHANGES_MAP_BUCKETS`, bit 128, is its first).
pub const WORLD_FIXES: u16 = super::FIXES;
/// One past share 1a's last fix bit.
pub const WORLD_FIXES_END: u16 = 214;
/// Share 1a's tweak rows: bits `640..704`.
pub const WORLD_TWEAKS: u16 = super::VNV_TWEAKS;
/// One past share 1a's last tweak bit.
pub const WORLD_TWEAKS_END: u16 = 704;
