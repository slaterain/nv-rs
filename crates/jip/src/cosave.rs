//! JIP LN NVSE's co-save section (`internal/serialization.h` (JIP LN NVSE), plan step 3.2): its
//! eight records, read after the `.fos` (`LoadGameCallback`) and written on every save
//! (`SaveGameCallback`) through xNVSE's co-save (`nvse::plugins`), and the messages that decide
//! what a load reads again (`NVSEMessageHandler`'s save and load cases).
//!
//! | Record | Holds |
//! | --- | --- |
//! | `JPSV` | variables JIP added to scripts (`AddScriptVariable`), by owner |
//! | `JPAV` | aux variables, by mod, owner and name |
//! | `JPRM` | ref maps, by mod, name and reference |
//! | `JPED` | `ExtraJIP` data (`SetRefExtraData`), by key |
//! | `JPLR` | linked references JIP changed |
//! | `JPGF` | its saved flags and armour damage cap |
//! | `JPUA` | the player's appearance before a change, to undo it |
//! | `JPNP` | perks given to NPCs (only with JIP's NPC perks patch) |
//!
//! Each record is written by walking JIP's own maps ([`crate::containers`]), so a save made after a
//! load lists them as JIP's would. A load reads a form's id through the game's own table of the
//! saved files (`GetResolvedRefID`) and drops what belongs to a file no longer loaded or a form
//! that is gone, as JIP does. Loading the save last loaded or saved again reads the aux
//! variables, ref maps, extra data, linked references and NPC perks only where a command changed
//! them since (`s_dataChangedFlags`); the commands come later (step 3.6).
//!
//! What a load does to the game besides keeping the data comes with JIP's saved data (step 3.3):
//! a script variable is kept but not yet made in its script and event list (`Script::AddVariable`,
//! which also refuses a name the script already has unless JIP added it this session: until then
//! such a variable is kept and written back where JIP would leave it out),
//! a linked reference is kept but not set on its reference (`SetLinkedRefID`), NPC perks are not
//! given to the actors, and the appearance undo is not applied. Until then a save holds what the
//! co-save loaded held. What JIP would pass over unread (another record type, a version it does
//! not read, the NPC perks without its NPC perks patch, `JPGF`'s bytes past its 8) is carried byte
//! for byte to the next save, so a co-save from a real JIP installation loses nothing in nv-rs.
//! While JIP answers "not installed" and holds nothing it writes no section, as without the DLL.
//!
//! JohnnyGuitar NVSE corrects JIP's extra data record (`JIPFixes.cpp`, `SerializationFixes`):
//! JIP counts 5 bytes for each mod entry's header where it writes 9, so its version 1 records hold
//! wrong sizes that a skip follows into the next entry; JohnnyGuitar writes version 2 with the
//! right sizes, reads no version 1 record, and skips an unloaded owner's script variables by 10
//! bytes each where JIP skips 9. That is a fixes row ([`set_johnny_guitar_fixes`], on in the fixes
//! preset): VNV installs JohnnyGuitar, so its players' co-saves are version 2.

use crate::containers::{align_bucket_count, Name, Uint, UnorderedMap};
use crate::game::{
    self, BGSPerk, BGSSaveLoadGame, ExtraLinkedRef, ExtraScript, NiTMapItem, NiTPointerMap,
    TESForm, TESObjectREFR, TESQuest,
};
use crate::Jip;
use engine_core::prelude::*;
use nvse::cosave::{Chunk, OutOfRange, Reader};
use nvse::plugins::{msg, Message, Records};
use std::collections::BTreeMap;

/// The record types (`JIPSerializationTags`: `'VSPJ'` and so on, the bytes `JPSV` in the file).
pub mod kind {
    use nvse::cosave::tag;
    pub const SCRIPT_VARS: u32 = tag(b"VSPJ");
    pub const AUX_VARS: u32 = tag(b"VAPJ");
    pub const REF_MAPS: u32 = tag(b"MRPJ");
    pub const EXTRA_DATA: u32 = tag(b"DEPJ");
    pub const LINKED_REFS: u32 = tag(b"RLPJ");
    pub const SERIALIZED_VARS: u32 = tag(b"FGPJ");
    pub const APPEARANCE_UNDO: u32 = tag(b"UAPJ");
    pub const NPC_PERKS: u32 = tag(b"PNPJ");
}

/// `JIP_VARS_VERSION`: the aux variables' and ref maps' records, written and the least read.
pub const VARS_VERSION: u32 = 10;
/// `ExtraJIP::kExtraJIP_Verion`, and JohnnyGuitar's (`uiJIPExtraDataVersion`).
pub const EXTRA_DATA_VERSION: u32 = 1;
pub const EXTRA_DATA_VERSION_JG: u32 = 2;
/// The versions the other records are written with (`WriteRecord(..., 9, ...)` and so on).
const SCRIPT_VARS_VERSION: u32 = 9;
const LINKED_REFS_VERSION: u32 = 9;
const SERIALIZED_VARS_VERSION: u32 = 1;
const APPEARANCE_UNDO_VERSION: u32 = 9;
const NPC_PERKS_VERSION: u32 = 10;

/// What has changed since the save last loaded or saved (`kChangedFlag_*`).
pub mod changed {
    pub const AUX_VARS: u8 = 1;
    pub const REF_MAPS: u8 = 2;
    pub const EXTRA_DATA: u8 = 4;
    pub const LINKED_REFS: u8 = 8;
    pub const NPC_PERKS: u8 = 0x10;
    pub const ALL: u8 = 0x1f;
}

/// An aux variable's or ref map's value (`AuxVariableValue`): type 1 a number, type 2 a form's
/// id, any other a text (JIP's own type for it is 4), its type kept.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Form(u32),
    Text { kind: u8, bytes: Vec<u8> },
}

impl Value {
    /// Its type byte.
    pub fn kind(&self) -> u8 {
        match self {
            Value::Number(_) => 1,
            Value::Form(_) => 2,
            Value::Text { kind, .. } => *kind,
        }
    }
}

/// A script variable JIP added (`ScriptVariableEntry`): the mod that added it and its value
/// (the 8 bytes of the variable's double, as the co-save holds them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptVar {
    pub mod_index: u8,
    pub value: u64,
}

/// One mod's `ExtraJIP` data on a reference (`ExtraJIPData`): the bit of each used index, the bit
/// of each that holds a form's id, the 18 values, and two texts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtraJipData {
    pub used: u32,
    pub forms: u32,
    pub values: [u32; 18],
    pub strings: [Vec<u8>; 2],
}

