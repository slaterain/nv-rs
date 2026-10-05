//! Ownership and crime, read from the game's code
//! (`%USERPROFILE%\nv-re\findings\reputation.md` §3).
//!
//! Who owns a reference (`00567790`): its own `XOWN`; for a door, the
//! door on the other side's; for anything but people, furniture, doors and
//! activators, its encounter zone's owner (`ECZN` `DATA`, the reference's
//! `XEZN` else its cell's); else its cell's `XOWN` (else the cell's zone's).
//! People have only their own. The player may take what nobody owns, what
//! the player owns, and what a faction they're in owns (`005785e0`; the
//! rank `XRNK` isn't checked there).
//!
//! Stealing (`008bfa40`): −5 karma (`fKarmaModStealing`) whenever the
//! owner isn't evil (a faction flagged evil, `DATA` 0x02, or a person
//! whose record karma is evil), seen or not. If the victim (the person
//! stolen from, else the owner or a member of the owning faction nearby)
//! sees the player (detection above 0): each of the victim's factions that
//! tracks crime (`DATA` 0x100) counts a minor crime and gives the player
//! `fReputationMinorCrimeNeg` (2) infamy with its reputation (`WMI1`); a
//! second witnessed theft on the same day of the month makes the victim
//! attack; the first `iStealWarnings` (2) times the victim only comes to
//! warn (not here: the warning package and its line).
//!
//! A faction can hold the player as an enemy for their crimes (its runtime
//! flag 0x10; `SetPCEnemyofFaction`, `ClearFactionPlayerEnemyFlag`): its
//! members then react to the player as enemies (`world::factions`).
//! Trespassing (`00546da0`): in a cell with an owner, not public (`DATA`
//! 0x20 or 0x40), with no `XGLB`, that isn't the player's or a faction
//! they're in (at the cell's `XRNK` rank or above). Scripts change owners
//! and the public flag (`SetOwnership`, `SetCellOwnership`,
//! `SetCellPublicFlag`), and who notices (`IgnoreCrime`,
//! `SetIgnoreFriendlyHits`): `world::script_functions`.

use esm::{FormId, FourCC, LoadOrder, RecordRef};

use crate::cell::{le_f32, le_u32};
use crate::dialogue::PLAYER_REF;
use crate::scripting::{base_of, game_setting, GameState};

const XOWN: FourCC = FourCC::new(b"XOWN");
const XRNK: FourCC = FourCC::new(b"XRNK");
const XGLB: FourCC = FourCC::new(b"XGLB");
const XEZN: FourCC = FourCC::new(b"XEZN");
const XTEL: FourCC = FourCC::new(b"XTEL");
const FACT: FourCC = FourCC::new(b"FACT");

fn form_in(rr: &RecordRef<'_>, record: &esm::Record, kind: FourCC) -> Option<FormId> {
    record
        .get(kind)
        .filter(|s| s.data.len() >= 4)
        .map(|s| rr.plugin.to_global(FormId(le_u32(&s.data, 0))))
        .filter(|f| f.0 != 0)
}

/// An encounter zone's owner (`ECZN` `DATA` form at 0).
fn zone_owner(order: &LoadOrder, zone: FormId) -> Option<FormId> {
    let rr = order.get(zone)?;
    let record = rr.record().ok()?;
    form_in(&rr, &record, esm::sig::DATA)
}

/// A cell's owner: its `XOWN`, else its encounter zone's owner.
pub fn cell_owner(order: &LoadOrder, cell: FormId) -> Option<FormId> {
    let rr = order.get(cell)?;
    let record = rr.record().ok()?;
    form_in(&rr, &record, XOWN)
        .or_else(|| form_in(&rr, &record, XEZN).and_then(|z| zone_owner(order, z)))
}

