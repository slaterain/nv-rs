//! Running an installer: the steps a player sees, the choices, and the files to copy.
//!
//! [`Session`] holds the player's choices and nothing else; everything else is worked out from
//! the [`Config`] and the [`Facts`] each time, so going back a step and choosing again needs no
//! undoing. A front end asks for the first visible step, shows its [`StepView`], hands back the
//! chosen options with [`Session::choose`], asks for the next visible step, and at the end asks
//! for the [`Plan`].
//!
//! The rules (CENSUS.md, "Behaviour of the managers"):
//! - Flags: choosing an option sets its flags; a later step's setting wins over an earlier one.
//!   A step's visibility and its options' types are worked out with the flags of the visible
//!   steps before it (Mod Organizer 2). A flag never set equals `""` (Mod Organizer 2).
//! - A step that is not visible counts for nothing: neither its flags nor its files.
//! - A step the player has not answered counts with its default choices (below).
//! - Default choices of a group: Required options, Recommended ones (in a group of one choice,
//!   the last Required, else the first Recommended), every usable option of a SelectAll group;
//!   an ExactlyOne or AtLeastOne group left with none gets its first Optional option, else its
//!   first CouldBeUsable one (Mod Organizer 2; see `defaults`). [`Session::problems`] says what
//!   in the current choices breaks a group's rule; [`Session::plan`] refuses while there is any.
//! - Group fix-ups: one option in an ExactlyOne or AtLeastOne group is SelectAll, in an
//!   AtMostOne group SelectAny (Mod Organizer 2); more than one Required option turns AtMostOne
//!   into SelectAny and ExactlyOne into AtLeastOne (Vortex).
//! - Files: the required files; each chosen option's files in a visible step; from options not
//!   chosen, files marked `alwaysInstall`, and `installIfUsable` ones when the option is not
//!   NotUsable (Vortex; Mod Organizer 2 ignores both marks); the conditional installs whose
//!   condition holds with every visible step's flags.
//! - Conflicts: the higher priority wins; at equal priority the file or folder written later in
//!   the document wins. That is Mod Organizer 2's order: it numbers each `file`/`folder` as it
//!   reads the document (`m_FileSystemItemSequence`, before any sorting for display) and
//!   stable-sorts by priority, then that number; the schema puts required files first and
//!   conditional ones last. Vortex breaks an equal priority by the larger source path instead.
//! - Destinations are compared as Windows and macOS compare names (`layout::fold`: case, and
//!   dots and spaces at a name's end, do not count), and one that would leave Data or that no
//!   system holds safely (`..`, a drive or full path, a `:`, a Windows device name such as
//!   `CON`, `CONIN$` or `COM1`) is refused on every system.
//! - Work stays in proportion to the installer and the listing: one map of flags while walking
//!   the steps (never a copy per step), each group's choices as one flag per option (no
//!   searching a list per option), the player's files indexed once ([`Known`]), the listing
//!   indexed once, repeated identical file items resolved once, and at most [`MAX_COPIES`]
//!   copies (more is refused; the listing's own caps are in `layout`).

use crate::config::{
    Condition, Config, FileItem, FileState, GroupType, Plugin, PluginType, TypeDescriptor,
};
use crate::layout::{fold, fold_part, join, normalise, Listing, ListingError};
use std::collections::{BTreeMap, BTreeSet};

/// The most copies a plan may make: as many as the listing may hold files
/// (`layout::MAX_ENTRIES`); more can only come from one file copied to many places.
pub const MAX_COPIES: usize = crate::layout::MAX_ENTRIES;

/// What the conditions are asked against: the player's load order and versions.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// Files the player has under Data, with their state, compared as [`fold`] compares them:
    /// plugins Active or Inactive by the load order; any other file there (a DLL, a loose file,
    /// the built-in extenders' plugins the port provides) Active. One not listed is Missing.
    pub plugins: Vec<(String, FileState)>,
    /// The game's version (`1.4.0.525`); `None` lets every game version condition hold.
    pub game_version: Option<String>,
    /// Script extenders the port provides, by name (`nvse`) and version. One not listed fails
    /// its conditions.
    pub extenders: Vec<(String, String)>,
}

