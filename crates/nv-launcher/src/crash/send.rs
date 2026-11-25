//! Sending a crash report to nv-rs's crash service (Sentry, or one that speaks its protocol):
//! one envelope, the event and the report's log as an attachment, posted to the project's
//! envelope endpoint over HTTPS by the system's own `curl` (in Windows 10 from 1803 on, macOS and
//! most Linux systems). No HTTP library: a missing curl or no network leaves the local copy only,
//! said plainly.
//!
//! Where to send comes from the package, not the build: `crash-reports.txt` beside the launcher
//! (the repository's `distribution/crash-reports.txt`, copied by the package scripts into release
//! packages only, never into test builds) holds the project's DSN, a public client key. Without
//! that file (a build from source, a test package) sending is off. A fork's release package sends
//! to nv-rs's project unless the fork changes that file.
//!
//! What is sent is the scrubbed report kept on disk: where the player was in the game goes with
//! it (an F12 report's place, the log's `F12: ` lines and its `hitch:` lines' cells and
//! worldspaces), and so does the note of an F12 report made in that run, both scrubbed like the
//! rest (user and computer names, folders, e-mail and network addresses). Nothing personal is
//! sent: the event has no user, no IP address field and no computer name, and its tags name only
//! the system, the renderer, the kind of end and the profile.

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
    /// secret part, a host of letters, digits, `.` and `-` (a port after `:`), a path of the
    /// same and `/`, and a project of letters and digits.
    pub fn parse(dsn: &str) -> Option<Dsn> {
        let dsn = dsn.trim();
        let (scheme, rest) = dsn.split_once("://")?;
        let (key, place) = rest.split_once('@')?;
        let (host_path, project) = place.trim_end_matches('/').rsplit_once('/')?;
        let (host, path) = host_path.split_once('/').unwrap_or((host_path, ""));
        let (name, port) = host.split_once(':').unwrap_or((host, ""));
        let plain = |s: &str, also: &[u8]| {
            s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || also.contains(&b))
        };
        let here = name == "localhost" || name == "127.0.0.1";
        let fine = (scheme == "https" || (scheme == "http" && here))
            && !key.is_empty()
            && key.bytes().all(|b| b.is_ascii_alphanumeric())
            && !name.is_empty()
            && plain(name, b"")
            && (!host.contains(':')
                || (!port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())))
            && plain(path, b"/")
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
/// release, tags to sort by, the system as contexts, the settings, mods and an F12 report made
/// in the run (its folder, its place, the player's note when there is one) as extra data, and
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
    let mut extra = vec![
        format!("\"what happened\":{}", json(&what)),
        format!("\"settings\":{}", json_list(&report.settings)),
        format!("\"mods\":{}", json_list(&report.mods)),
    ];
    if let Some(f12) = &report.f12 {
        extra.push(format!(
            "\"F12 report\":{}",
            json(&format!("reports/{}", f12.folder))
        ));
        if let Some(place) = &f12.place {
            extra.push(format!("\"F12 place\":{}", json(place)));
        }
        if let Some(note) = &f12.note {
            extra.push(format!("\"F12 note\":{}", json(note)));
        }
    }
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
        format!("\"extra\":{{{}}}", extra.join(",")),
        format!("\"fingerprint\":{}", json_list(&report.fingerprint)),
    ];
    format!("{{{}}}", parts.join(","))
}

/// The envelope of `report` for `dsn`: its header, the event, and the log as `log.txt` (as kept
/// on this PC, every line whole), each item's length in bytes in its header.
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

