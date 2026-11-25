//! The player's system for a crash report: the operating system and its version, the processor
//! and its threads, the memory, and the computer's name (only so the report can leave it out).
//! Read from the system itself, only reading: Windows' registry and one memory call; Linux's
//! `/etc/os-release` and `/proc`; macOS's `SystemVersion.plist` and `sysctl`. The readers of the
//! files' text are plain functions, tested on every system.

/// What the report says about the system.
pub struct Facts {
    pub os: String,
    pub cpu: String,
    pub threads: Option<usize>,
    /// Bytes.
    pub memory: Option<u64>,
}

/// This system's facts.
pub fn facts() -> Facts {
    Facts {
        os: os(),
        cpu: cpu().unwrap_or_else(|| format!("not read ({})", std::env::consts::ARCH)),
        threads: std::thread::available_parallelism().ok().map(|n| n.get()),
        memory: memory(),
    }
}

/// Memory in the units Windows' and macOS's own windows show it: `15.6 GB` (of 1024^3 bytes).
pub fn memory_text(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn read(path: &str) -> String {
    std::fs::read(path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn sysctl(key: &str) -> Option<String> {
    let out = std::process::Command::new("sysctl")
        .args(["-n", key])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !text.is_empty()).then_some(text)
}

#[cfg(windows)]
fn os() -> String {
    crate::play::win::windows_version()
}

#[cfg(target_os = "linux")]
fn os() -> String {
    linux_os(
        &read("/etc/os-release"),
        &read("/proc/sys/kernel/osrelease"),
    )
}

#[cfg(target_os = "macos")]
fn os() -> String {
    mac_os(&read("/System/Library/CoreServices/SystemVersion.plist"))
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn os() -> String {
    std::env::consts::OS.to_string()
}

#[cfg(windows)]
fn cpu() -> Option<String> {
    crate::play::win::processor_name()
}

#[cfg(target_os = "linux")]
fn cpu() -> Option<String> {
    cpuinfo_model(&read("/proc/cpuinfo"))
}

#[cfg(target_os = "macos")]
fn cpu() -> Option<String> {
    sysctl("machdep.cpu.brand_string")
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn cpu() -> Option<String> {
    None
}

#[cfg(windows)]
fn memory() -> Option<u64> {
    crate::play::win::memory_bytes()
}

#[cfg(target_os = "linux")]
fn memory() -> Option<u64> {
    meminfo_total(&read("/proc/meminfo"))
}

#[cfg(target_os = "macos")]
fn memory() -> Option<u64> {
    sysctl("hw.memsize")?.parse().ok()
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn memory() -> Option<u64> {
    None
}

/// The computer's names, for the report to leave out (macOS: `name.local` and `name`).
pub fn computer_names() -> Vec<String> {
    #[cfg(windows)]
    let names: Vec<String> = std::env::var("COMPUTERNAME")
        .into_iter()
        .chain(crate::play::win::dns_host_name())
        .collect();
    #[cfg(target_os = "linux")]
    let names: Vec<String> = vec![read("/proc/sys/kernel/hostname")];
    #[cfg(target_os = "macos")]
    let names: Vec<String> = sysctl("kern.hostname").into_iter().collect();
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    let names: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for name in names {
        let name = name.trim().to_string();
        if let Some((short, _)) = name.split_once('.') {
            out.push(short.to_string());
        }
        if !name.is_empty() {
            out.push(name);
        }
    }
    out
}

/// `/etc/os-release`'s name (`PRETTY_NAME`, else `NAME` and `VERSION_ID`) and the kernel's
/// release: `Ubuntu 24.04.1 LTS (Linux 6.8.0-45-generic)`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn linux_os(os_release: &str, kernel: &str) -> String {
    let field = |key: &str| {
        os_release.lines().find_map(|l| {
            let v = l.trim().strip_prefix(key)?.strip_prefix('=')?;
            let v = v.trim().trim_matches('"').trim_matches('\'').trim();
            (!v.is_empty()).then(|| v.to_string())
        })
    };
    let name = field("PRETTY_NAME").unwrap_or_else(|| match (field("NAME"), field("VERSION_ID")) {
        (Some(n), Some(v)) => format!("{n} {v}"),
        (Some(n), None) => n,
        _ => "Linux".to_string(),
    });
    match kernel.trim() {
        "" => name,
        k => format!("{name} (Linux {k})"),
    }
}

/// `SystemVersion.plist`'s version and build: `macOS 14.5 (23F79)`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn mac_os(plist: &str) -> String {
    let value = |key: &str| {
        let at = plist.find(&format!("<key>{key}</key>"))?;
        let rest = &plist[at..];
        let start = rest.find("<string>")? + "<string>".len();
        let end = rest[start..].find("</string>")?;
        Some(rest[start..start + end].trim().to_string()).filter(|v| !v.is_empty())
    };
    match (value("ProductVersion"), value("ProductBuildVersion")) {
        (Some(v), Some(b)) => format!("macOS {v} ({b})"),
        (Some(v), None) => format!("macOS {v}"),
        _ => "macOS (version not read)".to_string(),
    }
}

/// `/proc/meminfo`'s `MemTotal`, in bytes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn meminfo_total(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: u64 = line["MemTotal:".len()..]
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(kb * 1024)
}

/// `/proc/cpuinfo`'s first `model name` (x86), else its `Model` (Arm boards).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn cpuinfo_model(text: &str) -> Option<String> {
    ["model name", "Model"].iter().find_map(|key| {
        text.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == *key && !v.trim().is_empty()).then(|| v.trim().to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_names_itself_from_os_release() {
        let text = "NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\n";
        assert_eq!(
            linux_os(text, "6.8.0-45-generic\n"),
            "Ubuntu 24.04.1 LTS (Linux 6.8.0-45-generic)"
        );
        assert_eq!(
            linux_os("NAME=SteamOS\nVERSION_ID=3.6\n", ""),
            "SteamOS 3.6"
        );
        assert_eq!(linux_os("", "6.1"), "Linux (Linux 6.1)");
    }

    #[test]
    fn macos_names_its_version_and_build() {
        let plist = "<dict>\n\t<key>ProductBuildVersion</key>\n\t<string>23F79</string>\n\t<key>ProductName</key>\n\t<string>macOS</string>\n\t<key>ProductVersion</key>\n\t<string>14.5</string>\n</dict>";
        assert_eq!(mac_os(plist), "macOS 14.5 (23F79)");
        assert_eq!(mac_os(""), "macOS (version not read)");
    }

    #[test]
    fn memory_and_processor_are_read_from_proc() {
        assert_eq!(
            meminfo_total("MemTotal:       16314232 kB\nMemFree: 1 kB\n"),
            Some(16314232 * 1024)
        );
        assert_eq!(meminfo_total("MemFree: 1 kB\n"), None);
        assert_eq!(
            cpuinfo_model("processor\t: 0\nmodel name\t: AMD Custom APU 0405\n").as_deref(),
            Some("AMD Custom APU 0405")
        );
        assert_eq!(
            cpuinfo_model("Model\t\t: Raspberry Pi 5\n").as_deref(),
            Some("Raspberry Pi 5")
        );
        assert_eq!(memory_text(16 * 1024 * 1024 * 1024), "16.0 GB");
    }

    #[test]
    fn this_system_answers() {
        let f = facts();
        assert!(!f.os.is_empty());
        assert!(!f.cpu.is_empty());
        assert!(f.threads.is_some());
    }

    /// Windows: the computer's names hold COMPUTERNAME (NetBIOS: capitals, at most 15 characters)
    /// and the DNS host name, which is the same name without case up to those 15 characters.
    #[cfg(windows)]
    #[test]
    fn windows_gives_both_of_the_computers_names() {
        let netbios = std::env::var("COMPUTERNAME").unwrap();
        let dns = crate::play::win::dns_host_name().expect("a DNS host name");
        assert!(!dns.is_empty() && dns.len() < 256);
        let names = computer_names();
        assert!(names.contains(&netbios), "{names:?}");
        assert!(names.contains(&dns), "{names:?}");
        let short: String = dns.chars().take(15).collect();
        assert!(short.eq_ignore_ascii_case(&netbios), "{dns} and {netbios}");
    }
}