/// [`Facts`] indexed once: each file condition then costs one lookup, however many files the
/// player has.
#[derive(Debug, Clone, Default)]
pub struct Known {
    files: BTreeMap<String, FileState>,
    game_version: Option<String>,
    extenders: BTreeMap<String, String>,
}

impl Facts {
    /// The facts indexed; of two files that fold to one name, the first.
    pub fn index(&self) -> Known {
        let mut files = BTreeMap::new();
        for (name, state) in &self.plugins {
            files.entry(fold(name)).or_insert(*state);
        }
        Known {
            files,
            game_version: self.game_version.clone(),
            extenders: self
                .extenders
                .iter()
                .map(|(n, v)| (n.to_ascii_lowercase(), v.clone()))
                .collect(),
        }
    }
}

/// `a >= b` for dotted versions (`1.4.0.525`): each part's leading digits, missing parts 0.
pub fn version_at_least(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim()
            .split('.')
            .map(|p| {
                let digits: String = p.trim().chars().take_while(char::is_ascii_digit).collect();
                digits.parse().unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parts(a), parts(b));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    true
}

type Flags = BTreeMap<String, String>;

/// Whether `condition` holds with these flags and facts.
pub fn holds(condition: &Condition, flags: &BTreeMap<String, String>, known: &Known) -> bool {
    match condition {
        Condition::All(parts) => parts.iter().all(|c| holds(c, flags, known)),
        Condition::Any(parts) => parts.iter().any(|c| holds(c, flags, known)),
        Condition::File { file, state } => {
            known
                .files
                .get(&fold(file))
                .copied()
                .unwrap_or(FileState::Missing)
                == *state
        }
        Condition::Flag { flag, value } => flags.get(flag).map_or(value.is_empty(), |v| v == value),
        Condition::Game { version } => known
            .game_version
            .as_deref()
            .map_or(true, |g| version_at_least(g, version)),
        // This port is the manager now; FOMM's own version numbers mean nothing to it.
        Condition::Manager { .. } => true,
        Condition::Extender { name, version } => known
            .extenders
            .get(&name.to_ascii_lowercase())
            .is_some_and(|v| version_at_least(v, version)),
    }
}

/// An option's type with these flags.
pub fn plugin_type(plugin: &Plugin, flags: &Flags, known: &Known) -> PluginType {
    match &plugin.kind {
        TypeDescriptor::Fixed(t) => *t,
        TypeDescriptor::Depends { default, patterns } => patterns
            .iter()
            .find(|(c, _)| holds(c, flags, known))
            .map_or(*default, |(_, t)| *t),
    }
}

/// The group type as shown, after the fix-ups above.
fn effective_kind(kind: GroupType, types: &[PluginType]) -> GroupType {
    let required = types.iter().filter(|t| **t == PluginType::Required).count();
    match (kind, types.len()) {
        (GroupType::SelectExactlyOne | GroupType::SelectAtLeastOne, 1) => GroupType::SelectAll,
        (GroupType::SelectAtMostOne, 1) => GroupType::SelectAny,
        (GroupType::SelectAtMostOne, _) if required > 1 => GroupType::SelectAny,
        (GroupType::SelectExactlyOne, _) if required > 1 => GroupType::SelectAtLeastOne,
        _ => kind,
    }
}

/// One option as the player sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionView {
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub kind: PluginType,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupView {
    pub name: String,
    /// After the fix-ups: radio buttons for ExactlyOne and AtMostOne (with a "None"), check
    /// boxes for the rest, all fixed for SelectAll.
    pub kind: GroupType,
    pub options: Vec<OptionView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepView {
    pub index: usize,
    pub name: String,
    pub groups: Vec<GroupView>,
}

/// Why a choice was refused, in words a player reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One file to copy: from the archive (the listing's own spelling) to a path under Data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCopy {
    pub source: String,
    /// Under Data, `/` between the parts, never leaving Data.
    pub destination: String,
    pub priority: i64,
}

/// What an install does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// One per destination (conflicts decided), by destination without case.
    pub copies: Vec<FileCopy>,
    /// Sources the installer names that the archive does not hold (Mod Organizer 2 lists these
    /// and asks; Vortex warns and goes on).
    pub missing: Vec<String>,
    /// Destinations that would leave Data (`..`, a drive, a full path): never copied.
    pub refused: Vec<String>,
}

