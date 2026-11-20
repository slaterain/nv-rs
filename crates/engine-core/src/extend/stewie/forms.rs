//! Share 1b's rows (forms, cells, scripts, saves and sound: forms, perks, weapons, script
//! functions, extra data, inventory changes, references, cells, models, saves, audio media and the
//! radio). Only share 1b adds rows here (see `super::world` for the rule). A row is
//! `Row::share(FORMS_FIXES, FORMS_FIXES_END, n)` (or the tweaks' pair), which does not compile past
//! the share's last bit.

/// Share 1b's fix rows: bits `214..300`.
pub const FORMS_FIXES: u16 = super::world::WORLD_FIXES_END;
/// One past share 1b's last fix bit.
pub const FORMS_FIXES_END: u16 = 300;
/// Share 1b's tweak rows: bits `704..768`.
pub const FORMS_TWEAKS: u16 = super::world::WORLD_TWEAKS_END;
/// One past share 1b's last tweak bit.
pub const FORMS_TWEAKS_END: u16 = 768;
