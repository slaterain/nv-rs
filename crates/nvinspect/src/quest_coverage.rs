//! `coverage quests`: every quest of the base game (`QUST` records the
//! first loaded plugin, `FalloutNV.esm`, defines), read from the data and
//! nv-rs's own tables, as the Markdown page `docs/QUEST_COVERAGE.md`:
//!
//! * the quest record: stages, objectives, flags;
//! * how it starts: running from a new game (`DATA` flag 0x01), or the
//!   scripts that call `StartQuest` or `SetStage` on it (a stage set on a
//!   quest that isn't running starts it: `SetStage` `005c7140` →
//!   `0060d510` → `0060c9c0`, `docs/GOODSPRINGS_ROUTE.md`);
//! * the script functions its scripts call (its quest script, its stages'
//!   result scripts, its dialogue lines' result scripts and every other
//!   script that starts it, sets its stages or objectives or completes it),
//!   each checked against what nv-rs carries out
//!   (`coverage_cmd::function_handled`, so a rerun picks up new work), and
//!   the scripts nv-rs can't run because their source doesn't parse (nv-rs
//!   runs scripts from their source text);
//! * where those scripts move the player (`player.MoveTo`);
//! * how far it has been played, from the hand-kept table in
//!   [`crate::quest_played`].
//!
//! Scripts are read from their compiled form (`SCDA`), not their text: each
//! call's function number and its parameters, and the records a call names
//! through the script's reference list (`SCRO` records and `SCRV`
//! variables, numbered together from 1 in the order they're stored).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;

use esm::{FormId, FourCC, LoadOrder, Record};
use script::compiled::{self, Value};

use crate::coverage_cmd::function_handled;
use crate::quest_played::{self, PlayStatus};
use crate::CliError;

const SCHR: FourCC = FourCC::new(b"SCHR");
const SCDA: FourCC = FourCC::new(b"SCDA");
const SCTX: FourCC = FourCC::new(b"SCTX");
const SCRO: FourCC = FourCC::new(b"SCRO");
const SCRV: FourCC = FourCC::new(b"SCRV");
const SLSD: FourCC = FourCC::new(b"SLSD");
const SCVR: FourCC = FourCC::new(b"SCVR");
const SCRI: FourCC = FourCC::new(b"SCRI");
const INDX: FourCC = FourCC::new(b"INDX");
const QSTI: FourCC = FourCC::new(b"QSTI");
const QOBJ: FourCC = FourCC::new(b"QOBJ");
const DATA: FourCC = FourCC::new(b"DATA");
const QUST: FourCC = FourCC::new(b"QUST");
const SCPT: FourCC = FourCC::new(b"SCPT");
const INFO: FourCC = FourCC::new(b"INFO");
const CTDA: FourCC = FourCC::new(b"CTDA");
const PERK: FourCC = FourCC::new(b"PERK");
const PRKE: FourCC = FourCC::new(b"PRKE");
const CSNO: FourCC = FourCC::new(b"CSNO");

/// The player's reference (`PlayerRef`, 00000014).
const PLAYER_REF: FormId = FormId(0x14);

// ---------------------------------------------------------------------------
// Scripts in records
// ---------------------------------------------------------------------------

/// One entry of a script's reference list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptRef {
    /// `SCRO`: a record (load-order form ID).
    Form(FormId),
    /// `SCRV`: one of the script's own reference variables.
    Variable,
}

/// One script kept in a record: a script record has one; a quest one per
/// stage log entry, a dialogue line two (begin and end), a package three,
/// a terminal one per menu item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptUnit<'a> {
    /// For a quest's: the stage (`INDX`) it belongs to.
    pub stage: Option<i16>,
    /// `SCHR`'s script type (0 object, 1 quest, 0x100 effect).
    pub kind: u16,
    pub compiled: Option<&'a [u8]>,
    pub source: Option<&'a [u8]>,
    pub refs: Vec<ScriptRef>,
}

/// A record's scripts, each from its `SCHR` to the first subrecord that
/// isn't part of a script. `global` turns a form ID as stored into a
/// load-order one.
pub fn script_units(record: &Record, global: impl Fn(FormId) -> FormId) -> Vec<ScriptUnit<'_>> {
    let mut out: Vec<ScriptUnit> = Vec::new();
    let mut open = false;
    let mut stage = None;
    for sub in &record.subrecords {
        let k = sub.kind;
        if k == INDX {
            stage = sub.data.get(..2).map(|b| i16::from_le_bytes([b[0], b[1]]));
        }
        if k == SCHR {
            out.push(ScriptUnit {
                stage,
                kind: sub
                    .data
                    .get(16..18)
                    .map_or(0, |b| u16::from_le_bytes([b[0], b[1]])),
                compiled: None,
                source: None,
                refs: Vec::new(),
            });
            open = true;
            continue;
        }
        let Some(unit) = out.last_mut().filter(|_| open) else {
            continue;
        };
        if k == SCDA {
            unit.compiled = Some(&sub.data);
        } else if k == SCTX {
            unit.source = Some(&sub.data);
        } else if k == SCRO {
            let id = sub
                .data
                .get(..4)
                .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            unit.refs.push(ScriptRef::Form(global(FormId(id))));
        } else if k == SCRV {
            unit.refs.push(ScriptRef::Variable);
        } else if k != SLSD && k != SCVR {
            open = false;
        }
    }
    out
}

/// What a quest-changing call does to the quest it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuestAction {
    /// `StartQuest`.
    Starts,
    /// `SetStage`, which also starts a quest that isn't running.
    SetsStage,
    /// Sets an objective, completes, stops or resets it, or sets one of its
    /// variables.
    Drives,
}

/// The quest action of a function, by name.
pub fn quest_action(function: &str) -> Option<QuestAction> {
    match function {
        "StartQuest" => Some(QuestAction::Starts),
        "SetStage" => Some(QuestAction::SetsStage),
        "SetObjectiveDisplayed"
        | "SetObjectiveCompleted"
        | "CompleteQuest"
        | "CompleteAllObjectives"
        | "StopQuest"
        | "ResetQuest" => Some(QuestAction::Drives),
        _ => None,
    }
}

/// What one compiled script does: the functions it calls, the quests it
/// starts or drives, and where it moves the player.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitUse {
    pub functions: BTreeSet<u16>,
    /// Statement codes below 0x1000 that aren't the language's keywords
    /// (console commands, if a script ever had one).
    pub other_codes: BTreeSet<u16>,
    pub quests: BTreeSet<(FormId, QuestAction)>,
    /// `MoveTo`/`MoveToFade` on the player: the reference moved to.
    pub player_moves: BTreeSet<FormId>,
    /// Every record its reference list names.
    pub names: BTreeSet<FormId>,
}

