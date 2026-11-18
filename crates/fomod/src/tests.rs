//! The crate's tests, on installers written for them: no mod's own files. Two of them have the
//! shapes met most in New Vegas mods, written here from scratch: a texture pack (a resolution
//! step that sets a flag, a step shown only for one resolution, files picked by flag
//! combinations, a priority to win a clash) and a weapon mod with DLC patches (a required base,
//! options whose type depends on which DLC the player has, a mod that needs the game's master).

use crate::config::*;
use crate::install::*;
use crate::layout::*;
use crate::text::decode;
use crate::xml;

// The tests' own short forms (local items shadow the glob imports above).
fn layout(listing: &[String]) -> Layout {
    crate::layout::layout(listing).expect("a mod's listing")
}

fn data_folder(listing: &[String]) -> Option<String> {
    crate::layout::data_folder(listing).expect("a mod's listing")
}

fn holds(c: &Condition, flags: &std::collections::BTreeMap<String, String>, f: &Facts) -> bool {
    crate::install::holds(c, flags, &f.index())
}

trait PlanOk {
    fn plan_ok(&self, listing: &[String], prefix: &str) -> Plan;
}

impl PlanOk for Session<'_> {
    fn plan_ok(&self, listing: &[String], prefix: &str) -> Plan {
        self.plan(listing, prefix).expect("a plan")
    }
}

