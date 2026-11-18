//! The installer (`fomod\ModuleConfig.xml`) and the mod's details (`fomod\info.xml`) as data.
//!
//! One tolerant pass reads every schema version (CENSUS.md, "Older schema versions"): 1.0's
//! spellings (`moduleDependancies`, `dependancyType`, `dependancies`, `dependancy`,
//! `moduleFileDependancy`, `falloutDependancy`, `fommDependancy`) and its one step of
//! `optionalFileGroups` straight under the root; 2.0's flags and conditional installs; 3.0's
//! `order` attributes; 4.0's `installSteps` and `visible`; 5.x's `gameDependency`, nested
//! `dependencies` and the extenders' `nvseDependency`-style elements. No newer version gives an
//! older name another meaning, so accepting all of them at once is safe.
//!
//! Names are compared without case. What the reader cannot place (an unknown element where a
//! condition or a file should be) is left out and noted in [`Config::warnings`]; a mod is never
//! refused for it, as Mod Organizer 2 skips such elements too.

use crate::text;
use crate::xml::{self, Element};

/// The order steps, groups or options are shown in (`order` attribute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// By name, A to Z (the schema's default).
    Ascending,
    /// By name, Z to A.
    Descending,
    /// As written.
    Explicit,
}

/// How many options of a group may be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupType {
    SelectExactlyOne,
    SelectAtMostOne,
    SelectAtLeastOne,
    SelectAll,
    SelectAny,
}

/// What an option is to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// Chosen and cannot be left out.
    Required,
    /// Chosen at the start; may be left out.
    Recommended,
    Optional,
    /// Cannot be chosen.
    NotUsable,
    /// May be chosen (shown with a caution by some managers).
    CouldBeUsable,
}

/// The state a `fileDependency` asks of a plugin in the player's load order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    /// In the load order and on.
    Active,
    /// Present but not on.
    Inactive,
    /// Not there.
    Missing,
}

/// A condition (`compositeDependency` and its parts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Every part holds (`operator="And"`, the default); none at all holds.
    All(Vec<Condition>),
    /// At least one part holds (`operator="Or"`); none at all does not hold.
    Any(Vec<Condition>),
    File {
        file: String,
        state: FileState,
    },
    Flag {
        flag: String,
        value: String,
    },
    /// The game's version is at least this.
    Game {
        version: String,
    },
    /// The mod manager's version is at least this (FOMM's own number).
    Manager {
        version: String,
    },
    /// A script extender (`nvse`, `fose`, ...) of at least this version.
    Extender {
        name: String,
        version: String,
    },
}

/// A file or folder an option, the required list or a conditional install copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    /// The path in the archive, from the folder holding `fomod` (as written).
    pub source: String,
    /// The `destination` attribute as written; `None` when there is none (the source's path
    /// then, as every manager does).
    pub destination: Option<String>,
    pub folder: bool,
    pub priority: i64,
    pub always_install: bool,
    pub install_if_usable: bool,
    /// Its place in the document (0 first): at equal priority the later one wins.
    pub sequence: usize,
}

/// How an option's type is decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDescriptor {
    Fixed(PluginType),
    /// The first pattern whose condition holds gives the type; else the default.
    Depends {
        default: PluginType,
        patterns: Vec<(Condition, PluginType)>,
    },
}

/// One option of a group (a `plugin` in the schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub files: Vec<FileItem>,
    /// The flags choosing it sets, name and value, in order.
    pub flags: Vec<(String, String)>,
    pub kind: TypeDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub kind: GroupType,
    /// In the order they are shown.
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Empty for the one unnamed step of the 1.0 to 3.0 layouts.
    pub name: String,
    /// Shown only when this holds; `None` always.
    pub visible: Option<Condition>,
    /// In the order they are shown.
    pub groups: Vec<Group>,
}

/// The whole installer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub name: String,
    pub image: Option<String>,
    /// The schema version the file names (`ModConfig5.0.xsd` gives `(5, 0)`); `None` when it
    /// names none (read as 1.0 where that matters, as Vortex does).
    pub schema: Option<(u32, u32)>,
    /// When this does not hold the mod cannot be installed.
    pub dependencies: Option<Condition>,
    pub required: Vec<FileItem>,
    /// In the order they are shown.
    pub steps: Vec<Step>,
    pub conditional: Vec<(Condition, Vec<FileItem>)>,
    /// What was left out, in plain words.
    pub warnings: Vec<String>,
}

