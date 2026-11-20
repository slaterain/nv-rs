//! Share 2's rows (AI, actors and the player: actors, processes, combat, packages, projectiles,
//! the player character, animation and the model loader). Only share 2 adds rows here (see
//! `super::world` for the rule). A row is `Row::share(AI_FIXES, AI_FIXES_END, n)` (or the
//! tweaks' pair), which does not compile past the share's last bit.

/// Share 2's fix rows: bits `300..470`.
pub const AI_FIXES: u16 = super::forms::FORMS_FIXES_END;
/// One past share 2's last fix bit.
pub const AI_FIXES_END: u16 = 470;
/// Share 2's tweak rows: bits `768..896`.
pub const AI_TWEAKS: u16 = super::forms::FORMS_TWEAKS_END;
/// One past share 2's last tweak bit.
pub const AI_TWEAKS_END: u16 = 896;