fn list(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

fn utf16le_bom(text: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

fn config(text: &str) -> Config {
    parse_config(text.as_bytes()).expect("the test's installer reads")
}

fn facts(plugins: &[(&str, FileState)]) -> Facts {
    Facts {
        plugins: plugins.iter().map(|(n, s)| ((*n).to_owned(), *s)).collect(),
        game_version: Some("1.4.0.525".into()),
        extenders: vec![],
    }
}

/// The plan's copies as "destination <- source".
fn copies(plan: &Plan) -> Vec<String> {
    plan.copies
        .iter()
        .map(|c| format!("{} <- {}", c.destination, c.source))
        .collect()
}

fn selected(view: &StepView) -> Vec<Vec<&str>> {
    view.groups
        .iter()
        .map(|g| {
            g.options
                .iter()
                .filter(|o| o.selected)
                .map(|o| o.name.as_str())
                .collect()
        })
        .collect()
}

// ------------------------------------------------------------------------------------------------
// Text and XML

#[test]
fn every_encoding_met_reads_as_the_same_text() {
    let text = "<config a=\"\u{e9}\u{2019}\"/>";
    let mut be = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        be.extend_from_slice(&unit.to_be_bytes());
    }
    let le_no_bom: Vec<u8> = utf16le_bom(text)[2..].to_vec();
    let be_no_bom: Vec<u8> = be[2..].to_vec();
    let mut utf8_bom = vec![0xEF, 0xBB, 0xBF];
    utf8_bom.extend_from_slice(text.as_bytes());
    for bytes in [
        utf16le_bom(text),
        be,
        le_no_bom,
        be_no_bom,
        utf8_bom,
        text.as_bytes().to_vec(),
    ] {
        assert_eq!(decode(&bytes), text, "{:02x?}", &bytes[..6]);
    }
    // Windows-1252 bytes in a file that is not UTF-8: e-acute and a right quote.
    assert_eq!(decode(b"<config a=\"\xE9\x92\"/>"), text);
}

#[test]
fn a_wrong_encoding_declaration_is_ignored() {
    // Declared UTF-16, really UTF-8; and declared UTF-8, really UTF-16.
    let utf8 =
        "<?xml version=\"1.0\" encoding=\"UTF-16\"?><config><moduleName>A</moduleName></config>";
    assert_eq!(config(utf8).name, "A");
    let utf16 = utf16le_bom(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><config><moduleName>B</moduleName></config>",
    );
    assert_eq!(parse_config(&utf16).unwrap().name, "B");
}

#[test]
fn the_reader_takes_what_mods_write() {
    let root = xml::parse(
        "\u{feff}  <?xml version='1.0'?>\n<!DOCTYPE config [ <!ENTITY x 'y'> ]>\n<!-- a <comment> -->\
         <CONFIG Name='single' other=bare>\
           <Description>Meshes & Textures &amp; more &lt;3 &#65;&#x42; &bogus; &</Description>\
           <d><![CDATA[<b>bold</b> & raw]]></d>\
           <empty/><a>x</b>y</a>\
           <open>never closed",
    )
    .unwrap();
    assert!(root.is(&["config"]));
    assert_eq!(root.attr("NAME"), Some("single"));
    assert_eq!(root.attr("other"), Some("bare"));
    assert_eq!(
        root.child(&["description"]).unwrap().text(),
        "Meshes & Textures & more <3 AB &bogus; &"
    );
    assert_eq!(root.child(&["d"]).unwrap().text(), "<b>bold</b> & raw");
    assert!(root.child(&["empty"]).unwrap().children.is_empty());
    // A stray end tag is skipped.
    assert_eq!(root.child(&["a"]).unwrap().text(), "xy");
    assert_eq!(root.child(&["open"]).unwrap().text(), "never closed");
    assert_eq!(xml::parse("no element here"), Err(xml::XmlError::NoElement));
}

#[test]
fn a_file_that_is_no_installer_is_refused_in_words() {
    assert!(parse_config(b"<fomod><Name>x</Name></fomod>")
        .unwrap_err()
        .0
        .contains("<fomod>"));
    assert!(parse_config(b"").is_err());
}

#[test]
fn info_gives_the_mods_details() {
    let info = parse_info(&utf16le_bom(
        "<fomod><Name> Gun Pack </Name><Author>Someone</Author>\
         <Version MachineVersion=\"1.2\">1.2b</Version><Website>https://example.org</Website></fomod>",
    ));
    assert_eq!(info.name, "Gun Pack");
    assert_eq!(info.author, "Someone");
    assert_eq!(info.version, "1.2b");
    assert_eq!(info.website, "https://example.org");
    assert_eq!(info.description, "");
    assert_eq!(parse_info(b"not xml"), Info::default());
}

// ------------------------------------------------------------------------------------------------
// Where things are in an archive

#[test]
fn the_fomod_folder_is_found_at_the_top_or_down_single_folders() {
    let top = list(&["fomod/ModuleConfig.xml", "fomod/info.xml", "a/b.dds"]);
    assert_eq!(
        layout(&top),
        Layout::Installer {
            prefix: String::new(),
            config: "fomod/ModuleConfig.xml".into(),
            info: Some("fomod/info.xml".into()),
        }
    );
    let nested = list(&[
        "__MACOSX/Gun Pack/._x",
        "Gun Pack\\v2\\FOMOD\\moduleconfig.XML",
        "Gun Pack\\v2\\Main\\GunPack.esp",
    ]);
    assert_eq!(
        layout(&nested),
        Layout::Installer {
            prefix: "Gun Pack/v2".into(),
            config: "Gun Pack\\v2\\FOMOD\\moduleconfig.XML".into(),
            info: None,
        }
    );
    // Two folders at the top: not looked into.
    let two = list(&["a/fomod/ModuleConfig.xml", "b/x.esp"]);
    assert_eq!(
        layout(&two),
        Layout::Plain {
            data: "b".into(),
            info: None
        }
    );
    // A file named fomod is not the folder.
    assert_eq!(
        layout(&list(&["fomod", "x.esp"])),
        Layout::Plain {
            data: String::new(),
            info: None
        }
    );
}

#[test]
fn archives_without_an_xml_installer() {
    let script = list(&[
        "Mod/fomod/script.cs",
        "Mod/fomod/info.xml",
        "Mod/Data/Textures/a.dds",
        "Mod/Data/Mod.esp",
    ]);
    assert_eq!(
        layout(&script),
        Layout::Script {
            prefix: "Mod".into(),
            script: "Mod/fomod/script.cs".into(),
            data: Some("Mod/Data".into()),
        }
    );
    // Only info.xml in fomod: plain, with its details.
    let info_only = list(&["fomod/info.xml", "Meshes/x.nif", "readme.txt"]);
    assert_eq!(
        layout(&info_only),
        Layout::Plain {
            data: String::new(),
            info: Some("fomod/info.xml".into()),
        }
    );
    let wrapped = list(&["Pack 1.0/Optional/readme.txt", "Pack 1.0/Textures/a.dds"]);
    assert_eq!(data_folder(&wrapped), Some("Pack 1.0".into()));
    assert_eq!(
        layout(&list(&["readme.txt", "pics/a.png"])),
        Layout::Unknown
    );
}

// ------------------------------------------------------------------------------------------------
// Group types, option types and the defaults

const GROUPS: &str = r#"
<config><moduleName>Groups</moduleName><installSteps><installStep name="One">
<optionalFileGroups order="Explicit">
  <group name="exactly" type="SelectExactlyOne"><plugins order="Explicit">
    <plugin name="e1"><description/><files><file source="e1.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
    <plugin name="e2"><description/><files><file source="e2.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  </plugins></group>
  <group name="atmost" type="SelectAtMostOne"><plugins order="Explicit">
    <plugin name="m1"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
    <plugin name="m2"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  </plugins></group>
  <group name="atleast" type="SelectAtLeastOne"><plugins order="Explicit">
    <plugin name="l1"><description/><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
    <plugin name="l2"><description/><typeDescriptor><type name="NotUsable"/></typeDescriptor></plugin>
    <plugin name="l3"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  </plugins></group>
  <group name="all" type="SelectAll"><plugins order="Explicit">
    <plugin name="a1"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
    <plugin name="a2"><description/><typeDescriptor><type name="NotUsable"/></typeDescriptor></plugin>
    <plugin name="a3"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  </plugins></group>
  <group name="any" type="SelectAny"><plugins order="Explicit">
    <plugin name="y1"><description/><typeDescriptor><type name="Required"/></typeDescriptor></plugin>
    <plugin name="y2"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  </plugins></group>
</optionalFileGroups></installStep></installSteps></config>"#;

#[test]
fn each_group_type_starts_with_its_defaults() {
    let c = config(GROUPS);
    let f = Facts::default();
    let s = Session::new(&c, &f).unwrap();
    let view = s.view(0);
    assert_eq!(
        selected(&view),
        [vec!["e1"], vec![], vec!["l1"], vec!["a1", "a3"], vec!["y1"]]
    );
    let kinds: Vec<GroupType> = view.groups.iter().map(|g| g.kind).collect();
    assert_eq!(
        kinds,
        [
            GroupType::SelectExactlyOne,
            GroupType::SelectAtMostOne,
            GroupType::SelectAtLeastOne,
            GroupType::SelectAll,
            GroupType::SelectAny
        ]
    );
}

#[test]
fn each_group_type_keeps_its_rule() {
    let c = config(GROUPS);
    let f = Facts::default();
    let mut s = Session::new(&c, &f).unwrap();
    // exactly one
    assert!(s.choose(0, 0, &[]).unwrap_err().0.contains("exactly one"));
    assert!(s.choose(0, 0, &[0, 1]).is_err());
    s.choose(0, 0, &[1]).unwrap();
    // at most one
    s.choose(0, 1, &[]).unwrap();
    s.choose(0, 1, &[1]).unwrap();
    assert!(s.choose(0, 1, &[0, 1]).unwrap_err().0.contains("or none"));
    // at least one; a NotUsable option can't be chosen
    assert!(s.choose(0, 2, &[]).unwrap_err().0.contains("at least one"));
    assert!(s
        .choose(0, 2, &[1])
        .unwrap_err()
        .0
        .contains("can't be chosen"));
    s.choose(0, 2, &[0, 2]).unwrap();
    // all: every usable one
    assert!(s.choose(0, 3, &[0]).unwrap_err().0.contains("every option"));
    s.choose(0, 3, &[0, 2]).unwrap();
    // any, but a Required one stays
    assert!(s.choose(0, 4, &[1]).unwrap_err().0.contains("required"));
    s.choose(0, 4, &[0, 1]).unwrap();
    // an option that is not there
    assert!(s.choose(0, 4, &[7]).is_err());
    assert_eq!(
        selected(&s.view(0)),
        [
            vec!["e2"],
            vec!["m2"],
            vec!["l1", "l3"],
            vec!["a1", "a3"],
            vec!["y1", "y2"]
        ]
    );
    let plan = s.plan_ok(&list(&["e1.esp", "e2.esp"]), "");
    assert_eq!(copies(&plan), ["e2.esp <- e2.esp"]);
}

#[test]
fn groups_are_fixed_up_as_the_managers_do() {
    let text = r#"<config><installSteps><installStep name="s"><optionalFileGroups order="Explicit">
      <group name="single exactly" type="SelectExactlyOne"><plugins><plugin name="o"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group>
      <group name="single atmost" type="SelectAtMostOne"><plugins><plugin name="o"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group>
      <group name="two required" type="SelectExactlyOne"><plugins>
        <plugin name="r1"><description/><typeDescriptor><type name="Required"/></typeDescriptor></plugin>
        <plugin name="r2"><description/><typeDescriptor><type name="Required"/></typeDescriptor></plugin>
        <plugin name="r3"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group>
      <group name="two recommended" type="SelectAtMostOne"><plugins>
        <plugin name="c1"><description/><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
        <plugin name="c2"><description/><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin></plugins></group>
    </optionalFileGroups></installStep></installSteps></config>"#;
    let c = config(text);
    let f = Facts::default();
    let s = Session::new(&c, &f).unwrap();
    let view = s.view(0);
    let kinds: Vec<GroupType> = view.groups.iter().map(|g| g.kind).collect();
    assert_eq!(
        kinds,
        [
            GroupType::SelectAll,
            GroupType::SelectAny,
            GroupType::SelectAtLeastOne,
            GroupType::SelectAtMostOne
        ]
    );
    // In a one-choice group the first Recommended is the default (Mod Organizer 2).
    assert_eq!(
        selected(&view),
        [vec!["o"], vec![], vec!["r1", "r2"], vec!["c1"]]
    );
}

#[test]
fn orders_sort_by_name_or_keep_the_written_order() {
    let text = r#"<config><installSteps order="Descending">
      <installStep name="A"><optionalFileGroups><group name="g" type="SelectAny"><plugins order="Descending">
        <plugin name="a"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="c"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="b"><description/><typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep>
      <installStep name="C"><optionalFileGroups><group name="z" type="SelectAny"><plugins/></group><group name="y" type="SelectAny"><plugins/></group></optionalFileGroups></installStep>
      <installStep name="B"><optionalFileGroups order="Explicit"><group name="z" type="SelectAny"><plugins/></group><group name="y" type="SelectAny"><plugins/></group></optionalFileGroups></installStep>
    </installSteps></config>"#;
    let c = config(text);
    let names: Vec<&str> = c.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["C", "B", "A"]);
    let groups =
        |i: usize| -> Vec<&str> { c.steps[i].groups.iter().map(|g| g.name.as_str()).collect() };
    // No order attribute: Ascending, the schema's default.
    assert_eq!(groups(0), ["y", "z"]);
    assert_eq!(groups(1), ["z", "y"]);
    let plugins: Vec<&str> = c.steps[2].groups[0]
        .plugins
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(plugins, ["c", "b", "a"]);
}