/// The mod's details (`info.xml`; every field may be missing).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub website: String,
    pub id: String,
}

/// Why a `ModuleConfig.xml` could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The largest installer or details file read: real ones are a few hundred KB at most, and a
/// larger one is refused before it is decoded.
pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

/// The most steps an installer may have (real ones: tens).
pub const MAX_STEPS: usize = 1_024;

/// The most groups, all steps together (real installers: tens).
pub const MAX_GROUPS: usize = 10_000;

/// The most options one group may have (real ones: a few, at most a few hundred).
pub const MAX_OPTIONS_PER_GROUP: usize = 2_000;

/// The most options, all groups together (real installers: tens to a few hundred).
pub const MAX_OPTIONS: usize = 20_000;

/// The most `file` and `folder` items, everywhere together (real installers: hundreds).
pub const MAX_FILE_ITEMS: usize = 200_000;

/// The most flags set and conditions tested, everywhere together (real installers: hundreds).
pub const MAX_CONDITIONS: usize = 50_000;

/// How deep conditions may nest inside each other (real ones: two or three).
pub const MAX_CONDITION_DEPTH: usize = 32;

/// Every cap above refuses the whole installer with a [`ConfigError`] that names it: nothing is
/// left out silently, so a mod is never installed by half. Each is far above any real installer
/// and low enough that the work on what it bounds stays short.
fn over(what: &str, cap: usize) -> String {
    format!("the installer has more {what} than any real installer ({cap} at most)")
}

/// The installer in `bytes` (the file as it is in the archive).
pub fn parse_config(bytes: &[u8]) -> Result<Config, ConfigError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(ConfigError(format!(
            "the installer file is {} bytes; more than {MAX_FILE_BYTES} is not an installer",
            bytes.len()
        )));
    }
    let text = text::decode(bytes);
    let root = xml::parse(&text)
        .map_err(|e| ConfigError(format!("the installer file can't be read: {e}")))?;
    if !root.is(&["config"]) {
        return Err(ConfigError(format!(
            "the installer file's first element is <{}>, not <config>",
            root.name
        )));
    }
    Reader::default().config(&root)
}

/// The details in `bytes` (`info.xml`); an unreadable file gives empty details.
pub fn parse_info(bytes: &[u8]) -> Info {
    if bytes.len() > MAX_FILE_BYTES {
        return Info::default();
    }
    let text = text::decode(bytes);
    let Ok(root) = xml::parse(&text) else {
        return Info::default();
    };
    let field = |name: &str| {
        root.child(&[name])
            .map(|e| e.text().trim().to_owned())
            .unwrap_or_default()
    };
    Info {
        name: field("Name"),
        author: field("Author"),
        version: field("Version"),
        description: field("Description"),
        website: field("Website"),
        id: field("Id"),
    }
}

/// The version in a schema location such as `http://qconsulting.ca/fo3/ModConfig5.0.xsd`.
fn schema_version(root: &Element) -> Option<(u32, u32)> {
    let location = root.attr("noNamespaceSchemaLocation")?.to_ascii_lowercase();
    let at = location
        .rfind("modconfig")
        .map(|i| i + "modconfig".len())
        .or_else(|| location.rfind("xmlscript").map(|i| i + "xmlscript".len()))?;
    let rest = &location[at..];
    let number = &rest[..rest.find(".xsd")?];
    let mut parts = number.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map_or(Some(0), |m| m.parse().ok())?;
    Some((major, minor))
}

fn order_of(element: &Element, default: Order) -> Order {
    match element.attr("order") {
        Some(o) if o.eq_ignore_ascii_case("Ascending") => Order::Ascending,
        Some(o) if o.eq_ignore_ascii_case("Descending") => Order::Descending,
        Some(o) if o.eq_ignore_ascii_case("Explicit") => Order::Explicit,
        _ => default,
    }
}

/// Sorts by name as Mod Organizer 2 and pyfomod do (plain comparison, case counts); stable, so
/// equal names keep their written order.
fn sort_by<T>(items: &mut [T], order: Order, name: impl Fn(&T) -> &str) {
    match order {
        Order::Ascending => items.sort_by(|a, b| name(a).cmp(name(b))),
        Order::Descending => items.sort_by(|a, b| name(b).cmp(name(a))),
        Order::Explicit => {}
    }
}

