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
//!   the last Required, else the last Recommended), every usable option of a SelectAll group;
//!   an ExactlyOne group left with none gets its first usable option (Vortex picks the first
//!   option even when it is not usable; a NotUsable one is never picked here).
//! - Group fix-ups: one option in an ExactlyOne or AtLeastOne group is SelectAll, in an
//!   AtMostOne group SelectAny (Mod Organizer 2); more than one Required option turns AtMostOne
//!   into SelectAny and ExactlyOne into AtLeastOne (Vortex).
//! - Files: the required files; each chosen option's files in a visible step; from options not
//!   chosen, files marked `alwaysInstall`, and `installIfUsable` ones when the option is not
//!   NotUsable (Vortex; Mod Organizer 2 ignores both marks); the conditional installs whose
//!   condition holds with every visible step's flags.
//! - Conflicts: the higher priority wins; at equal priority the file or folder written later in
//!   the document wins (Mod Organizer 2's order; it puts required files first and conditional
//!   ones last, as the schema orders them). Vortex breaks an equal priority by the larger
//!   source path instead.

use crate::config::{
    Condition, Config, FileItem, FileState, GroupType, Plugin, PluginType, TypeDescriptor,
};
use crate::layout::{join, normalise};
use std::collections::BTreeMap;

/// What the conditions are asked against: the player's load order and versions.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// Plugins the player has, with their state; names without case. One not listed is Missing.
    pub plugins: Vec<(String, FileState)>,
    /// The game's version (`1.4.0.525`); `None` lets every game version condition hold.
    pub game_version: Option<String>,
    /// Script extenders the port provides, by name (`nvse`) and version. One not listed fails
    /// its conditions.
    pub extenders: Vec<(String, String)>,
}

