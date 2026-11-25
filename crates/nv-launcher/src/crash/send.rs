//! Sending a crash report to nv-rs's crash service (Sentry, or one that speaks its protocol):
//! one envelope, the event and the report's log as an attachment, posted to the project's
//! envelope endpoint over HTTPS by the system's own `curl` (in Windows 10 from 1803 on, macOS and
//! most Linux systems). No HTTP library: a missing curl or no network leaves the local copy only,
//! said plainly.
//!
//! Where to send comes from the package, not the build: `crash-reports.txt` beside the launcher
//! (the repository's `distribution/crash-reports.txt`, copied by the package scripts) holds the
//! project's DSN, a public client key. Without that file (a build from source, a fork's run)
//! sending is off.
//!
//! Nothing personal is sent: the report is the scrubbed one kept on disk, without an F12
//! report's note and place (they stay on this PC); the event has no user, no IP address field
//! and no computer name, and its tags name only the system, the renderer, the kind of end and
//! the profile.

use super::Report;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The package's file with the crash service's DSN, beside the launcher.
pub const DSN_FILE: &str = "crash-reports.txt";

/// A DSN taken apart: `https://KEY@HOST[/PATH]/PROJECT`.
#[derive(Debug, PartialEq)]
pub struct Dsn {
    pub dsn: String,
    pub key: String,
    /// `https://HOST[/PATH]/api/PROJECT/envelope/`.
    pub endpoint: String,
}

impl Dsn {
    /// A DSN, or none: HTTPS (plain HTTP only to this PC), a key of letters and digits with no
    /// secret part, a host, and a project of letters and digits.
    pub fn parse(dsn: &str) -> Option<Dsn> {
        let dsn = dsn.trim();
        let (scheme, rest) = dsn.split_once("://")?;
        let (key, place) = rest.split_once('@')?;
        let (host_path, project) = place.trim_end_matches('/').rsplit_once('/')?;
        let host = host_path.split('/').next()?;
        let here = host == "localhost"
            || host.starts_with("localhost:")
            || host == "127.0.0.1"
            || host.starts_with("127.0.0.1:");
        let fine = (scheme == "https" || (scheme == "http" && here))
            && !key.is_empty()
            && key.bytes().all(|b| b.is_ascii_alphanumeric())
            && !host.is_empty()
            && !project.is_empty()
            && project.bytes().all(|b| b.is_ascii_alphanumeric());
        fine.then(|| Dsn {
            dsn: dsn.to_string(),
            key: key.to_string(),
            endpoint: format!("{scheme}://{host_path}/api/{project}/envelope/"),
        })
    }
}

/// The DSN of a `crash-reports.txt`: its first line that is not empty or a `#` comment.
pub fn dsn_of(text: &str) -> Option<Dsn> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))?;
    Dsn::parse(line)
}

/// The package's DSN (`crash-reports.txt` in the launcher's folder `program`): none means
/// sending is off.
pub fn package_dsn(program: &Path) -> Option<Dsn> {
    let text = std::fs::read(program.join(DSN_FILE)).ok()?;
    dsn_of(&String::from_utf8_lossy(&text))
}

/// `text` as a JSON string, quotes included.
pub fn json(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_list(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|i| json(i)).collect();
    format!("[{}]", items.join(","))
}