/// Why there is no plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// The current choices break a group's rule ([`Session::problems`]): the player must answer
    /// those steps first.
    Choices(Vec<(usize, Refused)>),
    /// The archive's listing is not a mod's (`layout::Listing::new`).
    Listing(ListingError),
    /// More copies than [`MAX_COPIES`].
    TooManyCopies,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Choices(problems) => {
                let words: Vec<String> = problems.iter().map(|(_, r)| r.0.clone()).collect();
                write!(f, "{}", words.join("; "))
            }
            PlanError::Listing(e) => write!(f, "{e}"),
            PlanError::TooManyCopies => write!(
                f,
                "the installer copies more files than any real mod ({MAX_COPIES} at most)"
            ),
        }
    }
}

/// One install in progress: the choices made so far.
#[derive(Debug, Clone)]
pub struct Session<'a> {
    config: &'a Config,
    known: Known,
    /// Per step answered: per group, one flag per option (chosen or not).
    chosen: BTreeMap<usize, Vec<Vec<bool>>>,
}

/// Per group, one flag per option: chosen or not.
type Selection = Vec<Vec<bool>>;

/// What working through the steps up to one gives: per step whether it is visible and its
/// effective choices, and the flags after them. One map of flags is kept, never a copy per step,
/// so a long installer costs time and memory in proportion to its size.
struct Walk {
    visible: Vec<bool>,
    selection: Vec<Selection>,
    flags: Flags,
}

/// The chosen options' indices.
fn chosen_indices(mask: &[bool]) -> Vec<usize> {
    mask.iter()
        .enumerate()
        .filter(|(_, on)| **on)
        .map(|(p, _)| p)
        .collect()
}

impl<'a> Session<'a> {
    /// A new install, or why the mod cannot be installed (its `moduleDependencies` fail).
    pub fn new(config: &'a Config, facts: &Facts) -> Result<Self, Refused> {
        let known = facts.index();
        if let Some(dependencies) = &config.dependencies {
            if !holds(dependencies, &Flags::new(), &known) {
                return Err(Refused(format!(
                    "\"{}\" needs something that is not there: {}",
                    config.name,
                    describe(dependencies)
                )));
            }
        }
        Ok(Self {
            config,
            known,
            chosen: BTreeMap::new(),
        })
    }

    /// The steps before `stop` worked through; `flags` are then the flags before step `stop`.
    fn walk_to(&self, stop: usize) -> Walk {
        let mut walk = Walk {
            visible: vec![],
            selection: vec![],
            flags: Flags::new(),
        };
        for (i, step) in self.config.steps.iter().enumerate().take(stop) {
            let visible = step
                .visible
                .as_ref()
                .map_or(true, |c| holds(c, &walk.flags, &self.known));
            let selection = self.selection(i, &walk.flags);
            if visible {
                apply_flags(&step.groups, &selection, &mut walk.flags);
            }
            walk.visible.push(visible);
            walk.selection.push(selection);
        }
        walk
    }

    fn walk(&self) -> Walk {
        self.walk_to(self.config.steps.len())
    }

    /// The types of a group's options with these flags.
    fn types(&self, plugins: &[Plugin], flags: &Flags) -> Vec<PluginType> {
        plugins
            .iter()
            .map(|p| plugin_type(p, flags, &self.known))
            .collect()
    }