// ------------------------------------------------------------------------------------------------
// Flags, visibility and conditions

const FLAGS: &str = r#"<config><installSteps order="Explicit">
  <installStep name="First"><optionalFileGroups><group name="g" type="SelectExactlyOne"><plugins order="Explicit">
    <plugin name="Red"><description/><files><file source="red.esp"/></files><conditionFlags><flag name="colour">red</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
    <plugin name="Blue"><description/><conditionFlags><flag name="colour">blue</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
  </plugins></group></optionalFileGroups></installStep>
  <installStep name="Only when red"><visible><flagDependency flag="colour" value="red"/></visible>
    <optionalFileGroups><group name="g" type="SelectAny"><plugins>
      <plugin name="Darker"><description/><files><file source="dark.esp"/></files><conditionFlags><flag name="colour">dark red</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
    </plugins></group></optionalFileGroups></installStep>
  <installStep name="Only when no shade"><visible><flagDependency flag="shade" value=""/></visible>
    <optionalFileGroups><group name="g" type="SelectAny"><plugins>
      <plugin name="Shade"><description/><conditionFlags><flag name="shade">on</flag></conditionFlags><typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
    </plugins></group></optionalFileGroups></installStep>
</installSteps>
<conditionalFileInstalls><patterns>
  <pattern><dependencies operator="Or"><flagDependency flag="colour" value="dark red"/><flagDependency flag="colour" value="blue"/></dependencies>
    <files><file source="extra.esp"/></files></pattern>
  <pattern><dependencies><flagDependency flag="shade" value="on"/></dependencies><files><file source="shade.esp"/></files></pattern>
</patterns></conditionalFileInstalls></config>"#;

#[test]
fn flags_decide_which_steps_show_and_a_later_step_overrides() {
    let c = config(FLAGS);
    let f = Facts::default();
    let listing = list(&["red.esp", "dark.esp", "extra.esp", "shade.esp"]);
    let mut s = Session::new(&c, &f).unwrap();
    // Red is the default (first of an ExactlyOne group): the red step shows.
    assert_eq!(s.next_step(None), Some(0));
    assert_eq!(s.next_step(Some(0)), Some(1));
    // A flag never set equals "": the third step shows, and its Recommended option sets shade.
    assert_eq!(s.next_step(Some(1)), Some(2));
    assert_eq!(s.previous_step(2), Some(1));
    assert_eq!(
        copies(&s.plan_ok(&listing, "")),
        ["red.esp <- red.esp", "shade.esp <- shade.esp"]
    );
    // Darker sets colour again: the later step wins, and the Or pattern installs extra.esp.
    s.choose(1, 0, &[0]).unwrap();
    assert_eq!(s.flags()["colour"], "dark red");
    assert_eq!(
        copies(&s.plan_ok(&listing, "")),
        [
            "dark.esp <- dark.esp",
            "extra.esp <- extra.esp",
            "red.esp <- red.esp",
            "shade.esp <- shade.esp"
        ]
    );
    // Blue hides the red step: its choice (Darker) counts for nothing now.
    s.choose(0, 0, &[1]).unwrap();
    assert_eq!(s.next_step(Some(0)), Some(2));
    assert_eq!(s.flags()["colour"], "blue");
    assert_eq!(
        copies(&s.plan_ok(&listing, "")),
        ["extra.esp <- extra.esp", "shade.esp <- shade.esp"]
    );
    // Leaving Shade out: the flag stays unset.
    s.choose(2, 0, &[]).unwrap();
    assert!(!s.flags().contains_key("shade"));
    assert_eq!(copies(&s.plan_ok(&listing, "")), ["extra.esp <- extra.esp"]);
}

#[test]
fn conditions_on_plugins_versions_and_extenders() {
    let f = Facts {
        plugins: vec![
            ("FalloutNV.esm".into(), FileState::Active),
            ("DeadMoney.esm".into(), FileState::Inactive),
        ],
        game_version: Some("1.4.0.525".into()),
        extenders: vec![("nvse".into(), "6.3.2".into())],
    };
    let flags = std::collections::BTreeMap::new();
    let file = |name: &str, state| Condition::File {
        file: name.into(),
        state,
    };
    assert!(holds(&file("falloutnv.ESM", FileState::Active), &flags, &f));
    assert!(holds(
        &file("DeadMoney.esm", FileState::Inactive),
        &flags,
        &f
    ));
    assert!(holds(
        &file("HonestHearts.esm", FileState::Missing),
        &flags,
        &f
    ));
    assert!(!holds(
        &file("HonestHearts.esm", FileState::Active),
        &flags,
        &f
    ));
    let game = |v: &str| Condition::Game { version: v.into() };
    assert!(holds(&game("1.4"), &flags, &f));
    assert!(holds(&game("1.4.0.525"), &flags, &f));
    assert!(!holds(&game("1.4.0.600"), &flags, &f));
    let nvse = |v: &str| Condition::Extender {
        name: "NVSE".into(),
        version: v.into(),
    };
    assert!(holds(&nvse("6.1"), &flags, &f));
    assert!(!holds(&nvse("6.4"), &flags, &f));
    assert!(!holds(
        &Condition::Extender {
            name: "fose".into(),
            version: "1".into()
        },
        &flags,
        &f
    ));
    assert!(holds(&Condition::All(vec![]), &flags, &f));
    assert!(!holds(&Condition::Any(vec![]), &flags, &f));
    assert!(holds(
        &Condition::Any(vec![game("9"), Condition::All(vec![game("1"), nvse("6")])]),
        &flags,
        &f
    ));
    assert!(version_at_least("1.10", "1.9"));
    assert!(!version_at_least("1.9", "1.10"));
    assert!(version_at_least("2.0b", "2"));
}

// ------------------------------------------------------------------------------------------------
// Files: destinations, folders, priorities, missing sources

#[test]
fn destinations_follow_the_managers_rules() {
    let text = r#"<config><requiredInstallFiles>
      <file source="a\x.esp"/>
      <file source="a\y.esp" destination=""/>
      <file source="a\z.esp" destination="sub/"/>
      <file source="a\w.esp" destination="renamed.esp"/>
      <folder source="tex" destination="textures\pack"/>
      <folder source="loose/" destination=""/>
      <file source="nowhere.esp"/>
      <folder source="nofolder"/>
      <file source="a\x.esp" destination="..\..\evil.esp"/>
      <file source="a\x.esp" destination="C:\Windows\evil.dll"/>
      <file source="a\x.esp" destination="\abs.esp"/>
      <file source="" destination="ignored"/>
    </requiredInstallFiles></config>"#;
    let c = config(text);
    let f = Facts::default();
    let s = Session::new(&c, &f).unwrap();
    let listing = list(&[
        "Pack/a/x.esp",
        "Pack\\a\\Y.esp",
        "Pack/a/z.esp",
        "Pack/a/w.esp",
        "Pack/Tex/One/1.dds",
        "Pack/tex/2.DDS",
        "Pack/loose/meshes/m.nif",
        "Pack/loose/../../outside.txt",
    ]);
    let plan = s.plan_ok(&listing, "Pack");
    assert_eq!(
        copies(&plan),
        [
            "a/x.esp <- Pack/a/x.esp",
            "meshes/m.nif <- Pack/loose/meshes/m.nif",
            "renamed.esp <- Pack/a/w.esp",
            "sub/z.esp <- Pack/a/z.esp",
            "textures/pack/2.DDS <- Pack/tex/2.DDS",
            "textures/pack/One/1.dds <- Pack/Tex/One/1.dds",
            "Y.esp <- Pack\\a\\Y.esp",
        ]
    );
    assert_eq!(plan.missing, ["nowhere.esp", "nofolder"]);
    assert_eq!(
        plan.refused,
        [
            "../../outside.txt",
            "..\\..\\evil.esp",
            "C:\\Windows\\evil.dll",
            "\\abs.esp"
        ]
    );
}