/// curl's arguments to post the envelope it reads on its input to `dsn`'s endpoint: no config
/// file of the player's (`--disable` first, as curl wants it), only the endpoint's own protocol
/// (HTTPS but on this PC), quiet but for errors, an HTTP error a failure, 10 seconds to connect
/// and 30 in all. The body is not a file argument, so no folder name goes through the system's
/// code page.
pub fn curl_args(dsn: &Dsn) -> Vec<OsString> {
    let auth = format!(
        "X-Sentry-Auth: Sentry sentry_version=7, sentry_key={}, sentry_client=nv-launcher/{}",
        dsn.key,
        env!("CARGO_PKG_VERSION")
    );
    let proto = if dsn.endpoint.starts_with("https://") {
        "=https"
    } else {
        "=http"
    };
    let mut args: Vec<OsString> = [
        "--disable",
        "--proto",
        proto,
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
        "@-",
    ]
    .iter()
    .map(OsString::from)
    .collect();
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
/// the folder as `envelope.txt` (what went, for the player to see), then given to curl on its
/// input and posted (on Windows without a console window of curl's; stopped after
/// [`CURL_LIMIT`] at most); `sent.txt` notes a success.
/// The event's id, or why it did not go.
pub fn send(report: &Report, folder: &Path, dsn: &Dsn, curl: &Path) -> Result<String, String> {
    let id = event_id();
    let sent_at = format!(
        "{}Z",
        super::utc(std::time::SystemTime::now()).0.replace(' ', "T")
    );
    let body = envelope(report, dsn, &id, &sent_at);
    let file = folder.join("envelope.txt");
    std::fs::write(&file, &body).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut command = std::process::Command::new(curl);
    command.args(curl_args(dsn));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: a console tool without a console window.
        command.creation_flags(0x0800_0000);
    }
    match post(&mut command, &body, CURL_LIMIT) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err("curl, the program that sends it, is not on this PC".into())
        }
        Err(e) => Err(format!("curl could not be started ({e})")),
        Ok(None) => Err("curl did not end in time and was stopped".into()),
        Ok(Some((status, _))) if status.success() => {
            let _ = std::fs::write(
                folder.join("sent.txt"),
                format!("Sent to nv-rs's crash service at {sent_at}: event {id}\n"),
            );
            Ok(id)
        }
        Ok(Some((status, _))) => Err(curl_failure(status.code())),
    }
}

/// How long curl may take in all, its own 30 seconds and some: past it, it is stopped.
const CURL_LIMIT: std::time::Duration = std::time::Duration::from_secs(45);