    /// The effective choices of step `i` with the flags before it: the player's (Required ones
    /// added, NotUsable ones dropped), else the defaults.
    fn selection(&self, i: usize, flags: &Flags) -> Selection {
        let step = &self.config.steps[i];
        step.groups
            .iter()
            .enumerate()
            .map(|(g, group)| {
                let types = self.types(&group.plugins, flags);
                match self.chosen.get(&i) {
                    Some(chosen) => types
                        .iter()
                        .enumerate()
                        .map(|(p, t)| {
                            *t == PluginType::Required
                                || (chosen[g][p] && *t != PluginType::NotUsable)
                        })
                        .collect(),
                    None => defaults(group.kind, &types),
                }
            })
            .collect()
    }

    /// The first visible step after `after` (from the start with `None`).
    pub fn next_step(&self, after: Option<usize>) -> Option<usize> {
        let walk = self.walk();
        let from = after.map_or(0, |a| a + 1);
        (from..walk.visible.len()).find(|&i| walk.visible[i])
    }

    /// The last visible step before `before`.
    pub fn previous_step(&self, before: usize) -> Option<usize> {
        let walk = self.walk_to(before);
        (0..walk.visible.len()).rev().find(|&i| walk.visible[i])
    }

    /// Step `index` as the player sees it now.
    pub fn view(&self, index: usize) -> StepView {
        let walk = self.walk_to(index);
        let step = &self.config.steps[index];
        let selection = self.selection(index, &walk.flags);
        StepView {
            index,
            name: step.name.clone(),
            groups: step
                .groups
                .iter()
                .enumerate()
                .map(|(g, group)| {
                    let types = self.types(&group.plugins, &walk.flags);
                    GroupView {
                        name: group.name.clone(),
                        kind: effective_kind(group.kind, &types),
                        options: group
                            .plugins
                            .iter()
                            .enumerate()
                            .map(|(p, plugin)| OptionView {
                                name: plugin.name.clone(),
                                description: plugin.description.clone(),
                                image: plugin.image.clone(),
                                kind: types[p],
                                selected: selection[g][p],
                            })
                            .collect(),
                    }
                })
                .collect(),
        }
    }

    /// Choose `options` (indices in the view's order) in group `group` of step `step`; the
    /// step's other groups keep what they had. Refused when it breaks the group's rule or an
    /// option's type.
    pub fn choose(&mut self, step: usize, group: usize, options: &[usize]) -> Result<(), Refused> {
        let Some(the_step) = self.config.steps.get(step) else {
            return Err(Refused(format!("there is no step {step}")));
        };
        let Some(the_group) = the_step.groups.get(group) else {
            return Err(Refused(format!(
                "there is no group {group} in \"{}\"",
                the_step.name
            )));
        };
        let walk = self.walk_to(step);
        let types = self.types(&the_group.plugins, &walk.flags);
        let mut mask = vec![false; types.len()];
        for &p in options {
            match mask.get_mut(p) {
                Some(on) => *on = true,
                None => return Err(Refused(format!("\"{}\" has no option {p}", the_group.name))),
            }
        }
        check(
            &the_group.name,
            &the_group.plugins,
            &types,
            effective_kind(the_group.kind, &types),
            &mask,
        )?;
        let mut answer = self.selection(step, &walk.flags);
        answer[group] = mask;
        self.chosen.insert(step, answer);
        Ok(())
    }

    /// The indices chosen now in group `group` of step `step` (empty when there is none).
    pub fn chosen(&self, step: usize, group: usize) -> Vec<usize> {
        if step >= self.config.steps.len() {
            return vec![];
        }
        let walk = self.walk_to(step);
        self.selection(step, &walk.flags)
            .get(group)
            .map_or(vec![], |mask| chosen_indices(mask))
    }