/// Reads a compiled script.
pub fn unit_use(unit: &ScriptUnit) -> UnitUse {
    let mut u = UnitUse::default();
    let Some(bytes) = unit.compiled else {
        return u;
    };
    for r in &unit.refs {
        if let ScriptRef::Form(f) = r {
            u.names.insert(*f);
        }
    }
    let form = |r: u16| match unit.refs.get(usize::from(r).wrapping_sub(1)) {
        Some(ScriptRef::Form(f)) => Some(*f),
        _ => None,
    };
    for s in compiled::statements(bytes) {
        if s.code < 0x1000 && !(0x10..=0x1E).contains(&s.code) {
            u.other_codes.insert(s.code);
        }
        // `set Quest.variable to ...`: the target is `r`, the reference's
        // number, then the variable. Kept for every record a script sets a
        // variable on; only quests are looked up.
        if s.code == 0x15 && s.data.first() == Some(&b'r') {
            if let Some(q) = s
                .data
                .get(1..3)
                .and_then(|b| form(u16::from_le_bytes([b[0], b[1]])))
            {
                u.quests.insert((q, QuestAction::Drives));
            }
        }
    }
    for call in compiled::calls(bytes) {
        u.functions.insert(call.function);
        let name = script::function_name(call.function);
        let first = || match compiled::parameters(call.function, call.params).first() {
            Some(Value::Ref(r)) => form(*r),
            _ => None,
        };
        if let Some(action) = quest_action(&name) {
            if let Some(q) = first() {
                u.quests.insert((q, action));
            }
        }
        if (name == "MoveToMarker" || name == "MoveToMarkerWithFade")
            && call.on.and_then(form) == Some(PLAYER_REF)
        {
            if let Some(to) = first() {
                u.player_moves.insert(to);
            }
        }
    }
    u
}

// ---------------------------------------------------------------------------
// Every script in the load order
// ---------------------------------------------------------------------------

/// One script and what it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The record it's kept in.
    pub record: FormId,
    pub kind: FourCC,
    pub editor_id: String,
    /// A quest's stage, or a dialogue line's quest.
    pub stage: Option<i16>,
    pub quest: Option<FormId>,
    /// `SCHR`'s type (for script records: 1 quest script, 0x100 effect).
    pub script_kind: u16,
    /// It has source text and that parses (nv-rs runs scripts from their
    /// source).
    pub runs: bool,
    pub uses: UnitUse,
}