/// Who owns a reference (see the module notes). Its own owner is as
/// `SetOwnership` set it, else its `XOWN`; a cell's as `SetCellOwnership`
/// set it, else its record's (`world::script_functions`).
pub fn owner_of(order: &LoadOrder, state: &GameState, reference: FormId) -> Option<FormId> {
    if let Some(o) = crate::script_functions::own_owner(order, state, reference) {
        return Some(o);
    }
    let rr = order.get(reference)?;
    let record = rr.record().ok()?;
    let kind = rr.entry.header.kind;
    if matches!(kind.as_bytes(), b"ACHR" | b"ACRE") {
        return None;
    }
    let base_kind = base_of(order, reference)
        .and_then(|b| order.get(b))
        .map(|b| b.entry.header.kind);
    let base = base_kind.as_ref().map(|k| *k.as_bytes());
    if base == Some(*b"DOOR") {
        let partner = form_in(&rr, &record, XTEL).and_then(|d| {
            let prr = order.get(d)?;
            let prec = prr.record().ok()?;
            form_in(&prr, &prec, XOWN)
        });
        if partner.is_some() {
            return partner;
        }
    }
    if !matches!(base, Some(b) if b == *b"FURN" || b == *b"DOOR" || b == *b"ACTI") {
        if let Some(o) = form_in(&rr, &record, XEZN).and_then(|z| zone_owner(order, z)) {
            return Some(o);
        }
    }
    let (_, cell, _, _) = state.place(order, reference)?;
    crate::script_functions::cell_owner_now(order, state, cell)
}

fn is_faction(order: &LoadOrder, id: FormId) -> bool {
    order.get(id).is_some_and(|r| r.entry.header.kind == FACT)
}

fn faction_flags(order: &LoadOrder, faction: FormId) -> u32 {
    order
        .get(faction)
        .and_then(|r| r.record().ok())
        .and_then(|r| r.get(esm::sig::DATA).map(|s| s.data.clone()))
        .map_or(0, |d| {
            let mut b = [0u8; 4];
            for (i, v) in d.iter().take(4).enumerate() {
                b[i] = *v;
            }
            u32::from_le_bytes(b)
        })
}

/// Whether a faction tracks crime (`FACT` `DATA` flag 0x100).
pub fn tracks_crime(order: &LoadOrder, faction: FormId) -> bool {
    faction_flags(order, faction) & 0x100 != 0
}

/// Whether the player may take or use what `owner` owns.
pub fn may_take(order: &LoadOrder, state: &GameState, owner: Option<FormId>) -> bool {
    let Some(owner) = owner else {
        return true;
    };
    if Some(owner) == base_of(order, PLAYER_REF) || owner == crate::dialogue::PLAYER_BASE {
        return true;
    }
    is_faction(order, owner)
        && crate::factions::factions_of(order, state, PLAYER_REF).contains(&owner)
}

/// Whether an owner is evil (`00578790`): a faction flagged evil, or a
/// person whose record karma is evil or very evil.
pub fn owner_is_evil(order: &LoadOrder, owner: FormId) -> bool {
    if is_faction(order, owner) {
        return faction_flags(order, owner) & 0x02 != 0;
    }
    let karma = order
        .get(owner)
        .and_then(|r| r.record().ok())
        .and_then(|r| r.get(FourCC::new(b"ACBS")).map(|s| s.data.clone()))
        .filter(|d| d.len() >= 20)
        .map_or(0.0, |d| le_f32(&d, 16));
    matches!(crate::reputation::alignment(order, karma), 2 | 4)
}

/// The person who would see a theft from `from` owned by `owner`: the
/// person stolen from, else someone in the player's cell who is the owner
/// or in the owning faction (the game asks for a loaded one).
fn victim(order: &LoadOrder, state: &GameState, from: FormId, owner: FormId) -> Option<FormId> {
    if order
        .get(from)
        .is_some_and(|r| r.entry.header.kind.as_bytes() == b"ACHR")
    {
        return Some(from);
    }
    let cell = state.player_cell?;
    order
        .references_in_cell(cell)
        .into_iter()
        .filter(|rr| rr.entry.header.kind.as_bytes() == b"ACHR")
        .map(|rr| rr.form_id)
        .find(|&who| {
            !state.dead.contains(&who)
                && (base_of(order, who) == Some(owner)
                    || crate::factions::factions_of(order, state, who).contains(&owner))
        })
}