/// 32 lowercase hex digits for an event's id (a version 4 UUID's form), from the system's random
/// keys (std's hasher seeds), the time and the process.
pub fn event_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for (i, half) in bytes.chunks_mut(8).enumerate() {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_usize(i);
        h.write_u32(std::process::id());
        half.copy_from_slice(&h.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The release the event names: nv-rs's version (`BUILD.txt`'s first line's last word) and its
/// commit's first 12 digits: `nv-rs@2026-11-16.1+0123456789ab`.
pub fn release(report: &Report) -> String {
    let field = |k: &str| {
        report
            .build
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.as_str())
    };
    let version = field("nv-rs")
        .filter(|l| l.starts_with("nv-rs "))
        .and_then(|l| l.split_whitespace().last())
        .unwrap_or("source");
    match field("Commit") {
        Some(c) => format!("nv-rs@{version}+{}", c.get(..12).unwrap_or(c)),
        None => format!("nv-rs@{version}"),
    }
}

/// The event: the stop as its exception (the service's title for the group), level fatal, the
/// release, tags to sort by, the system as contexts, the settings and mods as extra data, and
/// the fingerprint that groups the same crash. No user, no IP address, no server name.
pub fn event_json(report: &Report, id: &str) -> String {
    let system = |k: &str| {
        report
            .system
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    let kind = report
        .fingerprint
        .first()
        .map(String::as_str)
        .unwrap_or("exit");
    let what = report.ending.plain(report.status);
    let value = report.ending.line().unwrap_or(&what);
    let secs = report
        .when
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let context: Vec<String> = report
        .system
        .iter()
        .map(|(k, v)| format!("{}:{}", json(k), json(v)))
        .collect();
    let parts = [
        format!("\"event_id\":{}", json(id)),
        format!("\"timestamp\":{secs}"),
        "\"platform\":\"native\",\"level\":\"fatal\",\"logger\":\"nv-launcher\"".to_string(),
        format!("\"release\":{}", json(&release(report))),
        format!(
            "\"exception\":{{\"values\":[{{\"type\":{},\"value\":{},\"mechanism\":{{\"type\":\"nv-rs\",\"handled\":false}}}}]}}",
            json(kind),
            json(value)
        ),
        format!(
            "\"tags\":{{\"os\":{},\"renderer\":{},\"kind\":{},\"profile\":{}}}",
            json(system("System")),
            json(system("Renderer")),
            json(kind),
            json(report.profile)
        ),
        format!(
            "\"contexts\":{{\"os\":{{\"type\":\"os\",\"name\":{}}},\"gpu\":{{\"type\":\"gpu\",\"name\":{}}},\"system\":{{{}}}}}",
            json(system("System")),
            json(system("Graphics card")),
            context.join(",")
        ),
        format!(
            "\"extra\":{{\"what happened\":{},\"settings\":{},\"mods\":{}}}",
            json(&what),
            json_list(&report.settings),
            json_list(&report.mods)
        ),
        format!("\"fingerprint\":{}", json_list(&report.fingerprint)),
    ];
    format!("{{{}}}", parts.join(","))
}

/// The envelope of `report` for `dsn`: its header, the event, and the log as `log.txt`, each
/// item's length in bytes in its header.
pub fn envelope(report: &Report, dsn: &Dsn, id: &str, sent_at: &str) -> Vec<u8> {
    let event = event_json(report, id);
    let mut log = report.log.join("\n");
    log.push('\n');
    let mut out = format!(
        "{{\"event_id\":{},\"dsn\":{},\"sent_at\":{}}}\n",
        json(id),
        json(&dsn.dsn),
        json(sent_at)
    );
    out.push_str(&format!(
        "{{\"type\":\"event\",\"length\":{},\"content_type\":\"application/json\"}}\n{event}\n",
        event.len()
    ));
    out.push_str(&format!(
        "{{\"type\":\"attachment\",\"length\":{},\"content_type\":\"text/plain\",\"filename\":\"log.txt\"}}\n{log}\n",
        log.len()
    ));
    out.into_bytes()
}

/// The system's curl: Windows' own (`System32\curl.exe`) where it is, else the one the system
/// finds by name.
pub fn curl() -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("SystemRoot") {
        let own = PathBuf::from(root).join("System32").join("curl.exe");
        if own.is_file() {
            return own;
        }
    }
    PathBuf::from("curl")
}

/// curl's arguments to post the envelope file `file` to `dsn`'s endpoint: quiet but for errors,
/// an HTTP error a failure, 10 seconds to connect and 30 in all.
pub fn curl_args(dsn: &Dsn, file: &Path) -> Vec<OsString> {
    let auth = format!(
        "X-Sentry-Auth: Sentry sentry_version=7, sentry_key={}, sentry_client=nv-launcher/{}",
        dsn.key,
        env!("CARGO_PKG_VERSION")
    );
    let mut data = OsString::from("@");
    data.push(file);
    let mut args: Vec<OsString> = [
        "--silent",
        "--show-error",
        "--fail",
        "--connect-timeout",
        "10",
        "--max-time",
        "30",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/x-sentry-envelope",
        "-H",
        &auth,
        "--data-binary",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(data);
    args.push(OsString::from(&dsn.endpoint));
    args
}

/// Why curl's status `code` means the report did not go, in plain words.
pub fn curl_failure(code: Option<i32>) -> String {
    match code {
        Some(6) | Some(7) => "the crash service could not be reached (no network?)".into(),
        Some(28) => "the crash service did not answer in time".into(),
        Some(22) => "the crash service refused it".into(),
        Some(35) | Some(60) => "the secure connection to the crash service failed".into(),
        Some(c) => format!("curl ended with status {c}"),
        None => "curl was stopped".into(),
    }
}

/// `report` (kept in `folder`) sent to `dsn` by the program `curl`: its envelope written into
/// the folder as `envelope.txt` (what went, for the player to see), then posted; `sent.txt` notes
/// a success. The event's id, or why it did not go.
pub fn send(report: &Report, folder: &Path, dsn: &Dsn, curl: &Path) -> Result<String, String> {
    let id = event_id();
    let sent_at = format!(
        "{}Z",
        super::utc(std::time::SystemTime::now()).0.replace(' ', "T")
    );
    let file = folder.join("envelope.txt");
    std::fs::write(&file, envelope(report, dsn, &id, &sent_at))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    let out = std::process::Command::new(curl)
        .args(curl_args(dsn, &file))
        .stdin(std::process::Stdio::null())
        .output();
    match out {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err("curl, the program that sends it, is not on this PC".into())
        }
        Err(e) => Err(format!("curl could not be started ({e})")),
        Ok(o) if o.status.success() => {
            let _ = std::fs::write(
                folder.join("sent.txt"),
                format!("Sent to nv-rs's crash service at {sent_at}: event {id}\n"),
            );
            Ok(id)
        }
        Ok(o) => Err(curl_failure(o.status.code())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crash::{consent, deliver, next_step, Ending, Next, Report};
    use std::time::{Duration, SystemTime};

    const DSN: &str = "https://0123456789abcdef0123456789abcdef@o1.ingest.example.org/42";

    fn report() -> Report {
        let ending = Ending::Stop(
            "Game's main stopped in frame 9: open function 00abcdef (in 00a1b2c3 < 00400000)"
                .into(),
        );
        Report {
            fingerprint: crate::crash::fingerprint(&ending),
            ending,
            status: Some(3),
            when: SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000),
            build: vec![
                ("nv-rs", "nv-rs Windows preview 2026-11-16.1".into()),
                ("Commit", "0123456789abcdef0123".into()),
                ("Launcher", "0.1.0".into()),
            ],
            system: vec![
                ("System", "Some OS 1".into()),
                ("Graphics card", "Some \"Card\" (Vulkan)".into()),
                ("Renderer", "wgpu".into()),
            ],
            profile: "game",
            settings: vec!["[nv-rs General] sPreset=fixes".into()],
            mods: Vec::new(),
            f12: None,
            log: vec!["first".into(), "caf\u{e9} at <home>\\x".into()],
        }
    }

    #[test]
    fn a_dsn_names_its_key_and_envelope_endpoint() {
        let d = Dsn::parse(DSN).unwrap();
        assert_eq!(d.key, "0123456789abcdef0123456789abcdef");
        assert_eq!(d.endpoint, "https://o1.ingest.example.org/api/42/envelope/");
        assert_eq!(
            Dsn::parse("https://k@host.example/sub/7/")
                .unwrap()
                .endpoint,
            "https://host.example/sub/api/7/envelope/"
        );
        assert_eq!(
            Dsn::parse("http://k@localhost:9000/1").unwrap().endpoint,
            "http://localhost:9000/api/1/envelope/"
        );
        // A secret key, plain HTTP elsewhere, no key, no project: refused.
        assert_eq!(Dsn::parse("https://k:secret@host/1"), None);
        assert_eq!(Dsn::parse("http://k@host.example/1"), None);
        assert_eq!(Dsn::parse("https://@host/1"), None);
        assert_eq!(Dsn::parse("https://k@host"), None);
        assert_eq!(Dsn::parse("https://k@host/"), None);
        assert_eq!(Dsn::parse("ftp://k@host/1"), None);
    }

    /// The package's file: comments skipped; the repository's own parses (the packages carry it).
    #[test]
    fn the_crash_service_file_is_read_from_the_package() {
        let text = format!("# what this is\n\n{DSN}\nhttps://other@x/1\n");
        assert_eq!(dsn_of(&text), Dsn::parse(DSN));
        assert_eq!(dsn_of("# only a comment\n"), None);
        let shipped = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../distribution/crash-reports.txt"),
        )
        .unwrap();
        let d = dsn_of(&shipped).expect("distribution/crash-reports.txt holds a DSN");
        assert!(d.endpoint.starts_with("https://"));
        assert!(d.endpoint.ends_with("/envelope/"));
        let none = std::env::temp_dir().join(format!("nv-launcher-no-dsn-{}", std::process::id()));
        assert_eq!(package_dsn(&none), None);
    }

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json("a\"b\\c\nd\te\u{1}"), "\"a\\\"b\\\\c\\nd\\te\\u0001\"");
        assert_eq!(json("caf\u{e9}"), "\"caf\u{e9}\"");
    }

    #[test]
    fn an_event_id_is_a_random_uuid_in_32_hex_digits() {
        let (a, b) = (event_id(), event_id());
        assert_eq!(a.len(), 32);
        assert!(a
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(&a[12..13], "4");
        assert!(matches!(&a[16..17], "8" | "9" | "a" | "b"));
        assert_ne!(a, b);
    }

    #[test]
    fn the_release_is_the_version_and_the_commit() {
        assert_eq!(release(&report()), "nv-rs@2026-11-16.1+0123456789ab");
        let mut source = report();
        source.build = vec![("nv-rs", "built from source (no BUILD.txt)".into())];
        assert_eq!(release(&source), "nv-rs@source");
    }

    /// The envelope's three parts, each item's length its bytes (a non-ASCII log line too), the
    /// event's fields, and nothing personal in it: no user, no IP address, no server name.
    #[test]
    fn the_envelope_holds_the_event_and_the_log_with_their_lengths() {
        let r = report();
        let d = Dsn::parse(DSN).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let text = String::from_utf8(envelope(&r, &d, id, "2001-09-09T01:46:40Z")).unwrap();
        let event = event_json(&r, id);
        let log = "first\ncaf\u{e9} at <home>\\x\n";
        let want = format!(
            "{{\"event_id\":\"{id}\",\"dsn\":\"{DSN}\",\"sent_at\":\"2001-09-09T01:46:40Z\"}}\n\
             {{\"type\":\"event\",\"length\":{},\"content_type\":\"application/json\"}}\n{event}\n\
             {{\"type\":\"attachment\",\"length\":{},\"content_type\":\"text/plain\",\"filename\":\"log.txt\"}}\n{log}\n",
            event.len(),
            log.len()
        );
        assert_eq!(text, want);
        // Bytes, not characters: the log's one non-ASCII character is two bytes.
        assert_eq!(log.len(), log.chars().count() + 1);
        for field in [
            "\"level\":\"fatal\"",
            "\"platform\":\"native\"",
            "\"release\":\"nv-rs@2026-11-16.1+0123456789ab\"",
            "\"timestamp\":1000000000",
            "\"type\":\"stop\",\"value\":\"Game's main stopped in frame 9: open function 00abcdef (in 00a1b2c3 < 00400000)\"",
            "\"fingerprint\":[\"stop\",\"open function #\",\"00a1b2c3\",\"00400000\"]",
            "\"gpu\":{\"type\":\"gpu\",\"name\":\"Some \\\"Card\\\" (Vulkan)\"}",
            "\"settings\":[\"[nv-rs General] sPreset=fixes\"]",
        ] {
            assert!(event.contains(field), "{field} not in {event}");
        }
        for personal in ["\"user\"", "ip_address", "server_name", "\"device\""] {
            assert!(!event.contains(personal), "{personal} in {event}");
        }
    }

    /// An F12 report's note and place stay on this PC: not in the envelope, and the question
    /// before sending shows the report without them and says so.
    #[test]
    fn an_f12_reports_note_and_place_are_not_sent() {
        let mut r = report();
        r.f12 = Some(crate::crash::F12 {
            folder: "717".into(),
            note: Some("the door would not open".into()),
            place: Some("Goodsprings, by the saloon".into()),
        });
        let d = Dsn::parse(DSN).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let text = String::from_utf8(envelope(&r, &d, id, "2001-09-09T01:46:40Z")).unwrap();
        let asked = crate::crash::question(&r);
        for f12 in ["door would not open", "Goodsprings", "reports/717"] {
            assert!(!text.contains(f12), "{f12} in {text}");
            assert!(!asked.contains(f12), "{f12} in {asked}");
        }
        assert!(r.text().contains("the door would not open"));
        assert!(
            asked.contains(
                "The note and place of the F12 report made during this run stay on this PC."
            ),
            "{asked}"
        );
        assert!(
            asked.contains("The log's last 2 lines go with it."),
            "{asked}"
        );
    }

    #[test]
    fn curl_is_asked_to_post_the_file_quietly_with_limits() {
        let d = Dsn::parse(DSN).unwrap();
        let args: Vec<String> = curl_args(&d, Path::new("env.txt"))
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args.last(), Some(&d.endpoint));
        assert_eq!(args[args.len() - 2], "@env.txt");
        for want in [
            "--fail",
            "--max-time",
            "--connect-timeout",
            "Content-Type: application/x-sentry-envelope",
        ] {
            assert!(args.iter().any(|a| a == want), "{want}");
        }
        assert!(args.iter().any(|a| a
            .starts_with("X-Sentry-Auth: Sentry sentry_version=7, sentry_key=0123456789abcdef")));
        assert_eq!(
            curl_failure(Some(6)),
            "the crash service could not be reached (no network?)"
        );
        assert_eq!(curl_failure(Some(22)), "the crash service refused it");
        assert_eq!(curl_failure(Some(99)), "curl ended with status 99");
    }

    /// Nothing is sent without the package's crash service file, or without a yes.
    #[test]
    fn only_a_yes_with_a_crash_service_sends() {
        use consent::Consent::*;
        assert_eq!(next_step(Send, true), Next::Send);
        assert_eq!(next_step(Ask, true), Next::Ask);
        for answer in [Never, Unanswered] {
            assert_eq!(next_step(answer, true), Next::Keep);
        }
        for answer in [Send, Ask, Never, Unanswered] {
            assert_eq!(next_step(answer, false), Next::Keep);
        }
    }

    /// A kept report with each answer: never and none send nothing and ask nothing; "ask" asks
    /// with the report shown and a no keeps it; a yes (or "send") with no curl on the system says
    /// so, the envelope that would have gone kept beside the report; without the package's file
    /// nothing is asked or sent.
    #[test]
    fn a_kept_report_goes_as_the_player_answered() {
        let root = std::env::temp_dir().join(format!("nv-launcher-deliver-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("reports").join("crash-x");
        std::fs::create_dir_all(&folder).unwrap();
        let program = root.join("program");
        std::fs::create_dir_all(&program).unwrap();
        let r = report();
        let no_curl = Path::new("nv-rs-no-such-curl-here");
        let asked = std::cell::Cell::new(0);
        let ask = |yes: bool| {
            let asked = &asked;
            move |text: &str| {
                assert!(
                    text.starts_with("The game stopped, and nv-rs kept the crash report below.")
                );
                assert!(text.contains("open function 00abcdef"));
                asked.set(asked.get() + 1);
                yes
            }
        };
        // No crash service file: nothing, whatever the answer.
        consent::write(&root, consent::Consent::Send).unwrap();
        assert_eq!(
            deliver(&r, &folder, &root, &program, no_curl, ask(true)),
            None
        );
        std::fs::write(program.join(DSN_FILE), format!("# a comment\n{DSN}\n")).unwrap();
        for answer in [consent::Consent::Never, consent::Consent::Unanswered] {
            consent::write(&root, answer).unwrap();
            assert_eq!(
                deliver(&r, &folder, &root, &program, no_curl, ask(true)),
                None
            );
        }
        assert_eq!(asked.get(), 0);
        assert!(!folder.join("envelope.txt").exists());
        consent::write(&root, consent::Consent::Ask).unwrap();
        assert_eq!(
            deliver(&r, &folder, &root, &program, no_curl, ask(false)).as_deref(),
            Some("The crash report was not sent: it stays on this PC.")
        );
        assert_eq!(asked.get(), 1);
        assert!(!folder.join("envelope.txt").exists());
        let line = deliver(&r, &folder, &root, &program, no_curl, ask(true)).unwrap();
        assert!(
            line.contains("could not be sent (curl, the program that sends it, is not on this PC)"),
            "{line}"
        );
        assert_eq!(asked.get(), 2);
        assert!(folder.join("envelope.txt").exists());
        assert!(!folder.join("sent.txt").exists());
        consent::write(&root, consent::Consent::Send).unwrap();
        let line = deliver(&r, &folder, &root, &program, no_curl, ask(true)).unwrap();
        assert!(line.contains("not on this PC"), "{line}");
        assert_eq!(asked.get(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// curl's own answer decides (Linux and macOS have `true` and `false` to stand in for it):
    /// a success is noted beside the report, a failure says curl's status.
    #[cfg(unix)]
    #[test]
    fn curls_answer_decides() {
        let folder = std::env::temp_dir().join(format!("nv-launcher-curl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let d = Dsn::parse(DSN).unwrap();
        let id = send(&report(), &folder, &d, Path::new("true")).unwrap();
        let sent = std::fs::read_to_string(folder.join("sent.txt")).unwrap();
        assert!(sent.contains(&id));
        std::fs::remove_file(folder.join("sent.txt")).unwrap();
        assert_eq!(
            send(&report(), &folder, &d, Path::new("false")),
            Err("curl ended with status 1".into())
        );
        assert!(!folder.join("sent.txt").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