/// The bits of [`ExtraJipData::used`] and `forms` that name one of its 18 values.
const EXTRA_VALUES_MASK: u32 = (1 << 18) - 1;

impl ExtraJipData {
    /// The used indexes' count: past the highest used bit (`GetMaxIndex`).
    pub fn max_index(&self) -> u32 {
        // At most the 18 values (the load clears the bits past them).
        (32 - self.used.leading_zeros()).min(self.values.len() as u32)
    }

    /// The bytes written for it before its texts (`GetSaveSize`): the two bit words and the
    /// values up to the highest used.
    pub fn save_size(&self) -> u32 {
        (self.max_index() + 2) << 2
    }

    /// The first `size` bytes of its fields: the bit words, then the values.
    fn head_bytes(&self, size: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(0x50);
        out.extend_from_slice(&self.used.to_le_bytes());
        out.extend_from_slice(&self.forms.to_le_bytes());
        for v in self.values {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.truncate(size);
        out
    }

    /// `COPY_BYTES(pData, buffer, size)`: the first `size` bytes of its fields from `bytes`.
    fn copy_head(&mut self, bytes: &[u8]) {
        let mut head = self.head_bytes(0x50);
        head[..bytes.len()].copy_from_slice(bytes);
        let word = |i: usize| u32::from_le_bytes(head[4 * i..4 * i + 4].try_into().unwrap());
        self.used = word(0);
        self.forms = word(1);
        for (i, v) in self.values.iter_mut().enumerate() {
            *v = word(i + 2);
        }
    }
}

/// The `ExtraJIP` data of one key (`ExtraJIPEntry`): its reference and each mod's data, by mod
/// index from the lowest (JIP's sorted `Map`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtraJipEntry {
    pub ref_id: u32,
    pub data: BTreeMap<u32, ExtraJipData>,
}

/// A linked reference JIP changed (`LinkedRefEntry`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkedRef {
    pub link_id: u32,
    pub mod_index: u8,
}

/// `JIPSerializedVars`: flags (bit 0 `kSerializedFlag_NoHardcoreTracking`) and the armour
/// damage cap (`SetDamageToArmorMaxPercent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerializedVars {
    pub flags: u32,
    /// The cap's `float`, as its bits.
    pub dmg_armor_max_percent: u32,
}

impl Default for SerializedVars {
    /// `Reset`: no flags, a cap of 1.0.
    fn default() -> Self {
        SerializedVars {
            flags: 0,
            dmg_armor_max_percent: 1.0f32.to_bits(),
        }
    }
}

/// The player's appearance kept to undo a change (`AppearanceUndo`): the face values, flags,
/// hair length and colour as their 0x214 bytes, and the race, hair, eyes and head parts (form
/// addresses).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppearanceUndo {
    pub head: Vec<u8>,
    pub race: u32,
    pub hair: u32,
    pub eyes: u32,
    pub head_parts: Vec<u32>,
}

/// Perks given to an NPC (`NPCPerksInfo::perkRanks`): rank by perk, the perks by address from the
/// lowest (JIP's sorted `Map` of `BGSPerk*`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NpcPerks {
    pub ranks: BTreeMap<u32, u8>,
}

/// JIP's saved data and what decides its loads.
#[derive(Clone, Debug)]
pub struct Saved {
    /// `s_lastLoadedPath`: the save last loaded or saved.
    pub last_path: Vec<u8>,
    /// `s_dataChangedFlags`.
    pub changed: u8,
    /// JohnnyGuitar's corrections of the extra data record and the variable skip (the row).
    pub jg_fixes: bool,
    /// JIP's NPC perks patch is installed (`s_patchInstallState.NPCPerks`, the `uNPCPerks`
    /// switch): until it is built in, `JPNP` is neither read nor written.
    pub npc_perks_patch: bool,
    /// `s_scriptVariablesBuffer`: owner id, name.
    pub script_vars: UnorderedMap<u32, UnorderedMap<Name, ScriptVar>>,
    /// `s_auxVariables[0]` (the saved ones): mod index, owner id, name, values.
    pub aux_vars: UnorderedMap<u32, UnorderedMap<u32, UnorderedMap<Name, Vec<Value>>>>,
    /// `s_refMapArrays[0]`: mod index, name, reference id.
    pub ref_maps: UnorderedMap<u32, UnorderedMap<Name, UnorderedMap<u32, Value>>>,
    /// `s_extraDataKeysMap`.
    pub extra_data: UnorderedMap<Uint, ExtraJipEntry>,
    /// `s_linkedRefModified` and `s_linkedRefsTemp`.
    pub linked_refs: UnorderedMap<u32, LinkedRef>,
    pub linked_refs_temp: UnorderedMap<u32, u32>,
    /// `s_serializedVars`.
    pub serialized: SerializedVars,
    /// `s_appearanceUndoMap`, by NPC (form address).
    pub appearance_undo: UnorderedMap<u32, AppearanceUndo>,
    /// `s_NPCPerksInfoMap`, by actor id.
    pub npc_perks: UnorderedMap<u32, NpcPerks>,
    /// nv-rs's own: the records of the last co-save loaded that JIP would pass over without
    /// reading, in their order, written back at the next save ([`load`]).
    pub carried: Vec<Chunk>,
    /// nv-rs's own: the bytes of the last `JPGF` past the 8 JIP reads, written back after them.
    pub serialized_tail: Vec<u8>,
    /// nv-rs's own: the last co-save loaded had a section of JIP's (xNVSE writes one only for a
    /// plugin that writes a record).
    pub had_section: bool,
}

impl Default for Saved {
    fn default() -> Self {
        Saved {
            last_path: Vec::new(),
            changed: 0,
            jg_fixes: false,
            npc_perks_patch: false,
            script_vars: UnorderedMap::new(8),
            aux_vars: UnorderedMap::new(8),
            ref_maps: UnorderedMap::new(8),
            extra_data: UnorderedMap::new(0x20),
            linked_refs: UnorderedMap::new(8),
            linked_refs_temp: UnorderedMap::new(8),
            serialized: SerializedVars::default(),
            appearance_undo: UnorderedMap::new(8),
            npc_perks: UnorderedMap::new(0x80),
            carried: Vec::new(),
            serialized_tail: Vec::new(),
            had_section: false,
        }
    }
}

fn saved(e: &mut Engine) -> &mut Saved {
    &mut e.ext::<Jip>().saved
}

/// The fixes preset's row: JohnnyGuitar NVSE's corrections of JIP's co-save (above).
pub fn set_johnny_guitar_fixes(e: &mut Engine, on: bool) {
    saved(e).jg_fixes = on;
}