/// The player takes something `owner` owns from `from` (see the module
/// notes). Whether anyone saw it.
pub fn steal(order: &LoadOrder, state: &mut GameState, from: FormId, owner: FormId) -> bool {
    if !owner_is_evil(order, owner) {
        let karma = game_setting(order, "fKarmaModStealing")
            .unwrap_or(-5.0)
            .trunc() as i32;
        crate::reputation::reward_karma(order, state, karma);
    }
    let Some(victim) = victim(order, state, from, owner) else {
        return false;
    };
    let facts = crate::scripting::Facts {
        order,
        state,
        speaker: None,
    };
    // A victim who ignores crime (`IgnoreCrime`, vtable +0x320 in
    // `008bfa40`) doesn't count it.
    if crate::script_functions::ignores_crime(state, victim)
        || state.dead.contains(&victim)
        || facts
            .detection(victim, PLAYER_REF)
            .map_or(true, |v| v <= crate::detection::SEEN)
    {
        return false;
    }
    witnessed(order, state, victim, 1, 0, "fReputationMinorCrimeNeg", 2.0);
    // A second witnessed theft the same day: the victim attacks.
    let day = state.global(order, "GameDay").unwrap_or(0.0) as u32;
    if state.last_theft_day == Some(day) {
        state.combat.insert(victim, PLAYER_REF);
    } else {
        state.last_theft_day = Some(day);
    }
    let warnings = game_setting(order, "iStealWarnings").unwrap_or(2.0) as u32;
    if state.steal_warnings < warnings {
        state.steal_warnings += 1;
    }
    true
}

/// A witnessed crime against `victim`: each of its factions that tracks
/// crime counts `minor` and `major` crimes and gives the player the
/// setting's infamy (`fReputationMinorCrimeNeg` 2, `fReputationMajor
/// CrimeNeg` 30) with its reputation (`WMI1`).
pub fn witnessed(
    order: &LoadOrder,
    state: &mut GameState,
    victim: FormId,
    minor: u32,
    major: u32,
    infamy_setting: &str,
    infamy_default: f32,
) {
    let infamy = if infamy_setting.is_empty() {
        0.0
    } else {
        game_setting(order, infamy_setting).unwrap_or(infamy_default)
    };
    for faction in crate::factions::factions_of(order, state, victim) {
        if !tracks_crime(order, faction) {
            continue;
        }
        let counts = state.faction_crimes.entry(faction).or_insert((0, 0));
        counts.0 += minor;
        counts.1 += major;
        let rep = order
            .get(faction)
            .and_then(|r| r.record().ok().map(|rec| (r, rec)))
            .and_then(|(rr, rec)| form_in(&rr, &rec, FourCC::new(b"WMI1")));
        if let Some(rep) = rep.filter(|_| infamy > 0.0) {
            crate::reputation::change(order, state, rep, crate::reputation::INFAMY, infamy);
        }
    }
    state.player_crimes.0 += minor;
    state.player_crimes.1 += major;
}

/// Who sees a crime: the living people in the player's cell (the game
/// asks every loaded actor) who detect the criminal (above 0), the victim
/// left out, and those who ignore crime (`IgnoreCrime`: skipped in the
/// assault and murder witness loops, `008c0460`, `008c09e0`).
pub fn witnesses(
    order: &LoadOrder,
    state: &GameState,
    criminal: FormId,
    victim: FormId,
) -> Vec<FormId> {
    let Some(cell) = state.player_cell else {
        return Vec::new();
    };
    let facts = crate::scripting::Facts {
        order,
        state,
        speaker: None,
    };
    order
        .references_in_cell(cell)
        .into_iter()
        .filter(|rr| rr.entry.header.kind.as_bytes() == b"ACHR")
        .map(|rr| rr.form_id)
        .filter(|&w| w != victim && w != criminal && !state.dead.contains(&w))
        .filter(|&w| !crate::script_functions::ignores_crime(state, w))
        .filter(|&w| {
            facts
                .detection(w, criminal)
                .is_some_and(|v| v > crate::detection::SEEN)
        })
        .collect()
}

/// The player hits someone who isn't fighting them (`008987f0`): a friend
/// or ally takes a few hits first (`iFriendHitCombatAllowed` 4 /
/// `iFriendHitNonCombatAllowed` 0 for friends, `iAllyHitCombatAllowed`
/// 1000 / `iAllyHitNonCombatAllowed` 3 for allies, "combat" meaning the
/// victim is fighting someone) and only remarks on it; past that, or anyone
/// else, it's an assault: a major crime for the player, and if anyone saw
/// it, +1 major crime (no infamy) for each of the victim's factions that
/// tracks crime. A friend or ally who ignores friendly hits
/// (`SetIgnoreFriendlyHits`, form flag 0x100000: `008987f0` returns at
/// once) takes no notice at all; a victim who ignores crime (`IgnoreCrime`:
/// `008c0460` returns at once) counts no crime but still fights back.
/// Whether the victim fights back.
pub fn assault(order: &LoadOrder, state: &mut GameState, victim: FormId) -> bool {
    use crate::factions::Reaction;
    let reaction = crate::factions::reaction(order, state, victim, PLAYER_REF);
    let fighting = state.combat.contains_key(&victim);
    let allowed = |name: &str, default: f32| game_setting(order, name).unwrap_or(default) as u32;
    let allowance = match (reaction, fighting) {
        (Reaction::Friend, true) => Some(allowed("iFriendHitCombatAllowed", 3.0)),
        (Reaction::Friend, false) => Some(allowed("iFriendHitNonCombatAllowed", 0.0)),
        (Reaction::Ally, true) => Some(allowed("iAllyHitCombatAllowed", 1000.0)),
        (Reaction::Ally, false) => Some(allowed("iAllyHitNonCombatAllowed", 3.0)),
        _ => None,
    };
    if allowance.is_some() && crate::script_functions::ignores_friendly_hits(order, state, victim) {
        return false;
    }
    if let Some(limit) = allowance {
        let hits = state.friendly_hits.entry(victim).or_insert(0);
        *hits += 1;
        if *hits <= limit {
            return false;
        }
    }
    assault_crime(order, state, victim);
    true
}