#[test]
fn priority_then_document_order_decides_a_clash() {
    let text = r#"<config>
      <requiredInstallFiles><file source="req.dds" destination="t.dds"/><file source="req.dds" destination="u.dds" priority="9"/></requiredInstallFiles>
      <installSteps><installStep name="s"><optionalFileGroups><group name="g" type="SelectAll"><plugins><plugin name="p"><description/>
        <files><file source="opt.dds" destination="t.dds"/><file source="opt.dds" destination="u.dds"/>
               <file source="opt.dds" destination="v.dds" priority="2"/><file source="late.dds" destination="v.dds" priority="1"/>
               <file source="first.dds" destination="w.dds"/><file source="second.dds" destination="w.dds"/></files>
        <typeDescriptor><type name="Optional"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep></installSteps>
      <conditionalFileInstalls><patterns><pattern><dependencies/><files><file source="cond.dds" destination="t.dds"/></files></pattern></patterns></conditionalFileInstalls>
    </config>"#;
    let c = config(text);
    let f = Facts::default();
    let s = Session::new(&c, &f).unwrap();
    let listing = list(&[
        "req.dds",
        "opt.dds",
        "late.dds",
        "cond.dds",
        "first.dds",
        "second.dds",
    ]);
    let plan = s.plan_ok(&listing, "");
    assert_eq!(
        copies(&plan),
        [
            // Equal priority: conditional (last in the document) over option over required.
            "t.dds <- cond.dds",
            // The required file's priority 9 beats the option's 0.
            "u.dds <- req.dds",
            // Priority 2 beats a later priority 1.
            "v.dds <- opt.dds",
            // Equal priority inside one option: the later element.
            "w.dds <- second.dds",
        ]
    );
}

#[test]
fn always_install_and_install_if_usable() {
    let text = r#"<config><installSteps><installStep name="s"><optionalFileGroups><group name="g" type="SelectAny"><plugins order="Explicit">
      <plugin name="off usable"><description/><files>
        <file source="always.esp" alwaysInstall="true"/><file source="usable.esp" installIfUsable="True"/><file source="plain.esp"/></files>
        <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      <plugin name="off unusable"><description/><files>
        <file source="always2.esp" alwaysInstall="1"/><file source="usable2.esp" installIfUsable="true"/></files>
        <typeDescriptor><type name="NotUsable"/></typeDescriptor></plugin>
    </plugins></group></optionalFileGroups></installStep></installSteps></config>"#;
    let c = config(text);
    let f = Facts::default();
    let s = Session::new(&c, &f).unwrap();
    assert_eq!(selected(&s.view(0)), [Vec::<&str>::new()]);
    let listing = list(&[
        "always.esp",
        "usable.esp",
        "plain.esp",
        "always2.esp",
        "usable2.esp",
    ]);
    assert_eq!(
        copies(&s.plan_ok(&listing, "")),
        [
            "always.esp <- always.esp",
            "always2.esp <- always2.esp",
            "usable.esp <- usable.esp"
        ]
    );
}

// ------------------------------------------------------------------------------------------------
// Older schema versions

#[test]
fn a_version_one_installer_reads() {
    let text = r#"<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="ModConfig1.0.xsd">
      <moduleName>Old</moduleName>
      <moduleDependancies><dependancy file="FalloutNV.esm" state="Active"/><moduleFileDependancy file="DeadMoney.esm"/><falloutDependancy version="1.0"/><fommDependancy version="0.12"/></moduleDependancies>
      <requiredInstallFiles><file source="base.esp" alwaysInstall="false" installIfUsable="false" priority="0"/></requiredInstallFiles>
      <optionalFileGroups>
        <group name="Zeta" type="SelectAny"><plugins>
          <plugin name="b"><description>second</description><files><file source="b.esp"/></files>
            <typeDescriptor><dependancyType><defaultType name="NotUsable"/><patterns><pattern>
              <dependancies operator="And"><dependancy file="HonestHearts.esm" state="Active"/></dependancies><type name="Recommended"/>
            </pattern></patterns></dependancyType></typeDescriptor></plugin>
          <plugin name="a"><description>first</description><files><file source="a.esp"/></files><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        </plugins></group>
        <group name="Alpha" type="SelectAny"><plugins/></group>
      </optionalFileGroups>
    </config>"#;
    let c = config(text);
    assert_eq!(c.schema, Some((1, 0)));
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    assert_eq!(c.steps.len(), 1);
    // 1.0: groups as written, options by name.
    let groups: Vec<&str> = c.steps[0].groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(groups, ["Zeta", "Alpha"]);
    let plugins: Vec<&str> = c.steps[0].groups[0]
        .plugins
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(plugins, ["a", "b"]);
    let without = facts(&[("FalloutNV.esm", FileState::Active)]);
    assert!(Session::new(&c, &without)
        .unwrap_err()
        .0
        .contains("DeadMoney.esm switched on"));
    let with = facts(&[
        ("FalloutNV.esm", FileState::Active),
        ("DeadMoney.esm", FileState::Active),
        ("HonestHearts.esm", FileState::Active),
    ]);
    let s = Session::new(&c, &with).unwrap();
    assert_eq!(s.view(0).groups[0].options[1].kind, PluginType::Recommended);
    assert_eq!(
        copies(&s.plan_ok(&list(&["base.esp", "a.esp", "b.esp"]), "")),
        ["b.esp <- b.esp", "base.esp <- base.esp"]
    );
}

#[test]
fn unknown_elements_are_left_out_with_a_warning() {
    let text = r#"<config><moduleName>W</moduleName><extra/>
      <requiredInstallFiles><file source="a.esp"/><thing source="b"/></requiredInstallFiles>
      <installSteps><installStep name="s"><visible><frobDependency/><flagDependency flag="f" value=""/></visible>
      <optionalFileGroups><group name="g" type="PickSome"><plugins><plugin name="p"><description/><typeDescriptor><type name="Maybe"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep></installSteps>
    </config>"#;
    let c = config(text);
    assert_eq!(c.warnings.len(), 5, "{:#?}", c.warnings);
    assert_eq!(c.steps[0].groups[0].kind, GroupType::SelectAny);
    assert_eq!(
        c.steps[0].groups[0].plugins[0].kind,
        TypeDescriptor::Fixed(PluginType::Optional)
    );
    // The unknown condition is skipped, the step still shows.
    let f = Facts::default();
    assert_eq!(Session::new(&c, &f).unwrap().next_step(None), Some(0));
}