fn boolean(value: Option<&str>) -> bool {
    value.is_some_and(|v| v.trim().eq_ignore_ascii_case("true") || v.trim() == "1")
}

fn plugin_type(name: Option<&str>) -> Option<PluginType> {
    let name = name?.trim();
    [
        ("Required", PluginType::Required),
        ("Recommended", PluginType::Recommended),
        ("Optional", PluginType::Optional),
        ("NotUsable", PluginType::NotUsable),
        ("CouldBeUsable", PluginType::CouldBeUsable),
    ]
    .into_iter()
    .find(|(n, _)| n.eq_ignore_ascii_case(name))
    .map(|(_, t)| t)
}

const DEPENDENCIES: &[&str] = &["dependencies", "dependancies"];

/// The kinds of things counted against a cap.
#[derive(Clone, Copy)]
enum Kind {
    Groups,
    Options,
    Conditions,
}

#[derive(Default)]
struct Reader {
    sequence: usize,
    /// How many conditions the one being read is inside.
    depth: usize,
    schema: Option<(u32, u32)>,
    warnings: Vec<String>,
    // Counts against the caps; the first cap passed.
    groups: usize,
    options: usize,
    conditions: usize,
    refused: Option<String>,
}

impl Reader {
    fn warn(&mut self, text: String) {
        self.warnings.push(text);
    }

    /// Notes the first cap passed; the installer is then refused.
    fn refuse(&mut self, why: String) {
        self.refused.get_or_insert(why);
    }

    /// Counts `n` more of a capped kind; true when still within the cap.
    fn count(&mut self, kind: Kind, n: usize) -> bool {
        let (count, cap, what) = match kind {
            Kind::Groups => (&mut self.groups, MAX_GROUPS, "groups"),
            Kind::Options => (&mut self.options, MAX_OPTIONS, "options"),
            Kind::Conditions => (&mut self.conditions, MAX_CONDITIONS, "flags and conditions"),
        };
        *count += n;
        if *count > cap {
            self.refuse(over(what, cap));
            return false;
        }
        true
    }

    fn config(mut self, root: &Element) -> Result<Config, ConfigError> {
        self.schema = schema_version(root);
        let mut config = Config {
            schema: self.schema,
            ..Config::default()
        };
        // In document order, so the files' sequence numbers are the document's.
        for element in root.elements() {
            if element.is(&["moduleName"]) {
                config.name = element.text().trim().to_owned();
            } else if element.is(&["moduleImage"]) {
                config.image = element.attr("path").map(str::to_owned);
            } else if element.is(&["moduleDependencies", "moduleDependancies"]) {
                config.dependencies = Some(self.composite(element));
            } else if element.is(&["requiredInstallFiles"]) {
                config.required = self.files(element);
            } else if element.is(&["installSteps"]) {
                let order = order_of(element, Order::Ascending);
                let mut steps: Vec<Step> = element
                    .children_named(&["installStep"])
                    .map(|s| self.step(s))
                    .collect();
                sort_by(&mut steps, order, |s| &s.name);
                config.steps.extend(steps);
            } else if element.is(&["optionalFileGroups"]) {
                // 1.0 to 3.0: one unnamed step. 1.0 and 2.0 had no `order` and showed the groups
                // as written (Vortex's parsers); 3.0's schema default is Ascending.
                let default = if self.schema.unwrap_or((1, 0)) < (3, 0) {
                    Order::Explicit
                } else {
                    Order::Ascending
                };
                let groups = self.groups(element, default);
                config.steps.push(Step {
                    name: String::new(),
                    visible: None,
                    groups,
                });
            } else if element.is(&["conditionalFileInstalls"]) {
                for patterns in element.children_named(&["patterns"]) {
                    for pattern in patterns.children_named(&["pattern"]) {
                        let condition = match pattern.child(DEPENDENCIES) {
                            Some(d) => self.composite(d),
                            None => Condition::All(vec![]),
                        };
                        let files = pattern.child(&["files"]).map_or(vec![], |f| self.files(f));
                        config.conditional.push((condition, files));
                    }
                }
            } else {
                self.warn(format!(
                    "<{}> under <config> is not part of an installer; left out",
                    element.name
                ));
            }
            // After every element, so repeated `optionalFileGroups` cannot pass it either.
            if config.steps.len() > MAX_STEPS {
                self.refuse(over("steps", MAX_STEPS));
                break;
            }
        }
        if self.sequence > MAX_FILE_ITEMS {
            self.refuse(over("files and folders", MAX_FILE_ITEMS));
        }
        if let Some(why) = self.refused {
            return Err(ConfigError(why));
        }
        config.warnings = self.warnings;
        Ok(config)
    }