/// `command` started with `body` on its input and waited for, at most `limit`: its status and
/// what it wrote out, or none when it was stopped at the limit. The body is written, and the
/// output read, by threads of their own, so a program that never reads its input (or fills its
/// output) cannot hold the launcher past the limit.
fn post(
    command: &mut std::process::Command,
    body: &[u8],
    limit: std::time::Duration,
) -> std::io::Result<Option<(std::process::ExitStatus, Vec<u8>)>> {
    use std::io::{Read, Write};
    use std::process::Stdio;
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let input = child.stdin.take().map(|mut input| {
        let body = body.to_vec();
        std::thread::spawn(move || {
            // A program that stops before reading it all says why in its status.
            let _ = input.write_all(&body);
        })
    });
    let read_all = |pipe: Option<Box<dyn Read + Send>>| {
        pipe.map(|mut p| {
            std::thread::spawn(move || {
                let mut got = Vec::new();
                let _ = p.read_to_end(&mut got);
                got
            })
        })
    };
    let out = read_all(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = read_all(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = std::time::Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    if let Some(input) = input {
        let _ = input.join();
    }
    let written = out.and_then(|t| t.join().ok()).unwrap_or_default();
    if let Some(err) = err {
        let _ = err.join();
    }
    Ok(status.map(|s| (s, written)))
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
                // Not ASCII: an item's length is its bytes.
                ("System", "Syst\u{e8}me 1".into()),
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
        // Only this PC by name for plain HTTP; a host, port, path and project of plain words.
        assert_eq!(Dsn::parse("http://k@localhost.example.org/1"), None);
        assert_eq!(Dsn::parse("http://k@127.0.0.1.example.org/1"), None);
        assert!(Dsn::parse("http://k@localhost/1").is_some());
        assert_eq!(Dsn::parse("https://k@host/a-b"), None);
        assert_eq!(Dsn::parse("https://k@ho#st/1"), None);
        assert_eq!(Dsn::parse("https://k@a@host/1"), None);
        assert_eq!(Dsn::parse("https://k@host:x/1"), None);
        assert_eq!(Dsn::parse("https://k@host:/1"), None);
        assert_eq!(Dsn::parse("https://k@host/s p/1"), None);
        assert!(Dsn::parse("https://k@o1.ingest.example.org:8443/sub/7").is_some());
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

    /// An F12 report made in the run goes with the report: its folder, its place and the
    /// player's note as the event's extra data, and the question before sending shows them. A
    /// note not written is none (the place still goes); without an F12 report the event names
    /// none.
    #[test]
    fn an_f12_reports_place_and_note_are_sent() {
        let d = Dsn::parse(DSN).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let sent =
            |r: &Report| String::from_utf8(envelope(r, &d, id, "2001-09-09T01:46:40Z")).unwrap();
        let plain = sent(&report());
        assert!(!plain.contains("F12"), "{plain}");
        let mut r = report();
        r.f12 = Some(crate::crash::F12 {
            folder: "717".into(),
            note: Some("the door would not open\nafter I saved".into()),
            place: Some(
                "interior cell: GSDocMitchellHouse (0x00104c1c)\nposition: 125 -340 7".into(),
            ),
        });
        let text = sent(&r);
        for part in [
            "\"F12 report\":\"reports/717\"",
            "\"F12 place\":\"interior cell: GSDocMitchellHouse (0x00104c1c)\\nposition: 125 -340 7\"",
            "\"F12 note\":\"the door would not open\\nafter I saved\"",
        ] {
            assert!(text.contains(part), "{part} not in {text}");
        }
        let asked = crate::crash::question(&r);
        for part in [
            "F12 report made during this run: reports/717\n",
            "  Where you were:\n    interior cell: GSDocMitchellHouse (0x00104c1c)\n    position: 125 -340 7\n",
            "  Your note:\n    the door would not open\n    after I saved\n",
            "\nThe log's last 2 lines go with it.\n",
        ] {
            assert!(asked.contains(part), "{part} not in {asked}");
        }
        assert!(!asked.contains("stay on this PC"), "{asked}");
        // A note the player did not write: none goes, the place still does.
        r.f12.as_mut().unwrap().note = None;
        let text = sent(&r);
        assert!(!text.contains("F12 note"), "{text}");
        assert!(text.contains("\"F12 place\":\"interior cell: "), "{text}");
        let asked = crate::crash::question(&r);
        assert!(asked.contains("  Your note: (not written)\n"), "{asked}");
    }

    /// A game log as the game writes it, an F12 report and hitches in its last lines, and that F12
    /// report's folder, built into a report and its envelope: the game's own `F12: place:` and
    /// `F12: report in` lines and the hitch lines go whole, their places too; so do the F12
    /// report's place and the player's words in its note (never the note's template). The user's
    /// name, folders, e-mail and network addresses are taken out of all of them, nothing of the
    /// picture goes, and the log sent is the log kept.
    #[test]
    fn where_the_player_was_and_the_f12_note_are_sent_scrubbed() {
        let root =
            std::env::temp_dir().join(format!("nv-launcher-f12-sent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("Profiles").join("jtest");
        let own = root.join("userdata");
        std::fs::create_dir_all(own.join("logs")).unwrap();
        let f12 = own.join("reports").join("007");
        std::fs::create_dir_all(&f12).unwrap();
        let template = "Write here what went wrong and what you did just before it:\r\n\r\n";
        std::fs::write(
            f12.join("note.txt"),
            format!(
                "{template}Jtest's door would not open. Mail jtest@mail.example.net; my notes are in {}\\notes.txt; server 192.168.1.20\r\n",
                home.display()
            ),
        )
        .unwrap();
        std::fs::write(
            f12.join("place.txt"),
            "interior cell: Jtest_Shack (0x0100abcd)\r\nposition: 125 -340 7\r\nview: pitch 0.0 heading 90.0 (degrees)\r\ngame hour: 9.50\r\nweather: none\r\nlighting: NV_D3D9=wgpu\r\nconsole: coc Jtest_Shack; player.setpos x 125\r\n",
        )
        .unwrap();
        std::fs::write(f12.join("picture.bmp"), "BM picture-bytes").unwrap();
        let log = own.join("logs").join("game-1.log");
        let stop =
            "Game's main stopped in frame 900: read of 1a2b3c00 not mapped (in 00412345 < 0040cafe)";
        let lines = [
            "NV_D3D9=wgpu: drawing with Some Card (Vulkan)".to_string(),
            "F12: place: interior cell: Jtest_Shack (0x0100abcd); position: 125 -340 7; view: pitch 0.0 heading 90.0 (degrees); game hour: 9.50; weather: none; lighting: NV_D3D9=wgpu; console: coc Jtest_Shack; player.setpos x 125".to_string(),
            format!("F12: report in {}", f12.display()),
            "hitch: 64.0 ms big (typical 11.9) frame 812 at 14.0 s, interior Doc Mitchell's House (0x00104c1c); stages ms: update 40.1, before draw 0.2, culling and set-up 3.1, world draw 6.0, after world 1.2, present 2.3, after present 11.0; work: main's memory blocks 0 allocated, 22,093 freed; likely cause: freed 22,093 memory blocks".to_string(),
            "hitch: 52.3 ms big (typical 8.1) frame 860 at 16.5 s, first frame after a load, GoodspringsWorld (0x000da726) grid (-3, 5)".to_string(),
            // A mod's interior with the user's name in it, and a file named after it.
            "hitch: 30.0 ms (typical 9.0) frame 870 at 17.0 s, interior Jtest's Shack (0x0100abcd); largest read 2.1 MB of Jtest's Shack.nif".to_string(),
            stop.to_string(),
        ];
        std::fs::write(&log, lines.join("\n") + "\n").unwrap();
        let run = crate::crash::Run {
            status: Some(3),
            shell: crate::crash::Shell::Windows,
            log: &log,
            started: SystemTime::now() - Duration::from_secs(5),
            mods: false,
            own: &own,
            program: &root,
            games: Vec::new(),
            table_card: None,
        };
        let s = crate::crash::scrub::Scrubber::new(&crate::crash::scrub::Known {
            roots: vec![
                (home.display().to_string(), "<home>"),
                (root.display().to_string(), "<nv-rs>"),
            ],
            words: vec![("jtest".into(), "<user>")],
        });
        let facts = crate::crash::system::Facts {
            os: "x".into(),
            cpu: "x".into(),
            threads: None,
            memory: None,
        };
        let ending = Ending::Stop(stop.into());
        let r = crate::crash::build(&run, &ending, &facts, &s, SystemTime::now());
        assert_eq!(r.log.len(), 7);
        let d = Dsn::parse(DSN).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let text = String::from_utf8(envelope(&r, &d, id, "2001-09-09T01:46:40Z")).unwrap();
        for sent in [
            "\nF12: place: interior cell: <user>_Shack (0x0100abcd); position: 125 -340 7; view: pitch 0.0",
            "\nF12: report in <nv-rs>",
            "\nhitch: 64.0 ms big (typical 11.9) frame 812 at 14.0 s, interior Doc Mitchell's House (0x00104c1c); stages ms: update 40.1, before draw 0.2,",
            "; likely cause: freed 22,093 memory blocks\n",
            "\nhitch: 52.3 ms big (typical 8.1) frame 860 at 16.5 s, first frame after a load, GoodspringsWorld (0x000da726) grid (-3, 5)\n",
            "\nhitch: 30.0 ms (typical 9.0) frame 870 at 17.0 s, interior <user>'s Shack (0x0100abcd); largest read 2.1 MB of <user>'s Shack.nif\n",
            "\"F12 report\":\"reports/007\"",
            "\"F12 place\":\"interior cell: <user>_Shack (0x0100abcd)\\nposition: 125 -340 7\\nview: pitch 0.0 heading 90.0 (degrees)\\n",
            "console: coc <user>_Shack; player.setpos x 125\"",
            "\"F12 note\":\"<user>'s door would not open. Mail <e-mail>; my notes are in <home>",
            "; server <ip>\"",
        ] {
            assert!(text.contains(sent), "{sent} not in {text}");
        }
        for never in [
            "jtest",
            "Jtest",
            "mail.example.net",
            "192.168.1.20",
            "Write here",
            "picture-bytes",
            root.to_str().unwrap(),
        ] {
            assert!(!text.contains(never), "{never} in {text}");
        }
        // The log sent is the one kept, every line whole.
        let kept = r.save(&own.join("reports")).unwrap();
        let kept_log = std::fs::read_to_string(kept.join("log.txt")).unwrap();
        let kept_log = kept_log.replace("\r\n", "\n");
        assert!(
            text.contains(&format!("\"filename\":\"log.txt\"}}\n{kept_log}\n")),
            "{text}"
        );
        let asked = crate::crash::question(&r);
        assert!(
            asked.contains("    <user>'s door would not open."),
            "{asked}"
        );
        assert!(
            asked.contains("    interior cell: <user>_Shack (0x0100abcd)\n"),
            "{asked}"
        );
        assert!(
            asked.contains("\nThe log's last 7 lines go with it.\n"),
            "{asked}"
        );
        // A note left as F12 wrote it: no note goes, the place still does.
        std::fs::write(f12.join("note.txt"), template).unwrap();
        let r = crate::crash::build(&run, &ending, &facts, &s, SystemTime::now());
        let text = String::from_utf8(envelope(&r, &d, id, "2001-09-09T01:46:40Z")).unwrap();
        assert!(!text.contains("F12 note"), "{text}");
        assert!(!text.contains("Write here"), "{text}");
        assert!(text.contains("\"F12 place\":\"interior cell: "), "{text}");
        assert!(crate::crash::question(&r).contains("  Your note: (not written)\n"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The per-crash question through a stand-in window: only "Send" (entry 1) is a yes; "Don't
    /// send" is first (highlighted); a reply dropped without an answer and no window are a no.
    #[test]
    fn only_send_answers_the_crash_question_yes() {
        use crate::session::Event;
        let answering = |answer: Option<usize>, window: bool| {
            move |event: Event| {
                if let Event::Message { entries, reply, .. } = event {
                    assert_eq!(entries, ["Don't send", "Send"]);
                    if let Some(a) = answer {
                        let _ = reply.send(a);
                    }
                }
                window
            }
        };
        assert!(crate::crash::ask_with(answering(Some(1), true), "q"));
        assert!(!crate::crash::ask_with(answering(Some(0), true), "q"));
        assert!(!crate::crash::ask_with(answering(None, true), "q"));
        assert!(!crate::crash::ask_with(answering(Some(1), false), "q"));
        // Esc and a closed window answer Don't send.
        let entries: Vec<String> = crate::crash::ASK_ENTRIES.map(String::from).to_vec();
        assert_eq!(crate::app::back_entry(&entries), Some(0));
    }

    /// The stricter of the saved answer and this launcher's own is in force: an answer that
    /// could not be saved never sends more than the saved one, and a stricter one holds.
    #[test]
    fn the_stricter_of_the_saved_and_the_session_answer_holds() {
        let root = std::env::temp_dir().join(format!("nv-launcher-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("reports").join("crash-x");
        std::fs::create_dir_all(&folder).unwrap();
        let program = root.join("program");
        std::fs::create_dir_all(&program).unwrap();
        std::fs::write(program.join(DSN_FILE), format!("{DSN}\n")).unwrap();
        let r = report();
        let no_curl = Path::new("nv-rs-no-such-curl-here");
        let asked = std::cell::Cell::new(0);
        let ask = |text: &str| {
            assert!(text.starts_with("The game stopped"));
            asked.set(asked.get() + 1);
            false
        };
        use consent::Consent as C;
        // Saved Send, Never or no answer here (not saved): nothing goes, nothing is asked.
        consent::write(&root, C::Send).unwrap();
        for session in [C::Never, C::Unanswered] {
            assert_eq!(
                deliver(&r, &folder, &root, &program, no_curl, Some(session), ask),
                None
            );
        }
        // Saved Send, Ask here: asked first.
        assert_eq!(
            deliver(&r, &folder, &root, &program, no_curl, Some(C::Ask), ask).as_deref(),
            Some("The crash report was not sent: it stays on this PC.")
        );
        // Saved Never, Send here (it could not be saved): the saved one holds.
        consent::write(&root, C::Never).unwrap();
        assert_eq!(
            deliver(&r, &folder, &root, &program, no_curl, Some(C::Send), ask),
            None
        );
        assert_eq!(asked.get(), 1);
        assert!(!folder.join("envelope.txt").exists());
        // The launcher's own answer, given where it could not be kept (nv-rs may not write in
        // its folder): the saved Send no longer sends in this run.
        consent::write(&root, C::Send).unwrap();
        let note = crate::app::keep_consent(&root, true, C::Never, true);
        assert!(note.is_some_and(|n| n.starts_with("Not saved")));
        assert_eq!(consent::read(&root), C::Send);
        assert_eq!(consent::session(&root), Some(C::Never));
        assert_eq!(
            deliver(
                &r,
                &folder,
                &root,
                &program,
                no_curl,
                consent::session(&root),
                ask
            ),
            None
        );
        assert!(!folder.join("envelope.txt").exists());
        assert_eq!(consent::session(&program), None);
        assert_eq!(consent::in_force(C::Send, None), C::Send);
        assert_eq!(consent::in_force(C::Ask, Some(C::Send)), C::Ask);
        assert_eq!(consent::in_force(C::Send, Some(C::Ask)), C::Ask);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// curl reads no config file of the player's (`--disable` first), speaks only HTTPS (plain
    /// HTTP only to this PC), reads the body on its input, and has its limits.
    #[test]
    fn curl_is_asked_to_post_the_file_quietly_with_limits() {
        let d = Dsn::parse(DSN).unwrap();
        let strings = |d: &Dsn| -> Vec<String> {
            curl_args(d)
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect()
        };
        let args = strings(&d);
        assert_eq!(args[0], "--disable");
        assert_eq!(args.last(), Some(&d.endpoint));
        let after = |args: &[String], flag: &str| {
            let at = args.iter().position(|a| a == flag).expect(flag);
            args[at + 1].clone()
        };
        assert_eq!(after(&args, "--proto"), "=https");
        assert_eq!(after(&args, "--connect-timeout"), "10");
        assert_eq!(after(&args, "--max-time"), "30");
        assert_eq!(after(&args, "--data-binary"), "@-");
        assert_eq!(after(&args, "-X"), "POST");
        let local = Dsn::parse("http://k@127.0.0.1:9000/1").unwrap();
        assert_eq!(after(&strings(&local), "--proto"), "=http");
        for want in ["--fail", "Content-Type: application/x-sentry-envelope"] {
            assert!(args.iter().any(|a| a == want), "{want}");
        }
        assert!(!args.iter().any(|a| a == "-L" || a == "--location"));
        assert!(args.iter().any(|a| a
            .starts_with("X-Sentry-Auth: Sentry sentry_version=7, sentry_key=0123456789abcdef")));
        assert_eq!(
            curl_failure(Some(6)),
            "the crash service could not be reached (no network?)"
        );
        assert_eq!(curl_failure(Some(22)), "the crash service refused it");
        assert_eq!(curl_failure(Some(99)), "curl ended with status 99");
    }

    /// The envelope goes whole on the program's input (a program that echoes it stands in for
    /// curl: Windows' findstr, elsewhere cat), and a program that never ends is stopped at the
    /// limit.
    #[test]
    fn the_envelope_goes_on_curls_input_and_a_stuck_program_is_stopped() {
        use std::process::Command;
        let body = b"{\"event_id\":\"0123\"}\n{\"type\":\"event\"}\nthe log's last line\n";
        let mut echo = if cfg!(windows) {
            let mut c = Command::new("findstr");
            c.arg("^");
            c
        } else {
            Command::new("cat")
        };
        let (status, out) = post(&mut echo, body, Duration::from_secs(30))
            .unwrap()
            .expect("the echo ends by itself");
        assert!(status.success());
        assert_eq!(
            String::from_utf8_lossy(&out).replace("\r\n", "\n"),
            String::from_utf8_lossy(body)
        );
        let mut stuck = if cfg!(windows) {
            let mut c = Command::new("ping");
            c.args(["-n", "60", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("60");
            c
        };
        let started = std::time::Instant::now();
        assert!(post(&mut stuck, body, Duration::from_secs(1))
            .unwrap()
            .is_none());
        assert!(started.elapsed() < Duration::from_secs(20));
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
            deliver(&r, &folder, &root, &program, no_curl, None, ask(true)),
            None
        );
        std::fs::write(program.join(DSN_FILE), format!("# a comment\n{DSN}\n")).unwrap();
        for answer in [consent::Consent::Never, consent::Consent::Unanswered] {
            consent::write(&root, answer).unwrap();
            assert_eq!(
                deliver(&r, &folder, &root, &program, no_curl, None, ask(true)),
                None
            );
        }
        assert_eq!(asked.get(), 0);
        assert!(!folder.join("envelope.txt").exists());
        consent::write(&root, consent::Consent::Ask).unwrap();
        assert_eq!(
            deliver(&r, &folder, &root, &program, no_curl, None, ask(false)).as_deref(),
            Some("The crash report was not sent: it stays on this PC.")
        );
        assert_eq!(asked.get(), 1);
        assert!(!folder.join("envelope.txt").exists());
        let line = deliver(&r, &folder, &root, &program, no_curl, None, ask(true)).unwrap();
        assert!(
            line.contains("could not be sent (curl, the program that sends it, is not on this PC)"),
            "{line}"
        );
        assert_eq!(asked.get(), 2);
        assert!(folder.join("envelope.txt").exists());
        assert!(!folder.join("sent.txt").exists());
        consent::write(&root, consent::Consent::Send).unwrap();
        let line = deliver(&r, &folder, &root, &program, no_curl, None, ask(true)).unwrap();
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