// ------------------------------------------------------------------------------------------------
// The two shapes met most in New Vegas mods (written for this test)

/// A texture pack: required core textures; a resolution step whose choice sets a flag; an extras
/// step shown only at the high resolution; the textures themselves chosen by conditional
/// installs on the flags, one with a priority so it wins over the resolution's file. Saved as
/// UTF-16 with a byte order mark and wrapped in one folder, as such packs come.
const TEXTURE_PACK: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://qconsulting.ca/fo3/ModConfig5.0.xsd">
  <moduleName>Wasteland Textures</moduleName>
  <requiredInstallFiles><folder source="core" destination="textures\core"/></requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Resolution">
      <optionalFileGroups order="Explicit">
        <group name="Size" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="2K"><description>Sharper, needs more memory.</description><image path="fomod\2k.png"/>
              <conditionFlags><flag name="res">2k</flag></conditionFlags>
              <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
            <plugin name="1K"><description>Lighter.</description>
              <conditionFlags><flag name="res">1k</flag></conditionFlags>
              <typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
    <installStep name="Extras">
      <visible><flagDependency flag="res" value="2k"/></visible>
      <optionalFileGroups><group name="Extras" type="SelectAny"><plugins>
        <plugin name="Parallax"><description>Raised bricks.</description>
          <conditionFlags><flag name="parallax">on</flag></conditionFlags>
          <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group></optionalFileGroups>
    </installStep>
  </installSteps>
  <conditionalFileInstalls><patterns>
    <pattern><dependencies><flagDependency flag="res" value="2k"/></dependencies>
      <files><folder source="2k" destination="textures"/></files></pattern>
    <pattern><dependencies><flagDependency flag="res" value="1k"/></dependencies>
      <files><folder source="1k" destination="textures"/></files></pattern>
    <pattern><dependencies operator="And"><flagDependency flag="res" value="2k"/><flagDependency flag="parallax" value="on"/></dependencies>
      <files><folder source="parallax" destination="textures" priority="1"/></files></pattern>
  </patterns></conditionalFileInstalls>
</config>"#;

#[test]
fn a_texture_pack_installs_by_its_flags() {
    let bytes = utf16le_bom(TEXTURE_PACK);
    let listing = list(&[
        "Wasteland Textures 1.0/fomod/ModuleConfig.xml",
        "Wasteland Textures 1.0/fomod/2k.png",
        "Wasteland Textures 1.0/core/sky.dds",
        "Wasteland Textures 1.0/1k/rock.dds",
        "Wasteland Textures 1.0/2k/rock.dds",
        "Wasteland Textures 1.0/2k/sand.dds",
        "Wasteland Textures 1.0/parallax/rock.dds",
    ]);
    let Layout::Installer { prefix, config, .. } = layout(&listing) else {
        panic!("no installer found");
    };
    assert_eq!(config, "Wasteland Textures 1.0/fomod/ModuleConfig.xml");
    let c = parse_config(&bytes).unwrap();
    assert_eq!(c.name, "Wasteland Textures");
    assert_eq!(c.schema, Some((5, 0)));
    let f = facts(&[("FalloutNV.esm", FileState::Active)]);
    let mut s = Session::new(&c, &f).unwrap();
    let first = s.next_step(None).unwrap();
    let view = s.view(first);
    assert_eq!(view.name, "Resolution");
    assert_eq!(
        view.groups[0].options[0].image.as_deref(),
        Some("fomod\\2k.png")
    );
    assert_eq!(selected(&view), [vec!["1K"]]);
    // 1K: the extras step stays hidden.
    assert_eq!(s.next_step(Some(first)), None);
    assert_eq!(
        copies(&s.plan_ok(&listing, &prefix)),
        [
            "textures/core/sky.dds <- Wasteland Textures 1.0/core/sky.dds",
            "textures/rock.dds <- Wasteland Textures 1.0/1k/rock.dds",
        ]
    );
    // 2K, then parallax: the parallax rock wins by its priority.
    s.choose(first, 0, &[0]).unwrap();
    let extras = s.next_step(Some(first)).unwrap();
    assert_eq!(s.view(extras).name, "Extras");
    s.choose(extras, 0, &[0]).unwrap();
    assert_eq!(
        copies(&s.plan_ok(&listing, &prefix)),
        [
            "textures/core/sky.dds <- Wasteland Textures 1.0/core/sky.dds",
            "textures/rock.dds <- Wasteland Textures 1.0/parallax/rock.dds",
            "textures/sand.dds <- Wasteland Textures 1.0/2k/sand.dds",
        ]
    );
}

/// A weapon mod: it needs the game's master; a required plugin and meshes; one step with DLC
/// patches whose type depends on the player's DLC (recommended when on, usable when present but
/// off, not usable when missing) and a choice of sounds. No `order`: shown by name.
const WEAPON_MOD: &str = r#"<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://qconsulting.ca/fo3/ModConfig4.0.xsd">
  <moduleName>Gun Pack</moduleName>
  <moduleDependencies operator="And"><fileDependency file="FalloutNV.esm" state="Active"/></moduleDependencies>
  <requiredInstallFiles>
    <file source="Main\GunPack.esp" destination="GunPack.esp"/>
    <folder source="Main\Meshes" destination="Meshes"/>
  </requiredInstallFiles>
  <installSteps>
    <installStep name="Options">
      <optionalFileGroups>
        <group name="Sounds" type="SelectAtMostOne"><plugins>
          <plugin name="Quiet"><description>Softer shots.</description>
            <files><folder source="Sounds\Quiet" destination="Sound\fx\gunpack"/></files>
            <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
          <plugin name="Loud"><description>Louder shots.</description>
            <files><folder source="Sounds\Loud" destination="Sound\fx\gunpack"/></files>
            <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        </plugins></group>
        <group name="DLC patches" type="SelectAny"><plugins>
          <plugin name="Honest Hearts"><description>Adds the guns to Zion.</description>
            <files><file source="Patches\GunPack - HH.esp" destination=""/></files>
            <typeDescriptor><dependencyType><defaultType name="NotUsable"/><patterns>
              <pattern><dependencies><fileDependency file="HonestHearts.esm" state="Active"/></dependencies><type name="Recommended"/></pattern>
              <pattern><dependencies><fileDependency file="HonestHearts.esm" state="Inactive"/></dependencies><type name="CouldBeUsable"/></pattern>
            </patterns></dependencyType></typeDescriptor></plugin>
          <plugin name="Dead Money"><description>Adds the guns to the Sierra Madre.</description>
            <files><file source="Patches\GunPack - DM.esp" destination=""/></files>
            <typeDescriptor><dependencyType><defaultType name="NotUsable"/><patterns>
              <pattern><dependencies><fileDependency file="DeadMoney.esm" state="Active"/></dependencies><type name="Recommended"/></pattern>
              <pattern><dependencies><fileDependency file="DeadMoney.esm" state="Inactive"/></dependencies><type name="CouldBeUsable"/></pattern>
            </patterns></dependencyType></typeDescriptor></plugin>
          <plugin name="Lonesome Road"><description>Adds the guns to the Divide.</description>
            <files><file source="Patches\GunPack - LR.esp" destination=""/></files>
            <typeDescriptor><dependencyType><defaultType name="NotUsable"/><patterns>
              <pattern><dependencies><fileDependency file="LonesomeRoad.esm" state="Active"/></dependencies><type name="Recommended"/></pattern>
            </patterns></dependencyType></typeDescriptor></plugin>
        </plugins></group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