// ---- the game's tables JIP reads (internal/jip_core.cpp) ----

/// `g_BGSSaveLoadGame` (`BGSSaveLoadGame::GetSingleton`).
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
/// The game's map of every form by id (`NiTPointerMap`, its pointer at `0x011C54C0`).
const ALL_FORMS: u32 = 0x011c_54c0;
/// `PlayerCharacter` (`0x011DEA3C`).
const PLAYER: u32 = 0x011d_ea3c;
/// A vtable's entries that answer true (`0x008D0360`) for a reference (`+0xF0`) and an actor
/// (`+0x100`): JIP's `IS_REFERENCE` and `IS_ACTOR`.
const RETURN_TRUE: u32 = 0x008d_0360;
const VT_IS_REFERENCE: u32 = 0xf0;
const VT_IS_ACTOR: u32 = 0x100;
const QUEST: u8 = 0x47;
const PERK: u8 = 0x56;
/// `BaseExtraList::GetByType` (`thiscall`) and the extra data types `ExtraScript` (`0x0D`) and
/// `ExtraLinkedRef` (`0x51`).
const EXTRA_GET_BY_TYPE: u32 = 0x0041_0220;
const EXTRA_SCRIPT: u32 = 0x0d;
const EXTRA_LINKED_REF: u32 = 0x51;
/// Default appearance forms (the `JPUA` load): race, female and male hair, eyes.
const DEFAULT_RACE: u32 = 0x19;
const DEFAULT_HAIR_FEMALE: u32 = 0x0002_2e4e;
const DEFAULT_HAIR_MALE: u32 = 0x0001_4b90;
const DEFAULT_EYES: u32 = 0x4255;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetResolvedModIndex.
/// A saved mod index as loaded today: 0xFF stays; one no longer loaded is `None`.
pub fn resolved_mod_index(e: &Engine, index: u8) -> Option<u8> {
    if index == 0xff {
        return Some(0xff);
    }
    let now = e.mem.u8(e.mem.u32(SAVE_LOAD_GAME) + game::save_mod(index));
    (now != 0xff).then_some(now)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetResolvedRefID.
/// A saved form id as loaded today: 0 and an id of mod 0xFF stay; one of a file no longer loaded
/// is 0.
pub fn resolved_ref_id(e: &Engine, id: u32) -> u32 {
    if id == 0 || id >> 24 == 0xff {
        return id;
    }
    match resolved_mod_index(e, (id >> 24) as u8) {
        Some(now) => (u32::from(now) << 24) | (id & 0x00ff_ffff),
        None => 0,
    }
}

/// A walk of one of the game's `NiTMap`s of ids: the entry of `id`, 0 for none.
fn map_entry(e: &Engine, map: u32, id: u32) -> u32 {
    let map = Ptr::<NiTPointerMap>::new(map);
    let count = e.get(map, NiTPointerMap::m_uiHashSize);
    if count == 0 {
        return 0;
    }
    let buckets = e.get(map, NiTPointerMap::m_ppkHashTable);
    let mut entry = e.mem.u32(buckets + 4 * (id % count));
    while entry != 0 && e.get(Ptr::<NiTMapItem>::new(entry), NiTMapItem::m_key) != id {
        entry = e.get(Ptr::<NiTMapItem>::new(entry), NiTMapItem::m_pkNext);
    }
    entry
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: LookupFormByRefID.
/// The form of an id, 0 for none.
pub fn lookup_form(e: &Engine, id: u32) -> u32 {
    match map_entry(e, e.mem.u32(ALL_FORMS), id) {
        0 => 0,
        entry => e.get(Ptr::<NiTMapItem>::new(entry), NiTMapItem::m_val),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: HasChangeData.
/// Whether the save holds changes of the form `id` (the load's changes map).
pub fn has_change_data(e: &Engine, id: u32) -> bool {
    let sl = Ptr::<BGSSaveLoadGame>::new(e.mem.u32(SAVE_LOAD_GAME));
    map_entry(e, e.get(sl, BGSSaveLoadGame::pChangesMap), id) != 0
}

fn vtable_says(e: &Engine, form: u32, slot: u32) -> bool {
    e.mem.u32(e.mem.u32(form) + slot) == RETURN_TRUE
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp:
// TESForm::GetScriptAndEventList.
/// A reference's script and event list (its `ExtraScript`), or a quest's; both there or `None`.
fn script_and_event_list(e: &mut Engine, form: u32) -> Option<(u32, u32)> {
    let (script, events) = if vtable_says(e, form, VT_IS_REFERENCE) {
        let extra = e
            .call(EXTRA_GET_BY_TYPE, &args![extra_list(form), EXTRA_SCRIPT])
            .u32();
        if extra == 0 {
            return None;
        }
        let extra = Ptr::<ExtraScript>::new(extra);
        (
            e.get(extra, ExtraScript::pScript),
            e.get(extra, ExtraScript::pScriptVars),
        )
    } else if form_type(e, form) == QUEST {
        let quest = Ptr::<TESQuest>::new(form);
        (
            e.get(quest, TESQuest::pFormScript),
            e.get(quest, TESQuest::pScriptLocals),
        )
    } else {
        return None;
    };
    (script != 0 && events != 0).then_some((script, events))
}

/// The id of a form (0 for none: the original would read through it).
fn form_id(e: &Engine, form: u32) -> u32 {
    if form == 0 {
        0
    } else {
        e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
    }
}

/// A form's type.
fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

/// A reference's extra data list.
fn extra_list(form: u32) -> u32 {
    Ptr::<TESObjectREFR>::new(form)
        .at(TESObjectREFR::ExtraData)
        .addr()
}

/// The player's base form (`g_thePlayer->baseForm`).
fn player_base(e: &Engine) -> u32 {
    let player = Ptr::<TESObjectREFR>::new(e.mem.u32(PLAYER));
    e.get(player, TESObjectREFR::pObjectReference)
}

// ---- reading a record ----

/// A record's bytes as JIP reads them (`ReadRecordToBuffer` and a moving pointer). JIP reads past
/// a record's end into whatever its buffer held before; here a read past the end ends the record
/// (only a damaged co-save reaches it).
struct Bytes<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }
    fn u16(&mut self) -> Option<u16> {
        self.take(2)
            .map(|s| u16::from_le_bytes(s.try_into().unwrap()))
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
    }
    fn u64(&mut self) -> Option<u64> {
        self.take(8)
            .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
    }
    fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }
    /// The bytes of a name of `len` bytes, up to a 0 in them (JIP reads it as a C string).
    fn name(&mut self, len: usize) -> Option<Vec<u8>> {
        let s = self.take(len)?;
        let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
        Some(s[..end].to_vec())
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: AuxVariableValue::ReadValData.
/// A value of type `kind` (a form's id resolved, 0 when its file is gone).
fn read_value(e: &Engine, b: &mut Bytes, kind: u8) -> Option<Value> {
    Some(match kind {
        1 => Value::Number(f64::from_bits(b.u64()?)),
        2 => Value::Form(resolved_ref_id(e, b.u32()?)),
        _ => {
            let len = b.u16()?;
            Value::Text {
                kind,
                bytes: b.take(usize::from(len))?.to_vec(),
            }
        }
    })
}

/// Skips a value of type `kind` as JIP's skip loops do.
fn skip_value(b: &mut Bytes, kind: u8) -> Option<()> {
    match kind {
        1 => b.skip(8),
        2 => b.skip(4),
        _ => {
            let len = b.u16()?;
            b.skip(usize::from(len))
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: ProcessDataChangedFlags.
/// Clears what a load will read again: the kinds `flags` names, the script variables, the
/// temporary links and the saved flags always.
pub fn process_changed_flags(e: &mut Engine, flags: u8) {
    let s = saved(e);
    if flags & changed::AUX_VARS != 0 {
        s.aux_vars.clear();
    }
    if flags & changed::REF_MAPS != 0 {
        s.ref_maps.clear();
    }
    if flags & changed::EXTRA_DATA != 0 {
        s.extra_data.clear();
    }
    if flags & changed::LINKED_REFS != 0 {
        s.linked_refs.clear();
    }
    s.script_vars.clear();
    s.linked_refs_temp.clear();
    s.serialized = SerializedVars::default();
    // The NPCs' perk info leaves the actors with it (step 3.3); the map empties.
    if flags & changed::NPC_PERKS != 0 {
        s.npc_perks.clear();
    }
    // nv-rs's own: what the last co-save loaded held that nv-rs carries without reading.
    s.carried.clear();
    s.serialized_tail.clear();
    s.had_section = false;
}

/// The record types JIP reads and writes, in the order it writes them.
const KINDS: [u32; 8] = [
    kind::SCRIPT_VARS,
    kind::AUX_VARS,
    kind::REF_MAPS,
    kind::EXTRA_DATA,
    kind::LINKED_REFS,
    kind::SERIALIZED_VARS,
    kind::APPEARANCE_UNDO,
    kind::NPC_PERKS,
];

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: LoadGameCallback.
/// The load callback: what changed is cleared, then each of JIP's records is read. What JIP
/// would pass over without reading (a record of another type, a version it does not read, the
/// NPC perks without the NPC perks patch, the saved flags' bytes past their 8) is carried byte for
/// byte to the next save instead ([`Saved::carried`]), so a co-save of a real JIP installation
/// loses nothing in nv-rs before the parts that use it are built. A record not read because the
/// same save is loaded again with nothing changed keeps the data loaded before, as in JIP.
pub fn load(e: &mut Engine, r: &mut Reader) -> Result<(), OutOfRange> {
    let flags = saved(e).changed;
    process_changed_flags(e, flags);
    // nv-rs's own: a section cut short (a record running past the file's end) ends JIP's read
    // where it breaks, with what was read kept; xNVSE then goes on at the section's end, so the
    // rest of the co-save still loads.
    let _ = read_records(e, r, flags);
    Ok(())
}

/// [`load`]'s records.
fn read_records(e: &mut Engine, r: &mut Reader, flags: u8) -> Result<(), OutOfRange> {
    while let Some((kind, version, length)) = r.next_record()? {
        saved(e).had_section = true;
        let (jg, perks_patch) = {
            let s = saved(e);
            (s.jg_fixes, s.npc_perks_patch)
        };
        let unread = match kind {
            kind::AUX_VARS | kind::REF_MAPS => version < VARS_VERSION,
            kind::EXTRA_DATA => {
                let newest = if jg {
                    EXTRA_DATA_VERSION_JG
                } else {
                    EXTRA_DATA_VERSION
                };
                // JohnnyGuitar gives up on JIP's own version: its sizes are wrong.
                version > newest || (jg && version == 1)
            }
            kind::NPC_PERKS => !perks_patch,
            k => !KINDS.contains(&k),
        };
        if unread {
            let data = r.read_data(length)?;
            saved(e).carried.push(Chunk {
                kind,
                version,
                data,
            });
            continue;
        }
        let gated = match kind {
            kind::AUX_VARS => flags & changed::AUX_VARS == 0,
            kind::REF_MAPS => flags & changed::REF_MAPS == 0,
            kind::EXTRA_DATA => flags & changed::EXTRA_DATA == 0,
            kind::LINKED_REFS => flags & changed::LINKED_REFS == 0,
            kind::NPC_PERKS => flags & changed::NPC_PERKS == 0,
            _ => false,
        };
        if gated {
            continue;
        }
        if kind == kind::SERIALIZED_VARS {
            // `ReadRecordData(&s_serializedVars, length)`: the bytes read over the values the
            // load's start reset (a shorter record leaves the rest as they were); bytes past
            // the 8 JIP reads (its own read would overrun) are carried.
            let bytes = r.read_data(length)?;
            let s = saved(e);
            let mut raw = [0u8; 8];
            raw[..4].copy_from_slice(&s.serialized.flags.to_le_bytes());
            raw[4..].copy_from_slice(&s.serialized.dmg_armor_max_percent.to_le_bytes());
            let n = bytes.len().min(8);
            raw[..n].copy_from_slice(&bytes[..n]);
            s.serialized.flags = u32::from_le_bytes(raw[..4].try_into().unwrap());
            s.serialized.dmg_armor_max_percent = u32::from_le_bytes(raw[4..].try_into().unwrap());
            s.serialized_tail = bytes[n..].to_vec();
            continue;
        }
        let data = r.read_data(length)?;
        let mut b = Bytes { b: &data, at: 0 };
        // A record that runs out before its counts say ends there (damaged data only).
        let _ = match kind {
            kind::SCRIPT_VARS => load_script_vars(e, &mut b, jg),
            kind::AUX_VARS => load_aux_vars(e, &mut b),
            kind::REF_MAPS => load_ref_maps(e, &mut b),
            kind::EXTRA_DATA => load_extra_data(e, &mut b),
            kind::LINKED_REFS => load_linked_refs(e, &mut b),
            kind::APPEARANCE_UNDO => load_appearance_undo(e, &mut b),
            kind::NPC_PERKS => load_npc_perks(e, &mut b),
            _ => Some(()),
        };
    }
    Ok(())
}

/// `JPSV`: each owner's variables, kept for an owner that is loaded with a script and an event
/// list (the variables made in them with step 3.3), a value that holds a form's id resolved.
fn load_script_vars(e: &mut Engine, b: &mut Bytes, jg: bool) -> Option<()> {
    let mut owners = b.u16()?;
    while owners != 0 {
        owners -= 1;
        let id = resolved_ref_id(e, b.u32()?);
        let mut vars = b.u16()?;
        let form = if id != 0 { lookup_form(e, id) } else { 0 };
        if form != 0 && script_and_event_list(e, form).is_some() {
            while vars != 0 {
                vars -= 1;
                let mod_index = b.u8()?;
                let len = usize::from(b.u8()?);
                let name = b.name(len)?;
                let mut value = b.u64()?;
                let Some(mod_index) = resolved_mod_index(e, mod_index) else {
                    continue;
                };
                // A form's id in the low word (the high one 0) is resolved.
                if value as u32 != 0 && value >> 32 == 0 {
                    value = u64::from(resolved_ref_id(e, value as u32));
                }
                // `Script::AddVariable` keeps every variable but a temporary one.
                if name.first() != Some(&b'*') {
                    // `s_scriptVariablesBuffer()[ownerID]`: a map of 4 buckets (`ScriptVariablesMap`).
                    let owner = saved(e)
                        .script_vars
                        .emplace_with(id, || UnorderedMap::new(4))
                        .0;
                    *owner.emplace(Name(name)) = ScriptVar { mod_index, value };
                }
            }
        } else {
            while vars != 0 {
                vars -= 1;
                // JIP skips one byte short of a variable (9 past its name); JohnnyGuitar 10.
                let _mod_index = b.u8()?;
                let len = usize::from(b.u8()?);
                b.skip(len + if jg { 8 } else { 7 })?;
            }
        }
    }
    Some(())
}

/// Whether a resolved owner or reference is there: loaded, or with changes in the save.
fn present(e: &Engine, id: u32) -> bool {
    id != 0 && (lookup_form(e, id) != 0 || has_change_data(e, id))
}

/// `JPAV`: by mod (from 6 up, as loaded today), owner (loaded or changed in the save) and name;
/// an empty name ends its owner's variables. A mod's map is made at its first owner kept, with
/// room for the owners left (`AlignBucketCount`), an owner's map with room for its variables.
fn load_aux_vars(e: &mut Engine, b: &mut Bytes) -> Option<()> {
    let mut mods = b.u16()?;
    while mods != 0 {
        mods -= 1;
        let saved_index = b.u8()?;
        let mut owners = b.u16()?;
        let mod_index = (saved_index > 5)
            .then(|| resolved_mod_index(e, saved_index))
            .flatten();
        let Some(m) = mod_index.map(u32::from) else {
            for _ in 0..owners {
                b.skip(4)?;
                let vars = b.u16()?;
                skip_aux_vars(b, vars)?;
            }
            continue;
        };
        while owners != 0 {
            let id = resolved_ref_id(e, b.u32()?);
            let mut vars = b.u16()?;
            if present(e, id) {
                let (left, room) = (owners, vars);
                saved(e)
                    .aux_vars
                    .emplace_with(m, || UnorderedMap::new(align_bucket_count(u32::from(left))))
                    .0
                    .emplace_with(id, || {
                        UnorderedMap::new(align_bucket_count(u32::from(room)))
                    });
                while vars != 0 {
                    let len = b.u8()?;
                    if len == 0 {
                        // `goto avSkipVars`: this variable's values and the owner's rest go.
                        skip_aux_values(b)?;
                        skip_aux_vars(b, vars - 1)?;
                        break;
                    }
                    let name = b.name(usize::from(len))?;
                    let count = b.u16()?;
                    let mut values = Vec::with_capacity(usize::from(count));
                    for _ in 0..count {
                        let kind = b.u8()?;
                        values.push(read_value(e, b, kind)?);
                    }
                    let owner = saved(e).aux_vars.get_mut(&m)?.get_mut(&id)?;
                    owner
                        .emplace_with(Name(name), || Vec::with_capacity(usize::from(count)))
                        .0
                        .extend(values);
                    vars -= 1;
                }
            } else {
                skip_aux_vars(b, vars)?;
            }
            owners -= 1;
        }
    }
    Some(())
}

/// One aux variable's values, skipped.
fn skip_aux_values(b: &mut Bytes) -> Option<()> {
    let count = b.u16()?;
    for _ in 0..count {
        let kind = b.u8()?;
        skip_value(b, kind)?;
    }
    Some(())
}

/// `vars` aux variables (name and values), skipped.
fn skip_aux_vars(b: &mut Bytes, vars: u16) -> Option<()> {
    for _ in 0..vars {
        let len = b.u8()?;
        b.skip(usize::from(len))?;
        skip_aux_values(b)?;
    }
    Some(())
}

/// `JPRM`: by mod (from 6 up, as loaded today), name and reference (loaded or changed in the
/// save); an empty name ends the mod's names.
fn load_ref_maps(e: &mut Engine, b: &mut Bytes) -> Option<()> {
    let mut mods = b.u16()?;
    while mods != 0 {
        mods -= 1;
        let saved_index = b.u8()?;
        let mut names = b.u16()?;
        let mod_index = (saved_index > 5)
            .then(|| resolved_mod_index(e, saved_index))
            .flatten();
        let Some(m) = mod_index.map(u32::from) else {
            skip_ref_map_names(b, names)?;
            continue;
        };
        while names != 0 {
            let len = b.u8()?;
            if len == 0 {
                // `goto rmSkipVars`: this name's references and the mod's rest go.
                skip_ref_map_refs(b)?;
                names -= 1;
                skip_ref_map_names(b, names)?;
                break;
            }
            let name = b.name(usize::from(len))?;
            let mut refs = b.u16()?;
            while refs != 0 {
                let id = resolved_ref_id(e, b.u32()?);
                let kind = b.u8()?;
                if present(e, id) {
                    // The mod's and the name's maps are made at the first reference kept, with
                    // room for the names and references left.
                    let (names_left, refs_left) = (names, refs);
                    let ids = saved(e)
                        .ref_maps
                        .emplace_with(m, || {
                            UnorderedMap::new(align_bucket_count(u32::from(names_left)))
                        })
                        .0
                        .emplace_with(Name(name.clone()), || {
                            UnorderedMap::new(align_bucket_count(u32::from(refs_left)))
                        })
                        .0;
                    // `Emplace(refID, type)`: a value already there keeps its type, and the
                    // read goes by it.
                    let kind = ids.get(&id).map_or(kind, Value::kind);
                    let value = read_value(e, b, kind)?;
                    let ids = saved(e)
                        .ref_maps
                        .get_mut(&m)?
                        .get_mut(&Name(name.clone()))?;
                    *ids.emplace_with(id, || Value::Number(0.0)).0 = value;
                } else {
                    skip_value(b, kind)?;
                }
                refs -= 1;
            }
            names -= 1;
        }
    }
    Some(())
}

/// One ref map's references, skipped.
fn skip_ref_map_refs(b: &mut Bytes) -> Option<()> {
    let refs = b.u16()?;
    for _ in 0..refs {
        b.skip(4)?;
        let kind = b.u8()?;
        skip_value(b, kind)?;
    }
    Some(())
}

/// `names` ref map names with their references, skipped.
fn skip_ref_map_names(b: &mut Bytes, names: u16) -> Option<()> {
    for _ in 0..names {
        let len = b.u8()?;
        b.skip(usize::from(len))?;
        skip_ref_map_refs(b)?;
    }
    Some(())
}

/// `JPED`: by key, for a reference with changes in the save, each loaded mod's data (its form
/// ids resolved, an unresolved one's index unused) and texts.
fn load_extra_data(e: &mut Engine, b: &mut Bytes) -> Option<()> {
    let mut keys = b.u32()?;
    saved(e).extra_data.set_bucket_count(keys);
    while keys != 0 {
        keys -= 1;
        let key = b.u32()?;
        let id = resolved_ref_id(e, b.u32()?);
        let mut mods = b.u8()?;
        let key_size = b.u32()?;
        if id != 0 && has_change_data(e, id) {
            while mods != 0 {
                mods -= 1;
                let saved_index = b.u8()?;
                let size = b.u32()? as usize;
                let len1 = usize::from(b.u16()?);
                let len2 = usize::from(b.u16()?);
                let head = b.take(size)?;
                let string1 = b.take(len1)?;
                let string2 = b.take(len2)?;
                // More than the fields hold would overrun JIP's copy (damaged data): left.
                if size > 0x50 {
                    continue;
                }
                let Some(m) = resolved_mod_index(e, saved_index).map(u32::from) else {
                    continue;
                };
                // The key's entry is made at its first mod kept (`Emplace(key, refID)`), the
                // mod's data at its index (`dataMap[modIdx]`), and the bytes copied over it.
                let mut data = saved(e)
                    .extra_data
                    .emplace_with(Uint(key), || ExtraJipEntry {
                        ref_id: id,
                        data: BTreeMap::new(),
                    })
                    .0
                    .data
                    .get(&m)
                    .cloned()
                    .unwrap_or_default();
                data.copy_head(head);
                // nv-rs's own guard: only 18 values exist; bits for indexes past them (damaged or
                // edited data, which JIP would read into the texts' fields) are cleared, so no
                // read goes past the values and the size written matches the bytes written.
                data.used &= EXTRA_VALUES_MASK;
                data.forms &= EXTRA_VALUES_MASK;
                resolve_extra_ids(e, &mut data);
                if len1 != 0 {
                    data.strings[0] = string1.to_vec();
                }
                if len2 != 0 {
                    data.strings[1] = string2.to_vec();
                }
                saved(e)
                    .extra_data
                    .get_mut(&Uint(key))?
                    .data
                    .insert(m, data);
            }
        } else {
            b.skip(key_size as usize)?;
        }
    }
    Some(())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp: ExtraJIPData::ResolvedRefIDs.
/// The form ids of the used indexes resolved; one whose file is gone leaves its index unused.
fn resolve_extra_ids(e: &Engine, data: &mut ExtraJipData) {
    let max = data.max_index();
    for i in 0..max {
        let mask = 1u32 << i;
        if data.forms & mask != 0 {
            let id = resolved_ref_id(e, data.values[i as usize]);
            data.values[i as usize] = id;
            if id == 0 {
                data.used ^= mask;
            }
        }
    }
    data.forms &= data.used;
}

/// `JPLR`: each reference whose own and linked ids resolve to loaded references, from a loaded
/// mod, is kept, and in the temporary links, as `SetLinkedRefID` keeps it: not a reference whose
/// linked reference extra data holds none (`TESObjectREFR::SetLinkedRef` refuses it). The link
/// itself is set on the reference, and its default kept, with step 3.3.
fn load_linked_refs(e: &mut Engine, b: &mut Bytes) -> Option<()> {
    let count = b.u16()?;
    for _ in 0..count {
        let id = resolved_ref_id(e, b.u32()?);
        let link = resolved_ref_id(e, b.u32()?);
        let saved_index = b.u8()?;
        if id == 0 || link == 0 {
            continue;
        }
        let Some(mod_index) = resolved_mod_index(e, saved_index) else {
            continue;
        };
        let (this, other) = (lookup_form(e, id), lookup_form(e, link));
        let references = this != 0
            && vtable_says(e, this, VT_IS_REFERENCE)
            && other != 0
            && vtable_says(e, other, VT_IS_REFERENCE);
        if !references {
            continue;
        }
        let extra = e
            .call(
                EXTRA_GET_BY_TYPE,
                &args![extra_list(this), EXTRA_LINKED_REF],
            )
            .u32();
        if extra != 0
            && e.get(
                Ptr::<ExtraLinkedRef>::new(extra),
                ExtraLinkedRef::pLinkedRef,
            ) == 0
        {
            continue;
        }
        let link_id = form_id(e, other);
        let s = saved(e);
        *s.linked_refs.emplace(id) = LinkedRef { link_id, mod_index };
        *s.linked_refs_temp.emplace(id) = link;
    }
    Some(())
}

/// The form of a saved id, or the default's when it is gone.
fn form_or(e: &Engine, saved_id: u32, default: u32) -> u32 {
    let id = resolved_ref_id(e, saved_id);
    let form = if id != 0 { lookup_form(e, id) } else { 0 };
    if form != 0 {
        form
    } else {
        lookup_form(e, default)
    }
}

/// `JPUA`: the player's appearance undo, its forms resolved (the race, hair and eyes replaced by
/// the game's defaults when gone, a gone head part left out).
fn load_appearance_undo(e: &mut Engine, b: &mut Bytes) -> Option<()> {
    let head = b.take(0x214)?.to_vec();
    let flags = u32::from_le_bytes(head[0x208..0x20c].try_into().unwrap());
    let race = form_or(e, b.u32()?, DEFAULT_RACE);
    let hair_default = if flags & 1 != 0 {
        DEFAULT_HAIR_FEMALE
    } else {
        DEFAULT_HAIR_MALE
    };
    let hair = form_or(e, b.u32()?, hair_default);
    let eyes = form_or(e, b.u32()?, DEFAULT_EYES);
    let count = b.u8()?;
    let mut head_parts = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let id = resolved_ref_id(e, b.u32()?);
        let form = if id != 0 { lookup_form(e, id) } else { 0 };
        if form != 0 {
            head_parts.push(form);
        }
    }
    let npc = player_base(e);
    *saved(e)
        .appearance_undo
        .emplace_with(npc, || AppearanceUndo {
            head: Vec::new(),
            race: 0,
            hair: 0,
            eyes: 0,
            head_parts: Vec::new(),
        })
        .0 = AppearanceUndo {
        head,
        race,
        hair,
        eyes,
        head_parts,
    };
    Some(())
}

/// `JPNP` (with JIP's NPC perks patch): each loaded actor's perks, a perk's rank at most its
/// count.
fn load_npc_perks(e: &mut Engine, b: &mut Bytes) -> Option<()> {
    let count = b.u16()?;
    saved(e).npc_perks.set_bucket_count(u32::from(count));
    for _ in 0..count {
        let id = resolved_ref_id(e, b.u32()?);
        let perks = b.u8()?;
        let actor = if id != 0 { lookup_form(e, id) } else { 0 };
        if actor == 0 || !vtable_says(e, actor, VT_IS_ACTOR) {
            b.skip(usize::from(perks) * 5)?;
            continue;
        }
        // The actor's entry is made at its first perk kept (`s_NPCPerksInfoMap()[refID]`); the
        // actor itself gets it with step 3.3 (`extraDataList.perksInfo`).
        for _ in 0..perks {
            let perk_id = resolved_ref_id(e, b.u32()?);
            let rank = b.u8()?;
            let perk = if perk_id != 0 {
                lookup_form(e, perk_id)
            } else {
                0
            };
            if perk == 0 || form_type(e, perk) != PERK {
                continue;
            }
            let rank = rank.min(e.get(Ptr::<BGSPerk>::new(perk), BGSPerk::cNumRanks));
            saved(e).npc_perks.emplace(id).ranks.insert(perk, rank);
        }
    }
    Some(())
}

// ---- writing ----

/// An aux variable's or ref map's value as written (`WriteValData`).
fn write_value(r: &mut Records, v: &Value) {
    match v {
        Value::Number(n) => {
            r.u8(1);
            r.u64(n.to_bits());
        }
        Value::Form(id) => {
            r.u8(2);
            r.u32(*id);
        }
        Value::Text { kind, bytes } => {
            r.u8(*kind);
            r.u16(bytes.len() as u16);
            r.data(bytes);
        }
    }
}

/// The save callback: JIP's own records ([`save_own`]), each followed by the carried records of
/// its type, then the carried records of other types, all in the order they were read.
///
/// While JIP answers "not installed" ([`crate::ANSWERS_INSTALLED`]) it writes nothing when it
/// holds nothing (no section loaded, no data a command made, its flags as reset), so the co-save
/// is the one xNVSE writes without JIP: xNVSE writes a plugin's section only when the plugin
/// opens a record (`HandleSaveGame`, `if (s_pluginHeader.numChunks)` (xNVSE)), and JIP writes
/// `JPGF` on every save.
pub fn save(e: &mut Engine, r: &mut Records) {
    let installed = e.ext::<Jip>().answers_installed;
    if !installed && holds_nothing(e) {
        return;
    }
    let mut own = Records::default();
    save_own(e, &mut own);
    // The keep row's rule: carried bytes hold mod indexes of the co-save they came from, so they
    // go back only while today's files are the ones it lists.
    let carry = nvse::saveload::files_as_saved(e);
    let s = &e.ext::<Jip>().saved;
    let carried: &[Chunk] = if carry { &s.carried } else { &[] };
    for k in KINDS {
        for c in own.chunks.iter().chain(carried).filter(|c| c.kind == k) {
            r.record(c.kind, c.version, &c.data);
        }
    }
    for c in carried.iter().filter(|c| !KINDS.contains(&c.kind)) {
        r.record(c.kind, c.version, &c.data);
    }
}

/// Nothing to save: no section loaded and every map empty, the flags as reset.
fn holds_nothing(e: &mut Engine) -> bool {
    let player = player_base(e);
    let s = saved(e);
    !s.had_section
        && s.carried.is_empty()
        && s.serialized_tail.is_empty()
        && s.serialized == SerializedVars::default()
        && s.script_vars.is_empty()
        && s.aux_vars.is_empty()
        && s.ref_maps.is_empty()
        && s.extra_data.is_empty()
        && s.linked_refs.is_empty()
        && s.appearance_undo.get(&player).is_none()
        && (!s.npc_perks_patch || s.npc_perks.is_empty())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: SaveGameCallback.
/// JIP's own records, each written when it holds anything, walking its maps.
fn save_own(e: &mut Engine, r: &mut Records) {
    let player = player_base(e);
    let jg = saved(e).jg_fixes;
    let s = e.ext::<Jip>().saved.clone();
    if !s.script_vars.is_empty() {
        r.record(
            kind::SCRIPT_VARS,
            SCRIPT_VARS_VERSION,
            &(s.script_vars.len() as u16).to_le_bytes(),
        );
        for (owner, vars) in s.script_vars.iter() {
            r.u32(*owner);
            r.u16(vars.len() as u16);
            for (name, var) in vars.iter() {
                r.u8(var.mod_index);
                r.u8(name.0.len() as u8);
                r.data(&name.0);
                r.u64(var.value);
            }
        }
    }
    if !s.aux_vars.is_empty() {
        r.record(
            kind::AUX_VARS,
            VARS_VERSION,
            &(s.aux_vars.len() as u16).to_le_bytes(),
        );
        for (mod_index, owners) in s.aux_vars.iter() {
            r.u8(*mod_index as u8);
            r.u16(owners.len() as u16);
            for (owner, vars) in owners.iter() {
                r.u32(*owner);
                r.u16(vars.len() as u16);
                for (name, values) in vars.iter() {
                    r.u8(name.0.len() as u8);
                    r.data(&name.0);
                    r.u16(values.len() as u16);
                    for v in values {
                        write_value(r, v);
                    }
                }
            }
        }
    }
    if !s.ref_maps.is_empty() {
        r.record(
            kind::REF_MAPS,
            VARS_VERSION,
            &(s.ref_maps.len() as u16).to_le_bytes(),
        );
        for (mod_index, names) in s.ref_maps.iter() {
            r.u8(*mod_index as u8);
            r.u16(names.len() as u16);
            for (name, ids) in names.iter() {
                r.u8(name.0.len() as u8);
                r.data(&name.0);
                r.u16(ids.len() as u16);
                for (id, v) in ids.iter() {
                    r.u32(*id);
                    write_value(r, v);
                }
            }
        }
    }
    if !s.extra_data.is_empty() {
        let (version, per_mod) = if jg {
            (EXTRA_DATA_VERSION_JG, 9)
        } else {
            (EXTRA_DATA_VERSION, 5)
        };
        r.record(kind::EXTRA_DATA, version, &s.extra_data.len().to_le_bytes());
        for (key, entry) in s.extra_data.iter() {
            r.u32(key.0);
            r.u32(entry.ref_id);
            r.u8(entry.data.len() as u8);
            // `ExtraJIPEntry::GetSaveSize`: JIP counts 5 bytes of each mod's header, which is 9.
            let size: u32 = entry.data.len() as u32 * per_mod
                + entry
                    .data
                    .values()
                    .map(|d| d.save_size() + d.strings[0].len() as u32 + d.strings[1].len() as u32)
                    .sum::<u32>();
            r.u32(size);
            for (mod_index, d) in &entry.data {
                r.u8(*mod_index as u8);
                let head = d.save_size();
                r.u32(head);
                r.u16(d.strings[0].len() as u16);
                r.u16(d.strings[1].len() as u16);
                r.data(&d.head_bytes(head as usize));
                r.data(&d.strings[0]);
                r.data(&d.strings[1]);
            }
        }
    }
    if !s.linked_refs.is_empty() {
        r.record(
            kind::LINKED_REFS,
            LINKED_REFS_VERSION,
            &(s.linked_refs.len() as u16).to_le_bytes(),
        );
        for (id, link) in s.linked_refs.iter() {
            r.u32(*id);
            r.u32(link.link_id);
            r.u8(link.mod_index);
        }
    }
    let mut serialized = s.serialized.flags.to_le_bytes().to_vec();
    serialized.extend_from_slice(&s.serialized.dmg_armor_max_percent.to_le_bytes());
    serialized.extend_from_slice(&s.serialized_tail);
    r.record(kind::SERIALIZED_VARS, SERIALIZED_VARS_VERSION, &serialized);
    if let Some(undo) = s.appearance_undo.get(&player) {
        r.record(kind::APPEARANCE_UNDO, APPEARANCE_UNDO_VERSION, &undo.head);
        r.u32(form_id(e, undo.race));
        r.u32(form_id(e, undo.hair));
        r.u32(form_id(e, undo.eyes));
        r.u8(undo.head_parts.len() as u8);
        for part in &undo.head_parts {
            r.u32(form_id(e, *part));
        }
    }
    // `JPNP` holds only what JIP's NPC perks patch gave (step 3.9's switch), which also decides
    // which actors' perks stay at a save (alive, a teammate or loaded, ...).
    if s.npc_perks_patch && !s.npc_perks.is_empty() {
        r.record(
            kind::NPC_PERKS,
            NPC_PERKS_VERSION,
            &(s.npc_perks.len() as u16).to_le_bytes(),
        );
        for (id, info) in s.npc_perks.iter() {
            r.u32(*id);
            r.u8(info.ranks.len() as u8);
            for (perk, rank) in &info.ranks {
                r.u32(form_id(e, *perk));
                r.u8(*rank);
            }
        }
    }
}

// ---- the messages (jip_nvse.cpp: NVSEMessageHandler) ----

/// The text of a message's data: `len` bytes at its address.
fn message_text(e: &Engine, m: &Message) -> Vec<u8> {
    e.mem.bytes(m.data, m.len)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h:
// DoPreLoadGameHousekeeping (the parts built so far: the player's appearance undo goes, the
// main-loop callbacks set to go at a load are marked (`CleanMLCallbacks`), the LN events' first
// frame after the load reports no change).
fn pre_load_housekeeping(e: &mut Engine) {
    let npc = player_base(e);
    saved(e).appearance_undo.erase(&npc);
    crate::main_loop::clean(e);
    crate::ln_events::pre_load(e);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), jip_nvse.cpp: NVSEMessageHandler (the save,
// load, exit and new game cases' saved data; their scripts come with JIP's script runner).
/// JIP's listener for xNVSE's messages.
pub fn listen(e: &mut Engine, m: &Message) {
    match m.kind {
        msg::EXIT_TO_MAIN_MENU => {
            process_changed_flags(e, changed::ALL);
            pre_load_housekeeping(e);
            saved(e).last_path.clear();
        }
        msg::SAVE_GAME => {
            // `COPY_BYTES(s_lastLoadedPath, fosPath, dataLen + 1)`.
            let path = message_text(e, m);
            let s = saved(e);
            s.last_path = path;
            s.changed = 0;
        }
        msg::PRE_LOAD_GAME => {
            let path = message_text(e, m);
            let s = saved(e);
            if path != s.last_path {
                s.last_path = path;
                s.changed = changed::ALL;
            }
            pre_load_housekeeping(e);
        }
        msg::POST_LOAD_GAME => {
            // `DoLoadGameHousekeeping` after a load that succeeded: its saved data's part.
            if m.data & 0xff != 0 {
                saved(e).changed = 0;
            }
        }
        msg::NEW_GAME => {
            let s = saved(e);
            s.serialized = SerializedVars::default();
            // nv-rs's own: nothing of the last co-save loaded goes into a new game's.
            s.carried.clear();
            s.serialized_tail.clear();
            s.had_section = false;
        }
        msg::MAIN_GAME_LOOP => {
            // The callbacks, then the LN events when any has a handler: one look at JIP's state
            // when there is nothing to do. Sent only once JIP answers "installed" (it asks for it,
            // `nvse::plugins::listen_per_frame`); the check keeps it so if another built-in asks.
            // (JIP's HUD cursor and item list box parts come with their UI commands.)
            let jip = e.ext::<Jip>();
            if !jip.answers_installed {
                return;
            }
            let callbacks = !jip.main_loop.callbacks.is_empty();
            let events = jip.ln_events.flags != 0;
            if callbacks {
                crate::main_loop::cycle(e);
                // A callback may have set an LN handler: the events see it this frame.
                crate::ln_events::process(e);
            } else if events {
                crate::ln_events::process(e);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "cosave_tests.rs"]
mod tests;