/// The assault crime itself (`008c0460`, also `SendAssaultAlarm`'s): none
/// against someone who ignores crime; else a major crime for the player,
/// and if anyone saw it, +1 major crime (no infamy) for each of the
/// victim's factions that tracks crime.
pub fn assault_crime(order: &LoadOrder, state: &mut GameState, victim: FormId) {
    if crate::script_functions::ignores_crime(state, victim) {
        return;
    }
    state.player_crimes.1 += 1;
    if !witnesses(order, state, PLAYER_REF, victim).is_empty() {
        witnessed(order, state, victim, 0, 1, "", 0.0);
        // `witnessed` counted it for the player as well.
        state.player_crimes.1 -= 1;
    }
}

/// The player (or a teammate) kills someone who wasn't fighting them: a
/// murder (`008c09e0`). If anyone saw it: +1 major crime and
/// `fReputationMajorCrimeNeg` (30) infamy for each of the victim's factions
/// that tracks crime, and the witnesses' crime-tracking factions hold the
/// player as an enemy (`008b8360`). The player becomes a murderer
/// (`IsPCAMurderer`) when the victim isn't evil. (Which deaths the game
/// counts as murder has two flags not traced; here: a person who wasn't
/// fighting the killer.)
pub fn murder(order: &LoadOrder, state: &mut GameState, victim: FormId, killer: FormId) {
    if crime_victim_is_creature(order, victim) {
        return;
    }
    state.player_crimes.1 += 1;
    let seen = witnesses(order, state, killer, victim);
    if !seen.is_empty() {
        witnessed(order, state, victim, 0, 1, "fReputationMajorCrimeNeg", 30.0);
        state.player_crimes.1 -= 1;
        for w in seen {
            for f in crate::factions::factions_of(order, state, w) {
                if tracks_crime(order, f) {
                    state.crime_enemies.insert(f);
                }
            }
        }
    }
    let evil = base_of(order, victim).is_some_and(|b| owner_is_evil(order, b));
    if !evil {
        state.player_murderer = true;
    }
}

fn crime_victim_is_creature(order: &LoadOrder, who: FormId) -> bool {
    crate::combat::is_creature(order, who)
}

/// Whether the player is trespassing in a cell (`00546da0`).
pub fn trespassing(order: &LoadOrder, state: &GameState, cell: FormId) -> bool {
    let Some(rr) = order.get(cell) else {
        return false;
    };
    let Ok(record) = rr.record() else {
        return false;
    };
    let Some(owner) = crate::script_functions::cell_owner_now(order, state, cell) else {
        return false;
    };
    if crate::script_functions::cell_is_public(order, state, cell) || record.get(XGLB).is_some() {
        return false;
    }
    if Some(owner) == base_of(order, PLAYER_REF) || owner == crate::dialogue::PLAYER_BASE {
        return false;
    }
    if !is_faction(order, owner) {
        return true;
    }
    let need = record
        .get(XRNK)
        .filter(|s| s.data.len() >= 4)
        .map_or(0, |s| le_u32(&s.data, 0) as i32);
    let rank = state
        .faction_changes
        .get(&(PLAYER_REF, owner))
        .map(|r| i32::from(*r))
        .or_else(|| {
            crate::factions::factions_of(order, state, PLAYER_REF)
                .contains(&owner)
                .then_some(0)
        })
        .unwrap_or(-1);
    rank < need
}