</config>"#;

#[test]
fn a_weapon_mod_offers_the_patches_for_the_dlc_there() {
    let c = config(WEAPON_MOD);
    assert_eq!(c.schema, Some((4, 0)));
    let listing = list(&[
        "fomod/ModuleConfig.xml",
        "Main/GunPack.esp",
        "Main/Meshes/Weapons/gun.nif",
        "Sounds/Quiet/shot.wav",
        "Sounds/Loud/shot.wav",
        "Patches/GunPack - HH.esp",
        "Patches/GunPack - DM.esp",
        "Patches/GunPack - LR.esp",
    ]);
    assert!(Session::new(&c, &Facts::default())
        .unwrap_err()
        .0
        .contains("FalloutNV.esm switched on"));
    let f = facts(&[
        ("FalloutNV.esm", FileState::Active),
        ("HonestHearts.esm", FileState::Active),
        ("DeadMoney.esm", FileState::Inactive),
    ]);
    let mut s = Session::new(&c, &f).unwrap();
    let view = s.view(0);
    let groups: Vec<&str> = view.groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(groups, ["DLC patches", "Sounds"]);
    let patches: Vec<(&str, PluginType)> = view.groups[0]
        .options
        .iter()
        .map(|o| (o.name.as_str(), o.kind))
        .collect();
    assert_eq!(
        patches,
        [
            ("Dead Money", PluginType::CouldBeUsable),
            ("Honest Hearts", PluginType::Recommended),
            ("Lonesome Road", PluginType::NotUsable),
        ]
    );
    assert_eq!(selected(&view), [vec!["Honest Hearts"], vec![]]);
    assert!(s.choose(0, 0, &[2]).is_err());
    s.choose(0, 0, &[0, 1]).unwrap();
    s.choose(0, 1, &[0]).unwrap();
    assert_eq!(
        copies(&s.plan_ok(&listing, "")),
        [
            "GunPack - DM.esp <- Patches/GunPack - DM.esp",
            "GunPack - HH.esp <- Patches/GunPack - HH.esp",
            "GunPack.esp <- Main/GunPack.esp",
            "Meshes/Weapons/gun.nif <- Main/Meshes/Weapons/gun.nif",
            "Sound/fx/gunpack/shot.wav <- Sounds/Loud/shot.wav",
        ]
    );
}

#[test]
fn steps_without_an_order_are_shown_by_name() {
    let text = r#"<config><installSteps>
      <installStep name="Second"><optionalFileGroups><group name="g" type="SelectAny"><plugins/></group></optionalFileGroups></installStep>
      <installStep name="First"><optionalFileGroups><group name="g" type="SelectAny"><plugins/></group></optionalFileGroups></installStep>
    </installSteps></config>"#;
    let names: Vec<String> = config(text).steps.into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["First", "Second"]);
}

/// An answered step whose options change type when an earlier answer changes: an option that
/// became Required is in, one that became NotUsable is out, without answering the step again.
#[test]
fn an_answer_follows_types_changed_by_an_earlier_step() {
    let text = r#"<config><installSteps order="Explicit">
      <installStep name="Mode"><optionalFileGroups><group name="g" type="SelectExactlyOne"><plugins order="Explicit">
        <plugin name="Light"><description/><conditionFlags><flag name="mode">light</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="Full"><description/><conditionFlags><flag name="mode">full</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group></optionalFileGroups></installStep>
      <installStep name="Parts"><optionalFileGroups><group name="g" type="SelectAny"><plugins order="Explicit">
        <plugin name="Core"><description/><files><file source="core.esp"/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns>
          <pattern><dependencies><flagDependency flag="mode" value="full"/></dependencies><type name="Required"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
        <plugin name="Lite patch"><description/><files><file source="lite.esp"/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns>
          <pattern><dependencies><flagDependency flag="mode" value="full"/></dependencies><type name="NotUsable"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
      </plugins></group></optionalFileGroups></installStep>
    </installSteps></config>"#;
    let c = config(text);
    let f = Facts::default();
    let listing = list(&["core.esp", "lite.esp"]);
    let mut s = Session::new(&c, &f).unwrap();
    s.choose(1, 0, &[1]).unwrap();
    assert_eq!(copies(&s.plan_ok(&listing, "")), ["lite.esp <- lite.esp"]);
    s.choose(0, 0, &[1]).unwrap();
    assert_eq!(selected(&s.view(1)), [vec!["Core"]]);
    assert_eq!(copies(&s.plan_ok(&listing, "")), ["core.esp <- core.esp"]);
}

// ------------------------------------------------------------------------------------------------
// Hostile and odd downloads (reviews of #720): every count from the download has a cap that
// refuses in words; every lookup is indexed. Sizes here would take many seconds if any of the
// work were quadratic; each test asserts a bound well under that.

/// A bound on the time a test's work may take: quadratic work at these sizes takes far longer.
fn quick(started: std::time::Instant) {
    assert!(
        started.elapsed().as_secs() < 10,
        "took {:?}",
        started.elapsed()
    );
}

/// Steps of `n` options in one group of type `kind`.
fn one_group(kind: &str, n: usize, plugin: &str) -> String {
    format!(
        "<config><installSteps><installStep name=\"s\"><optionalFileGroups><group name=\"g\" type=\"{kind}\"><plugins>{}</plugins></group></optionalFileGroups></installStep></installSteps></config>",
        plugin.repeat(n)
    )
}

#[test]
fn nesting_past_the_caps_is_refused() {
    let mut text = String::from("<config>");
    text.push_str(&"<a>".repeat(100_000));
    assert_eq!(xml::parse(&text), Err(xml::XmlError::TooDeep));
    assert!(parse_config(text.as_bytes())
        .unwrap_err()
        .0
        .contains("nest"));
    assert_eq!(
        parse_info(text.replace("config", "fomod").as_bytes()),
        Info::default()
    );
    // At the cap: read.
    let mut at_cap = String::new();
    at_cap.push_str(&"<a>".repeat(xml::MAX_DEPTH - 1));
    at_cap.push_str("<b/>");
    assert!(xml::parse(&at_cap).is_ok());
    let mut deep = String::from("<config><moduleDependencies>");
    deep.push_str(&"<dependencies operator=\"Or\">".repeat(200));
    deep.push_str("<flagDependency flag=\"x\" value=\"\"/>");
    let error = parse_config(deep.as_bytes()).unwrap_err();
    assert!(error.0.contains("conditions nest"), "{error}");
}

#[test]
fn long_runs_stay_linear_and_attributes_are_capped() {
    let started = std::time::Instant::now();
    let run = "&".repeat(1 << 20);
    assert_eq!(xml::unescape(&run).len(), 1 << 20);
    let mut tag = String::from("<config");
    for i in 0..=xml::MAX_ATTRIBUTES {
        tag.push_str(&format!(" a{i}=\"x\""));
    }
    tag.push_str("/>");
    assert_eq!(xml::parse(&tag), Err(xml::XmlError::TooManyAttributes));
    let ok = tag.replacen(&format!(" a{}=\"x\"", xml::MAX_ATTRIBUTES), "", 1);
    assert_eq!(
        xml::parse(&ok).unwrap().attributes.len(),
        xml::MAX_ATTRIBUTES
    );
    // Stray `<`s join the text around them: one node.
    let root = xml::parse("<config>x < y < z</config>").unwrap();
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.text(), "x < y < z");
    quick(started);
}