    /// What in the visible steps' current choices breaks a group's rule (a group that needs an
    /// option left with none, an option that became Required or NotUsable through an earlier
    /// answer): (step, why), in step order. A front end keeps the player on such a step, as Mod
    /// Organizer 2 keeps Next disabled; [`Session::plan`] refuses while there is any.
    pub fn problems(&self) -> Vec<(usize, Refused)> {
        let mut out = Vec::new();
        let mut flags = Flags::new();
        let walk = self.walk();
        for (i, step) in self.config.steps.iter().enumerate() {
            if !walk.visible[i] {
                continue;
            }
            for (g, group) in step.groups.iter().enumerate() {
                let types = self.types(&group.plugins, &flags);
                if let Err(why) = check(
                    &group.name,
                    &group.plugins,
                    &types,
                    effective_kind(group.kind, &types),
                    &walk.selection[i][g],
                ) {
                    out.push((i, why));
                }
            }
            apply_flags(&step.groups, &walk.selection[i], &mut flags);
        }
        out
    }

    /// The files this install copies, from the archive's `listing` (files with `/` or `\`, as
    /// the archive gives them; folder entries are left out), the installer's folder being
    /// `prefix` (from [`crate::layout::layout`]). Refused while [`Session::problems`] is not
    /// empty, for a listing that is not a mod's, and past [`MAX_COPIES`].
    pub fn plan(&self, listing: &[String], prefix: &str) -> Result<Plan, PlanError> {
        let problems = self.problems();
        if !problems.is_empty() {
            return Err(PlanError::Choices(problems));
        }
        let listing = Listing::new(listing).map_err(PlanError::Listing)?;
        let walk = self.walk();
        let mut items: Vec<&FileItem> = self.config.required.iter().collect();
        for (i, step) in self.config.steps.iter().enumerate() {
            if !walk.visible[i] {
                continue;
            }
            for (g, group) in step.groups.iter().enumerate() {
                for (p, plugin) in group.plugins.iter().enumerate() {
                    if walk.selection[i][g][p] {
                        items.extend(&plugin.files);
                    } else {
                        let usable =
                            plugin_type(plugin, &walk.flags, &self.known) != PluginType::NotUsable;
                        items.extend(
                            plugin
                                .files
                                .iter()
                                .filter(|f| f.always_install || (f.install_if_usable && usable)),
                        );
                    }
                }
            }
        }
        for (condition, files) in &self.config.conditional {
            if holds(condition, &walk.flags, &self.known) {
                items.extend(files);
            }
        }
        items.sort_by_key(|f| (f.priority, f.sequence));
        // Of identical items (same source, destination, kind and priority) only the last
        // counts, as it would win anyway: each is resolved once.
        let mut seen = BTreeSet::new();
        let mut unique: Vec<&FileItem> = Vec::with_capacity(items.len());
        for item in items.iter().rev() {
            let key = (
                fold(&item.source),
                item.destination.as_deref().map(fold),
                item.folder,
                item.priority,
            );
            if seen.insert(key) {
                unique.push(item);
            }
        }
        unique.reverse();
        resolve(&unique, &listing, prefix)
    }

    /// The flags after every visible step, for a front end that shows them.
    pub fn flags(&self) -> BTreeMap<String, String> {
        self.walk().flags
    }
}

/// Sets the flags of a step's chosen options.
fn apply_flags(groups: &[crate::config::Group], selection: &Selection, flags: &mut Flags) {
    for (g, group) in groups.iter().enumerate() {
        for (p, plugin) in group.plugins.iter().enumerate() {
            if selection[g][p] {
                for (name, value) in &plugin.flags {
                    flags.insert(name.clone(), value.clone());
                }
            }
        }
    }
}

