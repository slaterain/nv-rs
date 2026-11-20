//! Share 3's rows (menus and the HUD: every menu but the interface manager's and the console's,
//! tiles, fonts and the controller's input). Only share 3 adds rows here (see `super::world` for
//! the rule). A row is `Row::share(MENUS_FIXES, MENUS_FIXES_END, n)` (or the tweaks' pair), which
//! does not compile past the share's last bit.

/// Share 3's fix rows: bits `470..640`.
pub const MENUS_FIXES: u16 = super::ai::AI_FIXES_END;
/// One past share 3's last fix bit.
pub const MENUS_FIXES_END: u16 = super::VNV_TWEAKS;
/// Share 3's tweak rows: bits `896..1024`.
pub const MENUS_TWEAKS: u16 = super::ai::AI_TWEAKS_END;
/// One past share 3's last tweak bit.
pub const MENUS_TWEAKS_END: u16 = super::OTHER_TWEAKS;