#[test]
fn an_oversized_installer_is_refused_before_reading() {
    let big = vec![b' '; MAX_FILE_BYTES + 1];
    assert!(parse_config(&big).unwrap_err().0.contains("bytes"));
    assert_eq!(parse_info(&big), Info::default());
}

/// Steps over the cap refuse the installer, through either way of writing steps; at the cap,
/// 1,024 flag-setting steps walk with one map.
#[test]
fn steps_are_capped_and_walked_once() {
    let step = |i: usize| {
        format!("<installStep name=\"s{i:04}\"><optionalFileGroups><group name=\"g\" type=\"SelectAll\"><plugins><plugin name=\"p\"><description/><conditionFlags><flag name=\"f{i}\">1</flag></conditionFlags><typeDescriptor><type name=\"Optional\"/></typeDescriptor></plugin></plugins></group></optionalFileGroups></installStep>")
    };
    let steps = |n: usize| -> String {
        format!(
            "<config><installSteps order=\"Explicit\">{}</installSteps></config>",
            (0..n).map(step).collect::<String>()
        )
    };
    assert!(parse_config(steps(MAX_STEPS + 1).as_bytes())
        .unwrap_err()
        .0
        .contains("steps"));
    let repeated = format!(
        "<config>{}</config>",
        "<optionalFileGroups/>".repeat(MAX_STEPS + 1)
    );
    assert!(parse_config(repeated.as_bytes())
        .unwrap_err()
        .0
        .contains("steps"));
    let c = config(&steps(MAX_STEPS));
    let f = Facts::default();
    let s = Session::new(&c, &f).unwrap();
    let started = std::time::Instant::now();
    assert_eq!(s.flags().len(), MAX_STEPS);
    assert_eq!(s.view(MAX_STEPS - 1).name, "s1023");
    assert!(s.problems().is_empty());
    quick(started);
}

/// Options over the caps refuse the installer; at the cap, a group of 2,000 chosen options
/// views, checks and plans in time proportional to it (one flag per option, no list search).
#[test]
fn options_are_capped_and_selections_indexed() {
    let bare = "<plugin name=\"p\"><description/><typeDescriptor><type name=\"Required\"/></typeDescriptor></plugin>";
    let too_many = one_group("SelectAll", MAX_OPTIONS_PER_GROUP + 1, bare);
    assert!(parse_config(too_many.as_bytes())
        .unwrap_err()
        .0
        .contains("options"));
    // Over the total in groups each under the cap.
    let group = format!(
        "<group name=\"g\" type=\"SelectAny\"><plugins>{}</plugins></group>",
        bare.repeat(MAX_OPTIONS_PER_GROUP)
    );
    let total = format!(
        "<config><installSteps><installStep name=\"s\"><optionalFileGroups>{}</optionalFileGroups></installStep></installSteps></config>",
        group.repeat(MAX_OPTIONS / MAX_OPTIONS_PER_GROUP + 1)
    );
    assert!(parse_config(total.as_bytes())
        .unwrap_err()
        .0
        .contains("options"));
    let started = std::time::Instant::now();
    let with_file = "<plugin name=\"p\"><description/><files><file source=\"a.esp\"/></files><typeDescriptor><type name=\"Required\"/></typeDescriptor></plugin>";
    let c = config(&one_group("SelectAll", MAX_OPTIONS_PER_GROUP, with_file));
    let f = Facts::default();
    let mut s = Session::new(&c, &f).unwrap();
    let all: Vec<usize> = (0..MAX_OPTIONS_PER_GROUP).collect();
    s.choose(0, 0, &all).unwrap();
    for _ in 0..20 {
        assert_eq!(s.view(0).groups[0].options.len(), MAX_OPTIONS_PER_GROUP);
        assert!(s.problems().is_empty());
        assert_eq!(s.plan_ok(&list(&["a.esp"]), "").copies.len(), 1);
    }
    quick(started);
}

/// Conditions and file items over their caps refuse the installer.
#[test]
fn conditions_and_files_are_capped() {
    let conditions = format!(
        "<config><moduleDependencies>{}</moduleDependencies></config>",
        "<flagDependency flag=\"x\" value=\"\"/>".repeat(MAX_CONDITIONS + 1)
    );
    assert!(parse_config(conditions.as_bytes())
        .unwrap_err()
        .0
        .contains("conditions"));
    let files = format!(
        "<config><requiredInstallFiles>{}</requiredInstallFiles></config>",
        "<file source=\"a\"/>".repeat(MAX_FILE_ITEMS + 1)
    );
    assert!(parse_config(files.as_bytes())
        .unwrap_err()
        .0
        .contains("files and folders"));
}

/// 100,000 files the player has and 10,000 file conditions: each condition is one lookup.
#[test]
fn the_players_files_are_indexed_once() {
    let mut f = Facts::default();
    for i in 0..100_000 {
        f.plugins
            .push((format!("Textures\\Pack\\T{i}.dds"), FileState::Active));
    }
    f.plugins.push(("Needed.esm".into(), FileState::Active));
    let patterns: String = (0..10_000)
        .map(|i| format!("<pattern><dependencies><fileDependency file=\"missing{i}.esp\" state=\"Active\"/></dependencies><type name=\"Recommended\"/></pattern>"))
        .collect();
    let plugin = format!("<plugin name=\"p\"><description/><typeDescriptor><dependencyType><defaultType name=\"Optional\"/><patterns>{patterns}<pattern><dependencies><fileDependency file=\"needed.ESM\" state=\"Active\"/></dependencies><type name=\"Required\"/></pattern></patterns></dependencyType></typeDescriptor></plugin>");
    let c = config(&one_group("SelectAny", 1, &plugin));
    let started = std::time::Instant::now();
    let s = Session::new(&c, &f).unwrap();
    for _ in 0..5 {
        assert_eq!(s.view(0).groups[0].options[0].kind, PluginType::Required);
    }
    quick(started);
}

/// A listing of deep wrapper folders is walked one lookup per level; past the caps it is
/// refused as not a mod's.
#[test]
fn listings_are_capped_and_walked_by_index() {
    let started = std::time::Instant::now();
    let wrap = "w/".repeat(MAX_PATH_PARTS - 3);
    let mut listing: Vec<String> = (0..50_000)
        .map(|i| format!("{wrap}files/{i}.dds"))
        .collect();
    listing.push(format!("{wrap}fomod/ModuleConfig.xml"));
    let Layout::Installer { prefix, .. } = layout(&listing) else {
        panic!("no installer found");
    };
    assert_eq!(prefix.split('/').count(), MAX_PATH_PARTS - 3);
    quick(started);
    let deep = vec![format!("{}x.dds", "d/".repeat(MAX_PATH_PARTS))];
    assert!(crate::layout::layout(&deep).unwrap_err().0.contains("deep"));
    let long = vec![format!("{}.dds", "x".repeat(MAX_PATH_BYTES))];
    assert!(crate::layout::layout(&long)
        .unwrap_err()
        .0
        .contains("bytes"));
    let many: Vec<String> = (0..=MAX_ENTRIES).map(|i| i.to_string()).collect();
    assert!(crate::layout::layout(&many)
        .unwrap_err()
        .0
        .contains("entries"));
    let c = config("<config/>");
    let f = Facts::default();
    let error = Session::new(&c, &f).unwrap().plan(&deep, "").unwrap_err();
    assert!(matches!(error, PlanError::Listing(_)), "{error}");
}