impl Facts {
    fn state(&self, file: &str) -> FileState {
        let file = normalise(file);
        self.plugins
            .iter()
            .find(|(n, _)| normalise(n).eq_ignore_ascii_case(&file))
            .map_or(FileState::Missing, |(_, s)| *s)
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
pub fn holds(condition: &Condition, flags: &BTreeMap<String, String>, facts: &Facts) -> bool {
    match condition {
        Condition::All(parts) => parts.iter().all(|c| holds(c, flags, facts)),
        Condition::Any(parts) => parts.iter().any(|c| holds(c, flags, facts)),
        Condition::File { file, state } => facts.state(file) == *state,
        Condition::Flag { flag, value } => flags.get(flag).map_or(value.is_empty(), |v| v == value),
        Condition::Game { version } => facts
            .game_version
            .as_deref()
            .map_or(true, |g| version_at_least(g, version)),
        // This port is the manager now; FOMM's own version numbers mean nothing to it.
        Condition::Manager { .. } => true,
        Condition::Extender { name, version } => facts
            .extenders
            .iter()
            .any(|(n, v)| n.eq_ignore_ascii_case(name) && version_at_least(v, version)),
    }
}

/// An option's type with these flags.
pub fn plugin_type(plugin: &Plugin, flags: &Flags, facts: &Facts) -> PluginType {
    match &plugin.kind {
        TypeDescriptor::Fixed(t) => *t,
        TypeDescriptor::Depends { default, patterns } => patterns
            .iter()
            .find(|(c, _)| holds(c, flags, facts))
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

/// One install in progress: the choices made so far.
#[derive(Debug, Clone)]
pub struct Session<'a> {
    config: &'a Config,
    facts: &'a Facts,
    /// Per step answered: per group, the chosen options' indices.
    chosen: BTreeMap<usize, Vec<Vec<usize>>>,
}

/// What working through the steps gives: per step whether it is visible and its choices (the
/// effective ones), and the flags after all of them.
struct Walk {
    visible: Vec<bool>,
    flags_before: Vec<Flags>,
    selection: Vec<Vec<Vec<usize>>>,
    flags: Flags,
}

impl<'a> Session<'a> {
    /// A new install, or why the mod cannot be installed (its `moduleDependencies` fail).
    pub fn new(config: &'a Config, facts: &'a Facts) -> Result<Self, Refused> {
        if let Some(dependencies) = &config.dependencies {
            if !holds(dependencies, &Flags::new(), facts) {
                return Err(Refused(format!(
                    "\"{}\" needs something that is not there: {}",
                    config.name,
                    describe(dependencies)
                )));
            }
        }
        Ok(Self {
            config,
            facts,
            chosen: BTreeMap::new(),
        })
    }

    fn walk(&self) -> Walk {
        let mut walk = Walk {
            visible: vec![],
            flags_before: vec![],
            selection: vec![],
            flags: Flags::new(),
        };
        for (i, step) in self.config.steps.iter().enumerate() {
            let visible = step
                .visible
                .as_ref()
                .map_or(true, |c| holds(c, &walk.flags, self.facts));
            let selection = self.selection(i, &walk.flags);
            walk.flags_before.push(walk.flags.clone());
            if visible {
                for (g, group) in step.groups.iter().enumerate() {
                    for &p in &selection[g] {
                        for (name, value) in &group.plugins[p].flags {
                            walk.flags.insert(name.clone(), value.clone());
                        }
                    }
                }
            }
            walk.visible.push(visible);
            walk.selection.push(selection);
        }
        walk
    }

    /// The effective choices of step `i` with the flags before it: the player's (Required ones
    /// added, NotUsable ones dropped), else the defaults.
    fn selection(&self, i: usize, flags: &Flags) -> Vec<Vec<usize>> {
        let step = &self.config.steps[i];
        step.groups
            .iter()
            .enumerate()
            .map(|(g, group)| {
                let types: Vec<PluginType> = group
                    .plugins
                    .iter()
                    .map(|p| plugin_type(p, flags, self.facts))
                    .collect();
                match self.chosen.get(&i) {
                    Some(chosen) => {
                        let mut set: Vec<usize> = (0..types.len())
                            .filter(|&p| {
                                types[p] == PluginType::Required
                                    || (chosen[g].contains(&p) && types[p] != PluginType::NotUsable)
                            })
                            .collect();
                        set.dedup();
                        set
                    }
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
        let walk = self.walk();
        (0..before.min(walk.visible.len()))
            .rev()
            .find(|&i| walk.visible[i])
    }

    /// Step `index` as the player sees it now.
    pub fn view(&self, index: usize) -> StepView {
        let walk = self.walk();
        let step = &self.config.steps[index];
        let flags = &walk.flags_before[index];
        StepView {
            index,
            name: step.name.clone(),
            groups: step
                .groups
                .iter()
                .enumerate()
                .map(|(g, group)| {
                    let types: Vec<PluginType> = group
                        .plugins
                        .iter()
                        .map(|p| plugin_type(p, flags, self.facts))
                        .collect();
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
                                selected: walk.selection[index][g].contains(&p),
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
        let walk = self.walk();
        let Some(the_step) = self.config.steps.get(step) else {
            return Err(Refused(format!("there is no step {step}")));
        };
        let Some(the_group) = the_step.groups.get(group) else {
            return Err(Refused(format!(
                "there is no group {group} in \"{}\"",
                the_step.name
            )));
        };
        let flags = &walk.flags_before[step];
        let types: Vec<PluginType> = the_group
            .plugins
            .iter()
            .map(|p| plugin_type(p, flags, self.facts))
            .collect();
        let mut set: Vec<usize> = options.to_vec();
        set.sort_unstable();
        set.dedup();
        if let Some(&bad) = set.iter().find(|&&p| p >= types.len()) {
            return Err(Refused(format!(
                "\"{}\" has no option {bad}",
                the_group.name
            )));
        }
        check(
            &the_group.name,
            &the_group.plugins,
            &types,
            effective_kind(the_group.kind, &types),
            &set,
        )?;
        let mut answer = walk.selection[step].clone();
        answer[group] = set;
        self.chosen.insert(step, answer);
        Ok(())
    }

    /// The files this install copies, from the archive's `listing` (paths with `/` or `\`, as the
    /// archive gives them), the installer's folder being `prefix` (from
    /// [`crate::layout::layout`]).
    pub fn plan(&self, listing: &[String], prefix: &str) -> Plan {
        let walk = self.walk();
        let mut items: Vec<&FileItem> = self.config.required.iter().collect();
        for (i, step) in self.config.steps.iter().enumerate() {
            if !walk.visible[i] {
                continue;
            }
            for (g, group) in step.groups.iter().enumerate() {
                for (p, plugin) in group.plugins.iter().enumerate() {
                    if walk.selection[i][g].contains(&p) {
                        items.extend(&plugin.files);
                    } else {
                        let usable =
                            plugin_type(plugin, &walk.flags, self.facts) != PluginType::NotUsable;
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
            if holds(condition, &walk.flags, self.facts) {
                items.extend(files);
            }
        }
        items.sort_by_key(|f| (f.priority, f.sequence));
        items.dedup_by_key(|f| f.sequence);
        resolve(&items, listing, prefix)
    }

    /// The flags after every visible step, for a front end that shows them.
    pub fn flags(&self) -> BTreeMap<String, String> {
        self.walk().flags
    }
}

/// The default choices of a group whose options have these types.
fn defaults(kind: GroupType, types: &[PluginType]) -> Vec<usize> {
    let kind = effective_kind(kind, types);
    let usable = |p: &usize| types[*p] != PluginType::NotUsable;
    let all: Vec<usize> = (0..types.len()).collect();
    match kind {
        GroupType::SelectAll => all.into_iter().filter(usable).collect(),
        GroupType::SelectExactlyOne | GroupType::SelectAtMostOne => {
            let last = |t: PluginType| all.iter().rev().find(|&&p| types[p] == t).copied();
            match last(PluginType::Required).or_else(|| last(PluginType::Recommended)) {
                Some(p) => vec![p],
                None if kind == GroupType::SelectExactlyOne => {
                    all.into_iter().find(usable).into_iter().collect()
                }
                None => vec![],
            }
        }
        GroupType::SelectAtLeastOne | GroupType::SelectAny => all
            .into_iter()
            .filter(|&p| matches!(types[p], PluginType::Required | PluginType::Recommended))
            .collect(),
    }
}

/// Whether `set` keeps the group's rule and its options' types.
fn check(
    name: &str,
    plugins: &[Plugin],
    types: &[PluginType],
    kind: GroupType,
    set: &[usize],
) -> Result<(), Refused> {
    for (p, t) in types.iter().enumerate() {
        if *t == PluginType::Required && !set.contains(&p) {
            return Err(Refused(format!(
                "\"{}\" in \"{name}\" is required",
                plugins[p].name
            )));
        }
        if *t == PluginType::NotUsable && set.contains(&p) {
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
    let n = set.len();
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

/// Whether a destination stays inside Data: no `..`, no drive, no full path.
fn inside_data(raw: &str) -> bool {
    let text = raw.replace('\\', "/");
    !(text.starts_with('/')
        || text.as_bytes().get(1) == Some(&b':')
        || text.split('/').any(|p| p.trim() == ".."))
}

/// The copies for these items, in the order where a later one wins.
fn resolve(items: &[&FileItem], listing: &[String], prefix: &str) -> Plan {
    let entries: Vec<(String, &String)> = listing
        .iter()
        .map(|e| (normalise(e).to_ascii_lowercase(), e))
        .collect();
    let prefix = normalise(prefix);
    let mut plan = Plan::default();
    let mut chosen: BTreeMap<String, FileCopy> = BTreeMap::new();
    let mut put = |plan: &mut Plan, destination: String, source: &str, priority: i64| {
        if !inside_data(&destination) {
            if !plan.refused.contains(&destination) {
                plan.refused.push(destination);
            }
            return;
        }
        let destination = normalise(&destination);
        chosen.insert(
            destination.to_ascii_lowercase(),
            FileCopy {
                source: source.to_owned(),
                destination,
                priority,
            },
        );
    };
    for item in items {
        let source = join(&prefix, &normalise(&item.source));
        let wanted = source.to_ascii_lowercase();
        let raw_destination = item
            .destination
            .clone()
            .unwrap_or_else(|| item.source.clone());
        // Checked as written: making it a relative path would hide a leading `\`.
        if !inside_data(&raw_destination) {
            if !plan.refused.contains(&raw_destination) {
                plan.refused.push(raw_destination);
            }
            continue;
        }
        if item.folder {
            let under = format!("{wanted}/");
            let mut any = false;
            for (lower, given) in &entries {
                if let Some(rest) = lower.strip_prefix(&under) {
                    any = true;
                    // The rest in the listing's own case.
                    let given_rest = &normalise(given)[under.len()..];
                    debug_assert_eq!(given_rest.to_ascii_lowercase(), rest);
                    put(
                        &mut plan,
                        join(&normalise(&raw_destination), given_rest),
                        given,
                        item.priority,
                    );
                }
            }
            if !any {
                plan.missing.push(item.source.clone());
            }
        } else {
            match entries.iter().find(|(lower, _)| *lower == wanted) {
                Some((_, given)) => {
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
                    put(&mut plan, destination, given, item.priority);
                }
                None => plan.missing.push(item.source.clone()),
            }
        }
    }
    plan.copies = chosen.into_values().collect();
    plan
}