    fn step(&mut self, element: &Element) -> Step {
        let visible = element.child(&["visible"]).map(|v| self.composite(v));
        let groups = element
            .child(&["optionalFileGroups"])
            .map_or(vec![], |g| self.groups(g, Order::Ascending));
        Step {
            name: element.attr("name").unwrap_or_default().to_owned(),
            visible,
            groups,
        }
    }

    fn groups(&mut self, element: &Element, default: Order) -> Vec<Group> {
        let order = order_of(element, default);
        let mut groups: Vec<Group> = element
            .children_named(&["group"])
            .map(|g| self.group(g))
            .collect();
        self.count(Kind::Groups, groups.len());
        sort_by(&mut groups, order, |g| &g.name);
        groups
    }

    fn group(&mut self, element: &Element) -> Group {
        let name = element.attr("name").unwrap_or_default().to_owned();
        let written = element.attr("type").unwrap_or_default().trim();
        let kind = [
            ("SelectExactlyOne", GroupType::SelectExactlyOne),
            ("SelectAtMostOne", GroupType::SelectAtMostOne),
            ("SelectAtLeastOne", GroupType::SelectAtLeastOne),
            ("SelectAll", GroupType::SelectAll),
            ("SelectAny", GroupType::SelectAny),
        ]
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(written))
        .map(|(_, t)| t)
        .unwrap_or_else(|| {
            self.warn(format!(
                "group \"{name}\" has the type \"{written}\"; read as SelectAny"
            ));
            GroupType::SelectAny
        });
        let mut plugins = Vec::new();
        let mut order = Order::Ascending;
        if let Some(list) = element.child(&["plugins"]) {
            order = order_of(list, Order::Ascending);
            plugins = list
                .children_named(&["plugin"])
                .map(|p| self.plugin(p))
                .collect();
        }
        if plugins.len() > MAX_OPTIONS_PER_GROUP {
            self.refuse(format!(
                "the group \"{name}\" has more options than any real installer's ({MAX_OPTIONS_PER_GROUP} at most)"
            ));
        }
        self.count(Kind::Options, plugins.len());
        sort_by(&mut plugins, order, |p| &p.name);
        Group {
            name,
            kind,
            plugins,
        }
    }

    fn plugin(&mut self, element: &Element) -> Plugin {
        let name = element.attr("name").unwrap_or_default().to_owned();
        let mut plugin = Plugin {
            name,
            description: element
                .child(&["description"])
                .map(|d| d.text().trim().to_owned())
                .unwrap_or_default(),
            image: element
                .child(&["image"])
                .and_then(|i| i.attr("path"))
                .map(str::to_owned),
            files: vec![],
            flags: vec![],
            kind: TypeDescriptor::Fixed(PluginType::Optional),
        };
        // In document order: `files` and `conditionFlags` may come either way round.
        for child in element.elements() {
            if child.is(&["files"]) {
                let files = self.files(child);
                plugin.files.extend(files);
            } else if child.is(&["conditionFlags"]) {
                for flag in child.children_named(&["flag"]) {
                    self.count(Kind::Conditions, 1);
                    plugin.flags.push((
                        flag.attr("name").unwrap_or_default().to_owned(),
                        flag.text(),
                    ));
                }
            } else if child.is(&["typeDescriptor"]) {
                plugin.kind = self.type_descriptor(child, &plugin.name);
            }
        }
        plugin
    }

    fn type_descriptor(&mut self, element: &Element, plugin: &str) -> TypeDescriptor {
        if let Some(fixed) = element.child(&["type"]) {
            return TypeDescriptor::Fixed(self.named_type(fixed, plugin));
        }
        if let Some(depends) = element.child(&["dependencyType", "dependancyType"]) {
            let default = depends
                .child(&["defaultType"])
                .map_or(PluginType::Optional, |d| self.named_type(d, plugin));
            let mut patterns = Vec::new();
            for list in depends.children_named(&["patterns"]) {
                for pattern in list.children_named(&["pattern"]) {
                    let condition = match pattern.child(DEPENDENCIES) {
                        Some(d) => self.composite(d),
                        None => Condition::All(vec![]),
                    };
                    let kind = pattern
                        .child(&["type"])
                        .map_or(PluginType::Optional, |t| self.named_type(t, plugin));
                    patterns.push((condition, kind));
                }
            }
            return TypeDescriptor::Depends { default, patterns };
        }
        TypeDescriptor::Fixed(PluginType::Optional)
    }

    fn named_type(&mut self, element: &Element, plugin: &str) -> PluginType {
        plugin_type(element.attr("name")).unwrap_or_else(|| {
            self.warn(format!(
                "option \"{plugin}\" has the type \"{}\"; read as Optional",
                element.attr("name").unwrap_or_default()
            ));
            PluginType::Optional
        })
    }

    /// A `compositeDependency` (or 1.0's plain list, an And).
    fn composite(&mut self, element: &Element) -> Condition {
        if self.depth >= MAX_CONDITION_DEPTH {
            self.refuse(format!(
                "the installer's conditions nest deeper than any real installer's ({MAX_CONDITION_DEPTH} at most)"
            ));
            return Condition::Any(vec![]);
        }
        self.depth += 1;
        let parts: Vec<Condition> = element
            .elements()
            .filter_map(|c| self.condition(c))
            .collect();
        self.depth -= 1;
        match element.attr("operator") {
            Some(o) if o.trim().eq_ignore_ascii_case("Or") => Condition::Any(parts),
            _ => Condition::All(parts),
        }
    }

    fn condition(&mut self, element: &Element) -> Option<Condition> {
        self.count(Kind::Conditions, 1);
        let attr = |n: &str| element.attr(n).unwrap_or_default().to_owned();
        if element.is(DEPENDENCIES) {
            return Some(self.composite(element));
        }
        if element.is(&["fileDependency", "dependancy"]) {
            let written = attr("state");
            let state = [
                ("Active", FileState::Active),
                ("Inactive", FileState::Inactive),
                ("Missing", FileState::Missing),
            ]
            .into_iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(written.trim()))
            .map(|(_, s)| s)
            .unwrap_or_else(|| {
                self.warn(format!(
                    "file condition on \"{}\" asks for the state \"{written}\"; read as Active",
                    attr("file")
                ));
                FileState::Active
            });
            return Some(Condition::File {
                file: attr("file"),
                state,
            });
        }
        if element.is(&["moduleFileDependency", "moduleFileDependancy"]) {
            return Some(Condition::File {
                file: attr("file"),
                state: FileState::Active,
            });
        }
        if element.is(&["flagDependency"]) {
            return Some(Condition::Flag {
                flag: attr("flag"),
                value: attr("value"),
            });
        }
        if element.is(&["gameDependency", "falloutDependency", "falloutDependancy"]) {
            return Some(Condition::Game {
                version: attr("version"),
            });
        }
        if element.is(&["fommDependency", "fommDependancy"]) {
            return Some(Condition::Manager {
                version: attr("version"),
            });
        }
        let own = xml::local(&element.name).to_ascii_lowercase();
        if let Some(name) = own
            .strip_suffix("sedependency")
            .or_else(|| own.strip_suffix("sedependancy"))
            .filter(|n| !n.is_empty())
        {
            return Some(Condition::Extender {
                name: format!("{name}se"),
                version: attr("version"),
            });
        }
        self.warn(format!("<{}> is not a condition; left out", element.name));
        None
    }

    fn files(&mut self, element: &Element) -> Vec<FileItem> {
        let mut out = Vec::new();
        for item in element.elements() {
            let folder = item.is(&["folder"]);
            if !folder && !item.is(&["file"]) {
                self.warn(format!(
                    "<{}> in a file list is not a file or folder; left out",
                    item.name
                ));
                continue;
            }
            let sequence = self.sequence;
            self.sequence += 1;
            let source = item.attr("source").unwrap_or_default().trim().to_owned();
            // An empty source is how authors write "this option installs nothing" (Mod
            // Organizer 2 skips it the same way).
            if source.is_empty() {
                continue;
            }
            let priority = item
                .attr("priority")
                .and_then(|p| p.trim().parse::<i64>().ok())
                .unwrap_or(0);
            out.push(FileItem {
                source,
                destination: item.attr("destination").map(str::to_owned),
                folder,
                priority,
                always_install: boolean(item.attr("alwaysInstall")),
                install_if_usable: boolean(item.attr("installIfUsable")),
                sequence,
            });
        }
        out
    }
}