/// Folder entries in a raw listing ("Wrapper/", a bare "Wrapper", an empty "Wrapper/tex/Empty/")
/// neither hide the installer nor become copies.
#[test]
fn folder_entries_in_a_listing_are_not_files() {
    let listing = list(&[
        "Wrapper/",
        "Wrapper",
        "Wrapper/fomod/",
        "Wrapper/fomod/ModuleConfig.xml",
        "Wrapper/tex/",
        "Wrapper/tex/One",
        "Wrapper/tex/Empty/",
        "Wrapper/tex/One/1.dds",
        "Wrapper\\tex\\2.dds",
    ]);
    let Layout::Installer { prefix, .. } = layout(&listing) else {
        panic!("{:?}", layout(&listing));
    };
    assert_eq!(prefix, "Wrapper");
    let c = config(
        r#"<config><requiredInstallFiles><folder source="tex" destination="textures"/></requiredInstallFiles></config>"#,
    );
    let f = Facts::default();
    let plan = Session::new(&c, &f).unwrap().plan_ok(&listing, &prefix);
    assert_eq!(
        copies(&plan),
        [
            "textures/2.dds <- Wrapper\\tex\\2.dds",
            "textures/One/1.dds <- Wrapper/tex/One/1.dds"
        ]
    );
    // Names sorting around the `/` ("Wrapper-2", "Wrapper.old") are other top names.
    let two = list(&["Wrapper/fomod/ModuleConfig.xml", "Wrapper-2/x.esp"]);
    assert!(!matches!(layout(&two), Layout::Installer { .. }));
    let dot = list(&["A/fomod/ModuleConfig.xml", "A.old/x.esp"]);
    assert!(!matches!(layout(&dot), Layout::Installer { .. }));
}

/// Repeated identical file items are resolved once.
#[test]
fn repeated_items_are_resolved_once() {
    let text = format!(
        "<config><requiredInstallFiles>{}</requiredInstallFiles></config>",
        "<folder source=\"tex\" destination=\"textures\"/>".repeat(20_000)
    );
    let c = config(&text);
    let listing: Vec<String> = (0..20_000).map(|i| format!("tex/{i}.dds")).collect();
    let started = std::time::Instant::now();
    let f = Facts::default();
    let plan = Session::new(&c, &f).unwrap().plan_ok(&listing, "");
    assert_eq!(plan.copies.len(), 20_000);
    quick(started);
}

/// An AtLeastOne group of Optional options starts with its first (Mod Organizer 2), and a choice
/// an earlier answer made impossible shows as a problem; no plan is given while it does.
#[test]
fn an_at_least_one_group_is_never_left_empty() {
    let text = r#"<config><installSteps order="Explicit">
      <installStep name="Mode"><optionalFileGroups><group name="m" type="SelectExactlyOne"><plugins order="Explicit">
        <plugin name="Light"><description/><conditionFlags><flag name="mode">light</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
        <plugin name="Full"><description/><conditionFlags><flag name="mode">full</flag></conditionFlags><typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
      </plugins></group></optionalFileGroups></installStep>
      <installStep name="Body"><optionalFileGroups><group name="Body" type="SelectAtLeastOne"><plugins order="Explicit">
        <plugin name="Body A"><description/><files><file source="a.dds"/></files><typeDescriptor><dependencyType><defaultType name="Optional"/><patterns>
          <pattern><dependencies><flagDependency flag="mode" value="full"/></dependencies><type name="NotUsable"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
        <plugin name="Body B"><description/><files><file source="b.dds"/></files><typeDescriptor><type name="CouldBeUsable"/></typeDescriptor></plugin>
      </plugins></group></optionalFileGroups></installStep>
    </installSteps></config>"#;
    let c = config(text);
    let f = Facts::default();
    let listing = list(&["a.dds", "b.dds"]);
    let mut s = Session::new(&c, &f).unwrap();
    assert_eq!(selected(&s.view(1)), [vec!["Body A"]]);
    assert!(s.problems().is_empty());
    assert_eq!(copies(&s.plan_ok(&listing, "")), ["a.dds <- a.dds"]);
    // Unanswered, with Body A not usable: the first CouldBeUsable one.
    s.choose(0, 0, &[1]).unwrap();
    assert_eq!(selected(&s.view(1)), [vec!["Body B"]]);
    assert_eq!(s.chosen(1, 0), [1]);
    // Answered with Body A, then Full: the answer is empty now, a problem, and no plan.
    s.choose(0, 0, &[0]).unwrap();
    s.choose(1, 0, &[0]).unwrap();
    s.choose(0, 0, &[1]).unwrap();
    let problems = s.problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, 1);
    assert!(problems[0].1 .0.contains("at least one"));
    assert_eq!(
        s.plan(&listing, ""),
        Err(PlanError::Choices(problems.clone()))
    );
}

/// Destinations fold as Windows and macOS fold names, and parts no system can hold safely are
/// refused on every system; the device-name boundary is pinned.
#[test]
fn destinations_fold_and_odd_names_are_refused() {
    let text = r#"<config><requiredInstallFiles>
      <file source="x.dds" destination="Textures/Ä.dds"/>
      <file source="y.dds" destination="textures/ä.dds. "/>
      <file source="x.dds" destination="CON.esp"/>
      <file source="x.dds" destination="sub/nul"/>
      <file source="x.dds" destination="Com1.txt"/>
      <file source="x.dds" destination="a:stream"/>
      <file source="x.dds" destination="\\server\share\x"/>
      <file source="x.dds" destination="\\?\C:\x"/>
      <file source="x.dds" destination=" C:/x"/>
      <file source="Ü/É.ESP" destination="console.esp"/>
    </requiredInstallFiles></config>"#;
    let c = config(text);
    let f = Facts::default();
    let plan = Session::new(&c, &f)
        .unwrap()
        .plan_ok(&list(&["x.dds", "y.dds", "ü/é.esp"]), "");
    // One file for the two spellings: the later (y) wins.
    assert_eq!(
        copies(&plan),
        ["console.esp <- ü/é.esp", "textures/ä.dds. <- y.dds"]
    );
    assert_eq!(plan.refused.len(), 7, "{:?}", plan.refused);
    assert!(plan.missing.is_empty());
    for refused in [
        "CON",
        "con.",
        "CON .txt",
        "nul.tar.gz",
        "AUX",
        "prn.dds",
        "COM1",
        "lpt9.x",
        "COM¹",
        "CONIN$",
        "conout$.txt",
    ] {
        assert!(device_name(refused), "{refused}");
    }
    for accepted in [
        "COM10.dds",
        "aux_cable.nif",
        "console.txt",
        "connie.esp",
        "LPT",
        "COM0x",
    ] {
        assert!(!device_name(accepted), "{accepted}");
    }
}

#[test]
fn a_file_dependency_on_a_dll_counts_files_the_player_has() {
    let f = facts(&[("nvse\\plugins\\JIP NVSE.dll", FileState::Active)]);
    let flags = std::collections::BTreeMap::new();
    assert!(holds(
        &Condition::File {
            file: "NVSE/Plugins/jip nvse.dll".into(),
            state: FileState::Active
        },
        &flags,
        &f
    ));
}