/// The default choices of a group whose options have these types. Required options are on.
/// Recommended ones too; in a group of one choice the first Recommended (Mod Organizer 2; Vortex
/// takes the last). A group that must have one (ExactlyOne, AtLeastOne) and has none on gets its
/// first Optional option, else its first CouldBeUsable one: Mod Organizer 2's rule (its page set-up
/// ticks `first_optional`, else `first_couldbe`, when `mustSelectOne`). Vortex picks the first
/// option only for ExactlyOne, even a NotUsable one, and leaves AtLeastOne empty; a NotUsable
/// option is never picked here.
fn defaults(kind: GroupType, types: &[PluginType]) -> Vec<bool> {
    let kind = effective_kind(kind, types);
    let first = |t: PluginType| types.iter().position(|x| *x == t);
    let last = |t: PluginType| types.iter().rposition(|x| *x == t);
    let mut mask = vec![false; types.len()];
    let fallback = |mask: &mut Vec<bool>| {
        if let Some(p) = first(PluginType::Optional).or_else(|| first(PluginType::CouldBeUsable)) {
            mask[p] = true;
        }
    };
    match kind {
        GroupType::SelectAll => {
            for (p, t) in types.iter().enumerate() {
                mask[p] = *t != PluginType::NotUsable;
            }
        }
        GroupType::SelectExactlyOne | GroupType::SelectAtMostOne => {
            match last(PluginType::Required).or_else(|| first(PluginType::Recommended)) {
                Some(p) => mask[p] = true,
                None if kind == GroupType::SelectExactlyOne => fallback(&mut mask),
                None => {}
            }
        }
        GroupType::SelectAtLeastOne | GroupType::SelectAny => {
            for (p, t) in types.iter().enumerate() {
                mask[p] = matches!(t, PluginType::Required | PluginType::Recommended);
            }
            if kind == GroupType::SelectAtLeastOne && !mask.contains(&true) {
                fallback(&mut mask);
            }
        }
    }
    mask
}

/// Whether the chosen options (`mask`) keep the group's rule and its options' types.
fn check(
    name: &str,
    plugins: &[Plugin],
    types: &[PluginType],
    kind: GroupType,
    mask: &[bool],
) -> Result<(), Refused> {
    for (p, t) in types.iter().enumerate() {
        if *t == PluginType::Required && !mask[p] {
            return Err(Refused(format!(
                "\"{}\" in \"{name}\" is required",
                plugins[p].name
            )));
        }
        if *t == PluginType::NotUsable && mask[p] {
            return Err(Refused(format!(
                "\"{}\" in \"{name}\" can't be chosen",
                plugins[p].name
            )));
        }
    }
    let usable = types
        .iter()
        .filter(|t| **t != PluginType::NotUsable)
        .count();
    let n = mask.iter().filter(|on| **on).count();
    let ok = match kind {
        _ if usable == 0 => true,
        GroupType::SelectExactlyOne => n == 1,
        GroupType::SelectAtMostOne => n <= 1,
        GroupType::SelectAtLeastOne => n >= 1,
        GroupType::SelectAll => n == usable,
        GroupType::SelectAny => true,
    };
    if ok {
        return Ok(());
    }
    Err(Refused(match kind {
        GroupType::SelectExactlyOne => format!("choose exactly one option in \"{name}\""),
        GroupType::SelectAtMostOne => format!("choose one option or none in \"{name}\""),
        GroupType::SelectAtLeastOne => format!("choose at least one option in \"{name}\""),
        _ => format!("every option in \"{name}\" is needed"),
    }))
}

/// A condition in words, for the message when a mod cannot be installed.
fn describe(condition: &Condition) -> String {
    match condition {
        Condition::All(parts) => parts.iter().map(describe).collect::<Vec<_>>().join(" and "),
        Condition::Any(parts) => parts.iter().map(describe).collect::<Vec<_>>().join(" or "),
        Condition::File { file, state } => match state {
            FileState::Active => format!("{file} switched on"),
            FileState::Inactive => format!("{file} present but switched off"),
            FileState::Missing => format!("{file} not there"),
        },
        Condition::Flag { flag, value } => format!("the choice {flag} = {value}"),
        Condition::Game { version } => format!("the game at version {version} or later"),
        Condition::Manager { version } => format!("a mod manager at version {version} or later"),
        Condition::Extender { name, version } => {
            format!("{} at version {version} or later", name.to_uppercase())
        }
    }
}