/// Every script in the load order (winning records), the records each
/// script record is attached to (`SCRI`), the condition functions each
/// quest's records ask, and the other records that start a quest.
#[derive(Debug, Default)]
pub struct Scripts {
    pub sites: Vec<Site>,
    pub users: HashMap<FormId, Vec<FormId>>,
    /// By quest: the functions its conditions (`CTDA`) ask: the quest's own,
    /// its stages' and its targets', and its dialogue lines'.
    pub conditions: HashMap<FormId, BTreeSet<u16>>,
    /// By quest: records other than scripts that start it, labelled: a
    /// perk's quest stage entry (`PRKE` kind 0, `DATA` quest and stage), a
    /// casino's winnings quest (`CSNO` `DATA` +48).
    pub other_starters: HashMap<FormId, BTreeSet<String>>,
    /// By quest: the types of those records (`PERK`, `CSNO`).
    pub named_by: HashMap<FormId, BTreeSet<FourCC>>,
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn scripts(order: &LoadOrder) -> Scripts {
    let mut out = Scripts::default();
    let mut kinds: Vec<FourCC> = order.type_counts().into_iter().map(|(k, _)| k).collect();
    kinds.sort();
    for kind in kinds {
        for rr in order.records_of_type(kind) {
            let Ok(record) = rr.record() else { continue };
            if let Some(s) = record.get(SCRI) {
                if let Some(b) = s.data.get(..4) {
                    let id = rr
                        .plugin
                        .to_global(FormId(u32::from_le_bytes([b[0], b[1], b[2], b[3]])));
                    out.users.entry(id).or_default().push(rr.form_id);
                }
            }
            let editor_id = record.editor_id().unwrap_or_default();
            let global = |id: u32| rr.plugin.to_global(FormId(id));
            // The quest an `INFO` belongs to; a quest itself.
            let owner = if kind == INFO {
                record
                    .get(QSTI)
                    .and_then(|s| u32_at(&s.data, 0))
                    .map(global)
            } else if kind == QUST {
                Some(rr.form_id)
            } else {
                None
            };
            if let Some(q) = owner {
                let asked = record
                    .get_all(CTDA)
                    .filter_map(|c| c.data.get(8..10))
                    .map(|f| u16::from_le_bytes([f[0], f[1]]));
                out.conditions.entry(q).or_default().extend(asked);
            }
            if kind == PERK {
                // `PRKE` kind 0 (quest stage): the next `DATA` is the quest
                // and the stage.
                let mut quest_entry = false;
                for sub in &record.subrecords {
                    if sub.kind == PRKE {
                        quest_entry = sub.data.first() == Some(&0);
                    } else if sub.kind == DATA && quest_entry {
                        quest_entry = false;
                        if let Some(q) = u32_at(&sub.data, 0).filter(|&q| q != 0) {
                            out.other_starters
                                .entry(global(q))
                                .or_default()
                                .insert(format!("perk {editor_id}"));
                            out.named_by.entry(global(q)).or_default().insert(kind);
                        }
                    }
                }
            }
            if kind == CSNO {
                if let Some(q) = record
                    .get(DATA)
                    .and_then(|d| u32_at(&d.data, 48))
                    .filter(|&q| q != 0)
                {
                    out.other_starters
                        .entry(global(q))
                        .or_default()
                        .insert(format!("casino {editor_id} (its winnings quest) [G]"));
                    out.named_by.entry(global(q)).or_default().insert(kind);
                }
            }
            if record.get(SCHR).is_none() {
                continue;
            }
            let quest = if kind == INFO {
                record.get(QSTI).and_then(|s| s.data.get(..4)).map(|b| {
                    rr.plugin
                        .to_global(FormId(u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
                })
            } else if kind == QUST {
                Some(rr.form_id)
            } else {
                None
            };
            for unit in script_units(&record, |f| rr.plugin.to_global(f)) {
                if unit.compiled.is_none() && unit.source.is_none() {
                    continue;
                }
                let runs = unit
                    .source
                    .is_some_and(|s| script::parse(&esm::text::decode_cp1252(s)).is_ok());
                out.sites.push(Site {
                    record: rr.form_id,
                    kind,
                    editor_id: editor_id.clone(),
                    stage: unit.stage,
                    quest,
                    script_kind: unit.kind,
                    runs,
                    uses: unit_use(&unit),
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Quests
// ---------------------------------------------------------------------------

/// One quest's row.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestRow {
    pub form_id: FormId,
    pub editor_id: String,
    pub flags: u8,
    pub stages: usize,
    pub objectives: usize,
    /// What starts it (labels): scripts calling `StartQuest` on it, perks
    /// and casinos naming it; not counting its own scripts.
    pub started_by: Vec<String>,
    /// Other scripts that set one of its stages (which starts it if it
    /// isn't running).
    pub staged_by: Vec<String>,
    /// Its scripts and the ones that drive it.
    pub scripts: usize,
    /// Of those, the ones nv-rs can't run (no source, or it doesn't parse).
    pub not_running: Vec<String>,
    /// Functions they call, and the ones nv-rs doesn't carry out.
    pub functions: BTreeSet<u16>,
    pub missing: Vec<String>,
    /// Functions only its conditions ask that nv-rs doesn't carry out (a
    /// condition on one is always false: its value is 0).
    pub missing_conditions: Vec<String>,
    /// The systems its scripts need that nv-rs lacks
    /// ([`quest_played::SYSTEM_GAPS`]).
    pub systems: Vec<&'static str>,
    /// Statement codes that aren't keywords or functions.
    pub other_codes: BTreeSet<u16>,
    /// Where the player is moved (worldspace or interior cell editor IDs).
    pub player_moves: BTreeSet<String>,
    pub played: PlayStatus,
    /// The hand-kept note, or empty.
    pub note: String,
    pub source: String,
}

impl QuestRow {
    pub fn start_game_enabled(&self) -> bool {
        self.flags & world::quest::START_GAME_ENABLED != 0
    }
}

/// `DATA` flags as words.
pub fn flag_words(flags: u8) -> String {
    let mut words = Vec::new();
    if flags & 0x01 != 0 {
        words.push("start game enabled".to_string());
    }
    if flags & 0x04 != 0 {
        words.push("repeated topics".to_string());
    }
    if flags & 0x08 != 0 {
        words.push("repeated stages".to_string());
    }
    let rest = flags & !0x0D;
    if rest != 0 {
        words.push(format!("0x{rest:02X}"));
    }
    words.join(", ")
}

fn editor_id_of(order: &LoadOrder, id: FormId) -> String {
    order
        .get(id)
        .and_then(|r| r.editor_id().ok().flatten())
        .unwrap_or_else(|| format!("{:08X}", id.0))
}

/// How a script is named in the table.
fn site_label(order: &LoadOrder, scripts: &Scripts, s: &Site) -> String {
    let quest = |q: Option<FormId>| q.map(|q| editor_id_of(order, q)).unwrap_or_default();
    match s.kind {
        k if k == QUST => match s.stage {
            Some(n) => format!("{} stage {n}", s.editor_id),
            None => format!("{} stage", s.editor_id),
        },
        k if k == INFO => format!("{} dialogue", quest(s.quest)),
        k if k == SCPT => {
            let users = scripts.users.get(&s.record).map_or(&[][..], |u| &u[..]);
            match users.first() {
                Some(first) => {
                    let more = if users.len() > 1 {
                        format!(" +{}", users.len() - 1)
                    } else {
                        String::new()
                    };
                    let what = order
                        .get(*first)
                        .map(|r| r.entry.header.kind.to_string())
                        .unwrap_or_default();
                    format!(
                        "{} (on {what} {}{more})",
                        s.editor_id,
                        editor_id_of(order, *first)
                    )
                }
                None if s.script_kind == 0x100 => format!("{} (effect)", s.editor_id),
                None => s.editor_id.clone(),
            }
        }
        k => {
            let name = if s.editor_id.is_empty() {
                format!("{:08X}", s.record.0)
            } else {
                s.editor_id.clone()
            };
            format!("{k} {name}")
        }
    }
}

/// Where a reference stands: its worldspace, or its interior cell.
fn place_of(order: &LoadOrder, reference: FormId) -> Option<String> {
    let rr = order.get(reference)?;
    if let Some(w) = order.world_of(&rr) {
        return Some(editor_id_of(order, w));
    }
    order.cell_of(&rr).map(|c| editor_id_of(order, c))
}

/// Every base-game quest's row, by editor ID.
pub fn quest_rows(order: &LoadOrder, scripts: &Scripts) -> Vec<QuestRow> {
    let base = order.plugins().first().map(|p| p.load_index);
    // For each quest: the scripts that start it, and the ones that start
    // or drive it.
    let mut starters: HashMap<FormId, Vec<(usize, QuestAction)>> = HashMap::new();
    let mut drivers: HashMap<FormId, Vec<usize>> = HashMap::new();
    let mut own: HashMap<FormId, Vec<usize>> = HashMap::new();
    for (i, s) in scripts.sites.iter().enumerate() {
        if let Some(q) = s.quest {
            own.entry(q).or_default().push(i);
        }
        for &(q, action) in &s.uses.quests {
            if s.quest == Some(q) {
                continue;
            }
            drivers.entry(q).or_default().push(i);
            if action != QuestAction::Drives {
                starters.entry(q).or_default().push((i, action));
            }
        }
    }
    let script_site: HashMap<FormId, usize> = scripts
        .sites
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == SCPT)
        .map(|(i, s)| (s.record, i))
        .collect();
    let mut rows = Vec::new();
    for rr in order.records_of_type(QUST) {
        if Some(rr.form_id.mod_index()) != base {
            continue;
        }
        let Ok(record) = rr.record() else { continue };
        let editor_id = record.editor_id().unwrap_or_default();
        let flags = record
            .get(DATA)
            .and_then(|d| d.data.first().copied())
            .unwrap_or(0);
        let stages: BTreeSet<i16> = record
            .get_all(INDX)
            .filter_map(|s| s.data.get(..2).map(|b| i16::from_le_bytes([b[0], b[1]])))
            .collect();
        let objectives: BTreeSet<i32> = record
            .get_all(QOBJ)
            .filter_map(|s| {
                s.data
                    .get(..4)
                    .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            })
            .collect();
        let quest_script = record.get(SCRI).and_then(|s| s.data.get(..4)).map(|b| {
            rr.plugin
                .to_global(FormId(u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        });
        // Its scripts: its own (stages, dialogue), its quest script, and
        // the ones that start or drive it.
        let mut mine: BTreeSet<usize> = own
            .get(&rr.form_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        if let Some(i) = quest_script.and_then(|s| script_site.get(&s)) {
            mine.insert(*i);
        }
        mine.extend(drivers.get(&rr.form_id).into_iter().flatten().copied());
        let own_script = quest_script.and_then(|s| script_site.get(&s).copied());
        let by = |wanted: QuestAction| -> BTreeSet<String> {
            starters
                .get(&rr.form_id)
                .into_iter()
                .flatten()
                .filter(|&&(i, action)| action == wanted && Some(i) != own_script)
                .map(|&(i, _)| site_label(order, scripts, &scripts.sites[i]))
                .collect()
        };
        let mut started_by = by(QuestAction::Starts);
        let staged_by: Vec<String> = by(QuestAction::SetsStage)
            .into_iter()
            .filter(|l| !started_by.contains(l))
            .collect();
        started_by.extend(
            scripts
                .other_starters
                .get(&rr.form_id)
                .into_iter()
                .flatten()
                .cloned(),
        );
        let mut functions = BTreeSet::new();
        let mut other_codes = BTreeSet::new();
        let mut not_running = BTreeSet::new();
        let mut player_moves = BTreeSet::new();
        for &i in &mine {
            let s = &scripts.sites[i];
            functions.extend(s.uses.functions.iter().copied());
            other_codes.extend(s.uses.other_codes.iter().copied());
            if !s.runs {
                not_running.insert(site_label(order, scripts, s));
            }
            for &to in &s.uses.player_moves {
                if let Some(p) = place_of(order, to) {
                    player_moves.insert(p);
                }
            }
        }
        let missing: Vec<String> = functions
            .iter()
            .map(|&f| script::function_name(f))
            .filter(|name| !function_handled(name))
            .collect();
        let missing_conditions: Vec<String> = scripts
            .conditions
            .get(&rr.form_id)
            .into_iter()
            .flatten()
            .filter(|f| !functions.contains(f))
            .map(|&f| script::function_name(f))
            .filter(|name| !function_handled(name))
            .collect();
        let names: BTreeSet<String> = functions
            .iter()
            .map(|&f| script::function_name(f))
            .collect();
        let named_by = scripts.named_by.get(&rr.form_id);
        let systems: Vec<&'static str> = quest_played::SYSTEM_GAPS
            .iter()
            .filter(|g| {
                g.functions.iter().any(|f| names.contains(*f))
                    || g.records.iter().any(|k| {
                        named_by.is_some_and(|n| n.iter().any(|t| t.as_bytes() == k.as_bytes()))
                    })
            })
            .map(|g| g.system)
            .collect();
        // Other scripts that name it (reading or setting its variables,
        // asking its stages) without being its own.
        let named = scripts
            .sites
            .iter()
            .enumerate()
            .filter(|(i, s)| {
                s.quest != Some(rr.form_id)
                    && Some(*i) != quest_script.and_then(|q| script_site.get(&q).copied())
                    && s.uses.names.contains(&rr.form_id)
            })
            .count();
        let hand = quest_played::entry(&editor_id);
        let played = match hand {
            Some(e) => e.status,
            None if started_by.is_empty()
                && staged_by.is_empty()
                && flags & world::quest::START_GAME_ENABLED == 0 =>
            {
                PlayStatus::NeverStarted
            }
            None => PlayStatus::NotPlayed,
        };
        rows.push(QuestRow {
            form_id: rr.form_id,
            editor_id,
            flags,
            stages: stages.len(),
            objectives: objectives.len(),
            started_by: started_by.into_iter().collect(),
            staged_by,
            scripts: mine.len(),
            not_running: not_running.into_iter().collect(),
            functions,
            missing,
            missing_conditions,
            systems,
            other_codes,
            player_moves,
            played,
            note: match hand {
                Some(e) => e.note.to_string(),
                None if played == PlayStatus::NeverStarted && named > 0 => format!(
                    "Nothing starts it [C]; {named} other scripts name it (its variables, stages \
                     or objectives) without starting it, so it holds data for them [G]."
                ),
                None if played == PlayStatus::NeverStarted => {
                    "Nothing in the data starts it or names it in a script [C].".to_string()
                }
                None => String::new(),
            },
            source: hand.map(|e| e.source.to_string()).unwrap_or_default(),
        });
    }
    rows.sort_by(|a, b| {
        a.editor_id
            .to_ascii_lowercase()
            .cmp(&b.editor_id.to_ascii_lowercase())
    });
    rows
}

// ---------------------------------------------------------------------------
// The page
// ---------------------------------------------------------------------------

/// What holds a quest back: the systems nv-rs lacks that its scripts need,
/// then the functions its scripts call or its conditions ask that nv-rs
/// doesn't carry out.
pub fn blocker_keys(r: &QuestRow) -> Vec<String> {
    let mut keys: Vec<String> = r.systems.iter().map(|s| s.to_string()).collect();
    keys.extend(r.missing.iter().cloned());
    keys.extend(r.missing_conditions.iter().cloned());
    keys
}

/// Every blocker with the quests it holds back, most quests first (then by
/// name).
pub fn blockers(rows: &[QuestRow]) -> Vec<(String, Vec<&QuestRow>)> {
    let mut by: BTreeMap<String, Vec<&QuestRow>> = BTreeMap::new();
    for r in rows {
        for k in blocker_keys(r) {
            by.entry(k).or_default().push(r);
        }
    }
    let mut out: Vec<(String, Vec<&QuestRow>)> = by.into_iter().collect();
    out.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    out
}

/// A list cut to `n` items, each in backticks, with how many were left out.
fn some(items: &[String], n: usize) -> String {
    let mut s = items
        .iter()
        .take(n)
        .map(|i| format!("`{i}`"))
        .collect::<Vec<_>>()
        .join(", ");
    if items.len() > n {
        s.push_str(&format!(" +{} more", items.len() - n));
    }
    s
}

/// The editor IDs of some rows.
fn ids(rows: &[&QuestRow]) -> Vec<String> {
    rows.iter().map(|r| r.editor_id.clone()).collect()
}

/// The page, from the rows.
pub fn write_page(out: &mut impl Write, rows: &[QuestRow]) -> Result<(), CliError> {
    let count = |p: PlayStatus| rows.iter().filter(|r| r.played == p).count();
    let blocking = blockers(rows);
    // A quest's main blocker: the one that holds back the most quests.
    let rank: HashMap<&str, usize> = blocking
        .iter()
        .enumerate()
        .map(|(i, (k, _))| (k.as_str(), i))
        .collect();
    let main_blocker = |r: &QuestRow| {
        blocker_keys(r)
            .iter()
            .filter_map(|k| rank.get(k.as_str()).copied())
            .min()
    };

    writeln!(out, "# Base-game quest coverage matrix")?;
    writeln!(out)?;
    writeln!(
        out,
        "Every quest (`QUST`) in `FalloutNV.esm`, what its scripts need from the engine, and how far \
         it has been played in the nv-rs viewer. **Generated**; rerun it after changing nv-rs's \
         script functions, the played table or the system gaps, and commit the result:"
    )?;
    writeln!(out)?;
    writeln!(out, "```")?;
    writeln!(
        out,
        "cargo run --release -p nvinspect -- \"<Data>\\FalloutNV.esm\" coverage quests > docs/QUEST_COVERAGE.md"
    )?;
    writeln!(out, "```")?;
    writeln!(out)?;
    writeln!(
        out,
        "The generator is `crates/nvinspect/src/quest_coverage.rs`. Two small tables in \
         `crates/nvinspect/src/quest_played.rs` are kept by hand: how far each quest has been played \
         (status, note, evidence) and the systems nv-rs lacks (with the functions that need them and \
         the evidence). Everything else is read from the data and from nv-rs's own function tables. \
         Quests are named by editor ID; no game text is quoted and nothing decompiled is included."
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "**{} quests**: {} PLAYED, {} PARTIAL, {} NOT PLAYED, {} NEVER STARTED.",
        rows.len(),
        count(PlayStatus::Played),
        count(PlayStatus::Partial),
        count(PlayStatus::NotPlayed),
        count(PlayStatus::NeverStarted)
    )?;
    writeln!(out)?;
    let enabled = rows.iter().filter(|r| r.start_game_enabled()).count();
    let clear = rows.iter().filter(|r| blocker_keys(r).is_empty()).count();
    let all_run = rows.iter().filter(|r| r.not_running.is_empty()).count();
    let called: BTreeSet<u16> = rows
        .iter()
        .flat_map(|r| r.functions.iter().copied())
        .collect();
    let not_done = called
        .iter()
        .filter(|&&f| !function_handled(&script::function_name(f)))
        .count();
    let asked: BTreeSet<&String> = rows.iter().flat_map(|r| &r.missing_conditions).collect();
    writeln!(
        out,
        "- Running from a new game (start game enabled): {enabled}."
    )?;
    writeln!(
        out,
        "- Script functions the quests' scripts call: {}; carried out by nv-rs: {} ({not_done} not). \
         Functions only their conditions ask that nv-rs doesn't carry out: {}.",
        called.len(),
        called.len() - not_done,
        asked.len()
    )?;
    writeln!(
        out,
        "- Quests with nothing listed against them (no missing function, condition or system): \
         {clear} of {}. That says nothing about how well the functions they use work: see the \
         played status.",
        rows.len()
    )?;
    writeln!(
        out,
        "- Quests whose scripts all parse (so nv-rs can run them): {all_run} of {}.",
        rows.len()
    )?;
    let codes: BTreeSet<u16> = rows
        .iter()
        .flat_map(|r| r.other_codes.iter().copied())
        .collect();
    if codes.is_empty() {
        writeln!(
            out,
            "- Console commands in the quests' compiled scripts: none (every statement is a \
             keyword or a script function)."
        )?;
    } else {
        let list: Vec<String> = codes.iter().map(|c| format!("{c:#x}")).collect();
        writeln!(
            out,
            "- Statement codes that are neither keywords nor script functions: {}.",
            list.join(", ")
        )?;
    }
    writeln!(out)?;

    writeln!(out, "## How to read it")?;
    writeln!(out)?;
    for line in [
        "* **PLAYED**: reached its end in the viewer, with its stages moved by the game's own \
         scripts, triggers and dialogue (a test character may set the start).",
        "* **PARTIAL**: some of it was played; the note says how far and what was helped along \
         with a console line (`--run \"SetStage ...\"`).",
        "* **NOT PLAYED**: no recorded play; the row lists what its scripts need that nv-rs lacks.",
        "* **NEVER STARTED**: nothing in the data starts it (it doesn't run from a new game, no \
         script calls `StartQuest` or `SetStage` on it, no perk or casino names it), so the \
         original never plays it either, unless the engine starts it itself [C]. Computed from the \
         data; the played table can override it.",
        "* **[G]** marks a guess, **[C]** something to check against the original game.",
        "* **Starts**: `new game` when the quest runs from the start (`DATA` flag 0x01); \
         *started by* the scripts that call `StartQuest` on it, a perk's quest-stage entry, or a \
         casino whose winnings quest it is [G]; *stages set by* the other scripts that call \
         `SetStage` on it (a stage set on a quest that isn't running starts it: `SetStage` \
         `005c7140` → `0060d510` → `0060c9c0`, traced in docs/GOODSPRINGS_ROUTE.md; most of these \
         run while it already runs). A script is named by where it is: another quest's stage \
         (`VCG01 stage 200`), a quest's dialogue lines (`VDialogueBenny dialogue`), a script \
         with the record it's on (`SomeScript (on NPC_ Someone +2)`, +2: two more records carry \
         it), a package, a terminal or a placed reference. Its own stages, dialogue and quest \
         script don't count: they run only while it runs.",
        "* **Flags**: `DATA` byte 0: start game enabled (0x01), repeated stages (0x08, named in \
         `world::quest`), repeated topics (0x04, the editor's name [G]); other bits in hex.",
        "* **Scripts**: how many compiled scripts belong to the quest: its quest script, its \
         stages' result scripts, its dialogue lines' result scripts, and every other script that \
         starts it, sets one of its objectives or variables, completes, stops or resets it. \
         Read from the compiled scripts (function numbers, and the records their reference lists name), not \
         from their text. A shared script (a town's freeform quest, a generic actor script) counts \
         for every quest it drives.",
        "* **Systems missing**: systems those scripts open or need that nv-rs doesn't have yet, \
         from the hand-kept list below (a function nv-rs answers but whose screen or playback \
         isn't built).",
        "* **Missing functions**: script functions those scripts call that nv-rs doesn't carry out \
         (not in `world::scripting::HANDLED` or `world::script_functions::handled()`), then, \
         marked (condition), ones only the quest's conditions ask (its own, its stages', its \
         targets' and its dialogue lines' `CTDA`): nv-rs answers those with 0. A function counted \
         as carried out may still be incomplete.",
        "* **Can't run**: scripts nv-rs can't run because their source doesn't parse (nv-rs runs \
         scripts from their source text, not the compiled form).",
        "* **Moves the player to**: the worldspaces and interior cells its scripts send the player \
         to (`player.MoveTo`, `MoveToFade`), from the target reference's place. Whether each loads \
         well in the viewer isn't checked here.",
        "* Rows are grouped by status. Within a group, rows are ordered by their main blocker (the \
         one listed against the most quests), so quests waiting on the same system sit together; \
         rows with nothing listed come last.",
    ] {
        writeln!(out, "{line}")?;
    }
    writeln!(out)?;

    writeln!(out, "## Most common blockers")?;
    writeln!(out)?;
    writeln!(out, "### Systems nv-rs lacks")?;
    writeln!(out)?;
    writeln!(
        out,
        "Kept by hand (`quest_played::SYSTEM_GAPS`): a system and the functions that need it, \
         counted over the quests whose scripts call one of them (or, for a record type listed, \
         the quests such a record names)."
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "| System | Functions (records) | Quests | Some of them | Why, and the evidence |"
    )?;
    writeln!(out, "| --- | --- | --- | --- | --- |")?;
    let mut gaps: Vec<(&quest_played::SystemGap, Vec<&QuestRow>)> = quest_played::SYSTEM_GAPS
        .iter()
        .map(|g| {
            let quests: Vec<&QuestRow> = rows
                .iter()
                .filter(|r| r.systems.contains(&g.system))
                .collect();
            (g, quests)
        })
        .collect();
    gaps.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.system.cmp(b.0.system)));
    for (g, quests) in &gaps {
        let functions: Vec<String> = g.functions.iter().map(|f| f.to_string()).collect();
        let mut needs = some(&functions, 8);
        if !g.records.is_empty() {
            needs.push_str(&format!(" ({})", g.records.join(", ")));
        }
        writeln!(
            out,
            "| {} | {} | {} | {} | {} ({}) |",
            g.system,
            needs,
            quests.len(),
            some(&ids(quests), 6),
            g.note,
            g.source
        )?;
    }
    writeln!(out)?;
    writeln!(out, "### Script functions nv-rs doesn't carry out")?;
    writeln!(out)?;
    writeln!(
        out,
        "By how many quests' scripts call them, then how many quests' conditions only ask them. \
         Carrying out the top ones unblocks the most quests."
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "| Function | Called by | Asked by conditions of | Some of them |"
    )?;
    writeln!(out, "| --- | --- | --- | --- |")?;
    for (k, quests) in &blocking {
        if quest_played::SYSTEM_GAPS.iter().any(|g| g.system == k) {
            continue;
        }
        let calls = quests.iter().filter(|r| r.missing.contains(k)).count();
        let asks = quests.len() - calls;
        writeln!(
            out,
            "| `{k}` | {calls} | {asks} | {} |",
            some(&ids(quests), 6)
        )?;
    }
    let cant: Vec<&QuestRow> = rows.iter().filter(|r| !r.not_running.is_empty()).collect();
    if !cant.is_empty() {
        writeln!(out)?;
        writeln!(
            out,
            "Quests with a script whose source doesn't parse (`nvinspect <Data> scripts` lists \
             why): {}.",
            some(&ids(&cant), 20)
        )?;
    }
    writeln!(out)?;

    writeln!(out, "## Matrix")?;
    for status in [
        PlayStatus::Played,
        PlayStatus::Partial,
        PlayStatus::NotPlayed,
        PlayStatus::NeverStarted,
    ] {
        let mut these: Vec<&QuestRow> = rows.iter().filter(|r| r.played == status).collect();
        if these.is_empty() {
            continue;
        }
        these.sort_by(|a, b| {
            (main_blocker(a).unwrap_or(usize::MAX), &a.editor_id)
                .cmp(&(main_blocker(b).unwrap_or(usize::MAX), &b.editor_id))
        });
        writeln!(out)?;
        writeln!(out, "### {} ({})", status.label(), these.len())?;
        writeln!(out)?;
        writeln!(
            out,
            "| Quest | Form ID | Stages | Objectives | Flags | Starts | Scripts | Systems missing | Missing functions | Can't run | Moves the player to | Notes |"
        )?;
        writeln!(
            out,
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
        )?;
        for r in these {
            writeln!(out, "{}", row_line(r))?;
        }
    }
    Ok(())
}

/// One row of the matrix.
fn row_line(r: &QuestRow) -> String {
    let mut starts = Vec::new();
    if r.start_game_enabled() {
        starts.push("new game".to_string());
    }
    if !r.started_by.is_empty() {
        starts.push(format!("started by {}", some(&r.started_by, 4)));
    }
    if !r.staged_by.is_empty() {
        starts.push(format!("stages set by {}", some(&r.staged_by, 4)));
    }
    let starts = if starts.is_empty() {
        "nothing".to_string()
    } else {
        starts.join("; ")
    };
    let mut missing: Vec<String> = r.missing.iter().map(|f| format!("`{f}`")).collect();
    missing.extend(
        r.missing_conditions
            .iter()
            .map(|f| format!("`{f}` (condition)")),
    );
    if missing.len() > 8 {
        let more = missing.len() - 8;
        missing.truncate(8);
        missing.push(format!("+{more} more"));
    }
    let systems: Vec<String> = r.systems.iter().map(|s| s.to_string()).collect();
    let moves: Vec<String> = r.player_moves.iter().cloned().collect();
    let mut note = r.note.clone();
    if !r.source.is_empty() {
        note.push_str(&format!(" (Evidence: {}.)", r.source));
    }
    format!(
        "| `{}` | {:08X} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        r.editor_id,
        r.form_id.0,
        r.stages,
        r.objectives,
        flag_words(r.flags),
        starts,
        r.scripts,
        systems.join(", "),
        missing.join(", "),
        some(&r.not_running, 3),
        some(&moves, 4),
        note.trim()
    )
}

/// Every hand-kept row names a quest the first plugin has, once. A row for a
/// quest the data doesn't have (a typo, a renamed record, a DLC quest) would
/// otherwise do nothing at all and quietly lose its evidence, so the command
/// stops instead of writing a page that reads as if the quest were unplayed.
pub fn check_ledger(rows: &[QuestRow]) -> Result<(), CliError> {
    let have: BTreeSet<String> = rows
        .iter()
        .map(|r| r.editor_id.to_ascii_lowercase())
        .collect();
    let mut listed = BTreeSet::new();
    let mut bad = Vec::new();
    for e in quest_played::ENTRIES {
        let id = e.quest.to_ascii_lowercase();
        if !listed.insert(id.clone()) || !have.contains(&id) {
            bad.push(e.quest);
        }
    }
    if bad.is_empty() {
        return Ok(());
    }
    Err(CliError::Usage(format!(
        "coverage quests: {} hand-kept row(s) name a quest the data has not, or name one twice: {}",
        bad.len(),
        bad.join(", ")
    )))
}

pub fn quests(out: &mut impl Write, order: &LoadOrder) -> Result<(), CliError> {
    let scripts = scripts(order);
    let rows = quest_rows(order, &scripts);
    check_ledger(&rows)?;
    write_page(out, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testdata::{group, record, sub, zstr};

    // Compiled scripts, as the game stores them.

    fn code(name: &str) -> [u8; 2] {
        (0x1000 + script::function(name).unwrap().0).to_le_bytes()
    }

    /// A call statement.
    fn call(name: &str, params: &[u8]) -> Vec<u8> {
        let mut v = code(name).to_vec();
        v.extend((params.len() as u16).to_le_bytes());
        v.extend(params);
        v
    }

    /// A call on reference `r`.
    fn call_on(r: u16, name: &str, params: &[u8]) -> Vec<u8> {
        let mut v = vec![0x1C, 0x00];
        v.extend(r.to_le_bytes());
        v.extend(call(name, params));
        v
    }

    /// `set r.s<var> to 5`.
    fn set_ref_var(r: u16, var: u16) -> Vec<u8> {
        let mut data = vec![b'r'];
        data.extend(r.to_le_bytes());
        data.push(b's');
        data.extend(var.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(b" 5");
        let mut v = vec![0x15, 0x00];
        v.extend((data.len() as u16).to_le_bytes());
        v.extend(data);
        v
    }

    /// One parameter naming reference `r`.
    fn one_ref(r: u16) -> Vec<u8> {
        let mut v = vec![1, 0, b'r'];
        v.extend(r.to_le_bytes());
        v
    }

    enum R {
        Form(u32),
        Var(u32),
    }

    /// A script's subrecords: header (type), compiled form, source and
    /// reference list.
    fn script(kind: u16, compiled: &[u8], source: &str, refs: &[R]) -> Vec<u8> {
        let mut schr = vec![0u8; 20];
        schr[16..18].copy_from_slice(&kind.to_le_bytes());
        let mut v = sub(b"SCHR", &schr);
        v.extend(sub(b"SCDA", compiled));
        v.extend(sub(b"SCTX", source.as_bytes()));
        for r in refs {
            v.extend(match r {
                R::Form(f) => sub(b"SCRO", &f.to_le_bytes()),
                R::Var(i) => sub(b"SCRV", &i.to_le_bytes()),
            });
        }
        v
    }

    fn quest(id: u32, editor_id: &str, flags: u8, rest: &[u8]) -> Vec<u8> {
        let mut d = sub(b"EDID", &zstr(editor_id));
        d.extend(sub(b"DATA", &[flags, 0, 0, 0, 0, 0, 0, 0]));
        d.extend(rest);
        record(b"QUST", id, &d)
    }

    fn plugin(records: &[u8]) -> LoadOrder {
        let mut bytes = record(b"TES4", 0, &sub(b"HEDR", &[0; 12]));
        bytes.extend(records);
        LoadOrder::single("Test.esm", None, esm::Plugin::from_bytes(bytes).unwrap()).unwrap()
    }

    const LEVER_SOURCE: &str = "scn TestLeverScript\nbegin OnActivate\nStartQuest TestQuest\n\
                                player.MoveTo TestMarker\nend\n";

    /// A small game: quests started from a new game, by a lever's script,
    /// by another quest's dialogue line and by a perk, one nothing starts,
    /// and one in the played table.
    fn game() -> LoadOrder {
        let mut quests = Vec::new();
        // Running from a new game; its quest script calls a function nv-rs
        // doesn't carry out.
        let mut d = sub(b"SCRI", &0x200u32.to_le_bytes());
        d.extend(sub(b"INDX", &10i16.to_le_bytes()));
        quests.extend(quest(0x100, "TestStarted", 0x01, &d));
        // Started by the lever; its own stage 10 sets its stage 20.
        let mut d = sub(b"INDX", &10i16.to_le_bytes());
        d.extend(sub(b"QSDT", &[0]));
        let mut p = vec![2, 0, b'r', 1, 0, b'n'];
        p.extend(20i32.to_le_bytes());
        d.extend(script(
            0,
            &call("SetStage", &p),
            "SetStage TestQuest 20\n",
            &[R::Form(0x101)],
        ));
        d.extend(sub(b"INDX", &20i16.to_le_bytes()));
        d.extend(sub(b"QOBJ", &10i32.to_le_bytes()));
        d.extend(sub(b"NNAM", &zstr("objective")));
        quests.extend(quest(0x101, "TestQuest", 0, &d));
        quests.extend(quest(0x102, "TestStaged", 0, &[]));
        quests.extend(quest(0x103, "TestOrphan", 0, &[]));
        quests.extend(quest(0x104, "TestPerkQuest", 0, &[]));
        quests.extend(quest(0x105, "VCG02", 0, &[]));
        let mut bytes = group(*b"QUST", 0, &quests);

        let mut scripts = Vec::new();
        let mut d = sub(b"EDID", &zstr("TestStartedScript"));
        d.extend(script(
            1,
            &call("StartCannibal", &one_ref(1)),
            "scn TestStartedScript\nbegin GameMode\nplayer.StartCannibal player\nend\n",
            &[R::Form(0x14)],
        ));
        scripts.extend(record(b"SCPT", 0x200, &d));
        // The lever: a variable first, so the quest is reference 2, the
        // player 3 and the marker 4.
        let mut compiled = call("StartQuest", &one_ref(2));
        compiled.extend(call_on(3, "MoveTo", &one_ref(4)));
        compiled.extend(set_ref_var(2, 1));
        let mut d = sub(b"EDID", &zstr("TestLeverScript"));
        d.extend(script(
            0,
            &compiled,
            LEVER_SOURCE,
            &[R::Var(1), R::Form(0x101), R::Form(0x14), R::Form(0x500)],
        ));
        scripts.extend(record(b"SCPT", 0x201, &d));
        bytes.extend(group(*b"SCPT", 0, &scripts));

        let mut d = sub(b"EDID", &zstr("TestLever"));
        d.extend(sub(b"SCRI", &0x201u32.to_le_bytes()));
        bytes.extend(group(*b"ACTI", 0, &record(b"ACTI", 0x400, &d)));

        // TestQuest's line asks GetDisposition; TestStarted's line sets
        // TestStaged's stage 10 (its source doesn't parse).
        let mut infos = Vec::new();
        let mut d = sub(b"QSTI", &0x101u32.to_le_bytes());
        let mut ctda = vec![0u8; 28];
        ctda[8..10].copy_from_slice(&script::function("GetDisposition").unwrap().0.to_le_bytes());
        d.extend(sub(b"CTDA", &ctda));
        infos.extend(record(b"INFO", 0x300, &d));
        let mut d = sub(b"QSTI", &0x100u32.to_le_bytes());
        let mut p = vec![2, 0, b'r', 1, 0, b'n'];
        p.extend(10i32.to_le_bytes());
        d.extend(script(
            0,
            &call("SetStage", &p),
            BAD_SOURCE,
            &[R::Form(0x102)],
        ));
        infos.extend(record(b"INFO", 0x301, &d));
        bytes.extend(group(*b"DIAL", 0, &infos));

        // A perk whose quest-stage entry names TestPerkQuest.
        let mut d = sub(b"EDID", &zstr("TestPerk"));
        d.extend(sub(b"PRKE", &[0, 0, 0]));
        let mut data = 0x104u32.to_le_bytes().to_vec();
        data.extend([10, 0, 0, 0]);
        d.extend(sub(b"DATA", &data));
        d.extend(sub(b"PRKF", &[]));
        bytes.extend(group(*b"PERK", 0, &record(b"PERK", 0x600, &d)));

        // The lever's marker, in an interior cell.
        let mut cell = sub(b"EDID", &zstr("TestCell"));
        cell.extend(sub(b"DATA", &[1]));
        let mut contents = record(b"CELL", 0x700, &cell);
        contents.extend(group(
            0x700u32.to_le_bytes(),
            6,
            &group(
                0x700u32.to_le_bytes(),
                9,
                &testdata::placed(0x500, 0x400, [0.0; 3], [0.0; 3], &[]),
            ),
        ));
        bytes.extend(group(
            *b"CELL",
            0,
            &group([0; 4], 2, &group([0; 4], 3, &contents)),
        ));
        plugin(&bytes)
    }

    const BAD_SOURCE: &str = "if (\n";

    #[test]
    fn splits_a_records_scripts_and_numbers_their_references_together() {
        let order = game();
        let rr = order.get(FormId(0x101)).unwrap();
        let record = rr.record().unwrap();
        let units = script_units(&record, |f| f);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].stage, Some(10));
        assert_eq!(units[0].refs, vec![ScriptRef::Form(FormId(0x101))]);

        let rr = order.get(FormId(0x201)).unwrap();
        let record = rr.record().unwrap();
        let units = script_units(&record, |f| f);
        assert_eq!(units[0].refs[0], ScriptRef::Variable);
        let u = unit_use(&units[0]);
        // The quest is reference 2: counted after the variable.
        assert!(u.quests.contains(&(FormId(0x101), QuestAction::Starts)));
        assert!(u.quests.contains(&(FormId(0x101), QuestAction::Drives)));
        assert_eq!(u.player_moves, BTreeSet::from([FormId(0x500)]));
        let names: Vec<String> = u
            .functions
            .iter()
            .map(|&f| script::function_name(f))
            .collect();
        assert_eq!(names, ["StartQuest", "MoveToMarker"]);
        assert!(u.other_codes.is_empty());
    }

    #[test]
    fn finds_how_each_quest_starts_and_what_it_lacks() {
        // The functions used as missing here must stay missing for the test
        // to mean anything.
        assert!(!function_handled("StartCannibal"), "update this test");
        assert!(!function_handled("GetDisposition"), "update this test");
        assert!(script::parse(BAD_SOURCE).is_err());
        assert!(script::parse(LEVER_SOURCE).is_ok());

        let order = game();
        let scripts = scripts(&order);
        let rows = quest_rows(&order, &scripts);
        let ids: Vec<&str> = rows.iter().map(|r| r.editor_id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "TestOrphan",
                "TestPerkQuest",
                "TestQuest",
                "TestStaged",
                "TestStarted",
                "VCG02"
            ]
        );
        let row = |id: &str| rows.iter().find(|r| r.editor_id == id).unwrap();

        let r = row("TestStarted");
        assert!(r.start_game_enabled());
        assert_eq!(r.stages, 1);
        assert_eq!(r.missing, ["StartCannibal"]);
        assert_eq!(r.played, PlayStatus::NotPlayed);
        // Its dialogue line sets another quest's stage: not one of its own
        // scripts, and not a start of its own.
        assert!(r.started_by.is_empty() && r.staged_by.is_empty());

        let r = row("TestQuest");
        assert_eq!((r.stages, r.objectives), (2, 1));
        assert_eq!(r.started_by, ["TestLeverScript (on ACTI TestLever)"]);
        // Its own stage script sets its stage: not a start.
        assert!(r.staged_by.is_empty());
        assert_eq!(r.scripts, 2);
        assert!(r.missing.is_empty());
        assert_eq!(r.missing_conditions, ["GetDisposition"]);
        assert_eq!(r.player_moves, BTreeSet::from(["TestCell".to_string()]));
        assert!(r.not_running.is_empty());

        let r = row("TestStaged");
        assert!(r.started_by.is_empty());
        assert_eq!(r.staged_by, ["TestStarted dialogue"]);
        assert_eq!(r.not_running, ["TestStarted dialogue"]);
        assert_eq!(r.played, PlayStatus::NotPlayed);

        let r = row("TestPerkQuest");
        assert_eq!(r.started_by, ["perk TestPerk"]);
        assert_eq!(r.played, PlayStatus::NotPlayed);

        let r = row("TestOrphan");
        assert_eq!(r.played, PlayStatus::NeverStarted);
        assert!(
            r.note.contains("Nothing in the data starts it"),
            "{}",
            r.note
        );

        // In the played table.
        let r = row("VCG02");
        assert_eq!(r.played, PlayStatus::Partial);
        assert!(!r.note.is_empty() && !r.source.is_empty());

        let mut out = Vec::new();
        assert!(write_page(&mut out, &rows).is_ok());
        let page = String::from_utf8(out).unwrap();
        assert!(
            page.contains("**6 quests**: 0 PLAYED, 1 PARTIAL, 4 NOT PLAYED, 1 NEVER STARTED."),
            "{page}"
        );
        assert!(
            page.contains("| `StartCannibal` | 1 | 0 | `TestStarted` |"),
            "{page}"
        );
        assert!(
            page.contains("| `GetDisposition` | 0 | 1 | `TestQuest` |"),
            "{page}"
        );
        assert!(
            page.contains("started by `TestLeverScript (on ACTI TestLever)`"),
            "{page}"
        );
        assert!(page.contains("`GetDisposition` (condition)"), "{page}");
        // Within a status, the quests with something listed against them
        // come first.
        let not_played = page.split("### NOT PLAYED").nth(1).unwrap();
        let at = |id: &str| not_played.find(&format!("| `{id}` |")).unwrap();
        assert!(at("TestQuest") < at("TestPerkQuest"));
        assert!(at("TestStarted") < at("TestPerkQuest"));
    }

    #[test]
    fn names_the_flags() {
        assert_eq!(flag_words(0x00), "");
        assert_eq!(flag_words(0x01), "start game enabled");
        assert_eq!(
            flag_words(0x1D),
            "start game enabled, repeated topics, repeated stages, 0x10"
        );
    }

    #[test]
    fn ranks_blockers_by_how_many_quests_they_hold_back() {
        let row = |id: &str, missing: &[&str]| QuestRow {
            form_id: FormId(1),
            editor_id: id.to_string(),
            flags: 0,
            stages: 0,
            objectives: 0,
            started_by: Vec::new(),
            staged_by: Vec::new(),
            scripts: 0,
            not_running: Vec::new(),
            functions: BTreeSet::new(),
            missing: missing.iter().map(|s| s.to_string()).collect(),
            missing_conditions: Vec::new(),
            systems: Vec::new(),
            other_codes: BTreeSet::new(),
            player_moves: BTreeSet::new(),
            played: PlayStatus::NotPlayed,
            note: String::new(),
            source: String::new(),
        };
        let rows = [row("A", &["B", "Z"]), row("C", &["Z"]), row("D", &[])];
        let b = blockers(&rows);
        let order: Vec<(&str, usize)> = b.iter().map(|(k, q)| (k.as_str(), q.len())).collect();
        assert_eq!(order, [("Z", 2), ("B", 1)]);
    }

    #[test]
    fn a_hand_kept_row_the_data_lacks_stops_the_command() {
        let order = game();
        let scripts = scripts(&order);
        let rows = quest_rows(&order, &scripts);
        // The synthetic plugin has the one played row it needs and no other.
        assert!(rows.iter().any(|r| r.editor_id == "VCG02"));
        let message = match check_ledger(&rows) {
            Err(CliError::Usage(m)) => m,
            Err(_) => panic!("the ledger check reports a usage problem"),
            Ok(()) => panic!("a hand-kept row the data lacks should stop the command"),
        };
        assert!(message.contains("VCG00"), "{message}");
        assert!(!message.contains("VCG02"), "{message}");
        // With every row's quest present the page is written.
        let complete: Vec<QuestRow> = quest_played::ENTRIES
            .iter()
            .map(|e| {
                let mut r = rows[0].clone();
                r.editor_id = e.quest.to_string();
                r
            })
            .collect();
        assert!(check_ledger(&complete).is_ok());
    }

    #[test]
    fn the_page_is_the_same_every_time() {
        let order = game();
        let scripts = scripts(&order);
        let rows = quest_rows(&order, &scripts);
        let (mut once, mut twice) = (Vec::new(), Vec::new());
        assert!(write_page(&mut once, &rows).is_ok());
        assert!(write_page(&mut twice, &rows).is_ok());
        assert_eq!(once, twice);
        assert_eq!(
            rows.iter()
                .map(|r| r.editor_id.as_str())
                .collect::<Vec<_>>(),
            [
                "TestOrphan",
                "TestPerkQuest",
                "TestQuest",
                "TestStaged",
                "TestStarted",
                "VCG02"
            ]
        );
    }
}