/// Whether a path part is one of Windows' device names (`CON`, `NUL`, `CONIN$`, `COM1`...), with
/// or without an extension, and with spaces before the dot (`CON .txt`): Windows opens the device
/// for such a name in any folder. `COM10` and `aux_cable` are ordinary names.
pub fn device_name(part: &str) -> bool {
    let folded = fold_part(part);
    let stem = folded.split('.').next().unwrap_or_default().trim_end();
    matches!(stem, "con" | "prn" | "aux" | "nul" | "conin$" | "conout$")
        || ((stem.starts_with("com") || stem.starts_with("lpt"))
            && stem.chars().count() == 4
            && stem
                .chars()
                .nth(3)
                .is_some_and(|c| c.is_ascii_digit() || "¹²³".contains(c)))
}

/// Whether a destination stays inside Data on every system: no `..`, no drive or full path (a
/// leading separator, a `:` anywhere: drives and NTFS streams), no Windows device names.
fn inside_data(raw: &str) -> bool {
    let text = raw.replace('\\', "/");
    !(text.starts_with('/')
        || text.contains(':')
        || text.split('/').any(|p| p.trim() == ".." || device_name(p)))
}

/// The copies for these items, in the order where a later one wins.
fn resolve(items: &[&FileItem], listing: &Listing, prefix: &str) -> Result<Plan, PlanError> {
    let prefix = normalise(prefix);
    let mut plan = Plan::default();
    let mut refused: BTreeSet<String> = BTreeSet::new();
    let mut chosen: BTreeMap<String, FileCopy> = BTreeMap::new();
    let mut considered = 0usize;
    let mut put = |refused: &mut BTreeSet<String>,
                   destination: String,
                   source: &str,
                   priority: i64|
     -> Result<(), PlanError> {
        considered += 1;
        if considered > MAX_COPIES {
            return Err(PlanError::TooManyCopies);
        }
        if !inside_data(&destination) {
            refused.insert(destination);
            return Ok(());
        }
        let destination = normalise(&destination);
        chosen.insert(
            fold(&destination),
            FileCopy {
                source: source.to_owned(),
                destination,
                priority,
            },
        );
        Ok(())
    };
    for item in items {
        let source = join(&prefix, &normalise(&item.source));
        let raw_destination = item
            .destination
            .clone()
            .unwrap_or_else(|| item.source.clone());
        // Checked as written: making it a relative path would hide a leading `\`.
        if !inside_data(&raw_destination) {
            refused.insert(raw_destination);
            continue;
        }
        if item.folder {
            let files = listing.under(&source);
            if files.is_empty() {
                plan.missing.push(item.source.clone());
            }
            for (rest, given) in files {
                put(
                    &mut refused,
                    join(&normalise(&raw_destination), &rest),
                    given,
                    item.priority,
                )?;
            }
        } else {
            match listing.file(&source) {
                Some(given) => {
                    let trimmed = raw_destination.trim();
                    // An empty destination, or one ending in a separator, is a folder: the file
                    // keeps its own name there.
                    let destination = if trimmed.is_empty() || trimmed.ends_with(['/', '\\']) {
                        let own = normalise(given);
                        let name = own.rsplit('/').next().unwrap_or(&own).to_owned();
                        join(&normalise(trimmed), &name)
                    } else {
                        trimmed.to_owned()
                    };
                    put(&mut refused, destination, given, item.priority)?;
                }
                None => plan.missing.push(item.source.clone()),
            }
        }
    }
    plan.copies = chosen.into_values().collect();
    plan.refused = refused.into_iter().collect();
    Ok(plan)
}
