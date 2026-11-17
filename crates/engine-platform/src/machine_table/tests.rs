use super::*;

fn zone_of(tz: &str, year: i64) -> ZoneInfo {
    let zone = parse_posix_tz(tz).unwrap();
    ZoneInfo::from_bytes(&time_zone_information(&zone, year).unwrap())
}

/// Seconds since 1970 of a UTC date and hour.
fn at(year: i64, month: u32, day: u32, hour: i64) -> i64 {
    zone::days_from_civil(year, month, day) * 86_400 + hour * 3600
}

/// US Eastern gives what Windows' "Eastern Standard Time" entry holds (Bias 300, daylight
/// from the second Sunday of March 02:00, standard from the first Sunday of November 02:00,
/// DaylightBias -60; the names apart).
#[test]
fn us_eastern_is_windows_eastern() {
    let z = zone_of("EST5EDT,M3.2.0,M11.1.0", 2026);
    assert_eq!(z.bias, 300);
    assert_eq!(z.standard_bias, 0);
    assert_eq!(z.daylight_bias, -60);
    assert_eq!(z.daylight_date, [0, 3, 0, 2, 2, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 11, 0, 1, 2, 0, 0, 0]);
}

/// Central Europe: east of UTC (a negative bias), the last Sunday, changes at 02:00 and 03:00.
#[test]
fn central_europe_ends_on_the_last_sunday_at_three() {
    let z = zone_of("CET-1CEST,M3.5.0,M10.5.0/3", 2026);
    assert_eq!(z.bias, -60);
    assert_eq!(z.daylight_date, [0, 3, 0, 5, 2, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 10, 0, 5, 3, 0, 0, 0]);
}

/// Southern hemisphere: daylight time from October across New Year to April.
#[test]
fn sydney_has_daylight_time_across_new_year() {
    let z = zone_of("AEST-10AEDT,M10.1.0,M4.1.0/3", 2026);
    assert_eq!(z.bias, -600);
    assert_eq!(z.daylight_date, [0, 10, 0, 1, 2, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 4, 0, 1, 3, 0, 0, 0]);
    assert_eq!(zone::bias_at(&z, at(2026, 1, 15, 0)), -660);
    assert_eq!(zone::bias_at(&z, at(2026, 7, 15, 0)), -600);
    assert_eq!(zone::bias_at(&z, at(2026, 12, 31, 0)), -660);
}

/// No daylight time: no dates, the answer 0, the standard name in both name fields.
#[test]
fn a_zone_without_daylight_time_has_no_dates() {
    let zone = parse_posix_tz("JST-9").unwrap();
    let bytes = time_zone_information(&zone, 2026).unwrap();
    let z = ZoneInfo::from_bytes(&bytes);
    assert_eq!(z.bias, -540);
    assert_eq!((z.daylight_date, z.standard_date), ([0; 8], [0; 8]));
    assert_eq!(z.daylight_bias, 0);
    assert_eq!(&bytes[0x58..0x5e], &[b'J', 0, b'S', 0, b'T', 0]);
    let lines = zone_lines(&Zone::from_rule(zone), at(2026, 7, 1, 0)).unwrap();
    assert!(lines.starts_with("GetTimeZoneInformation\t\t0\t"));
}

/// 30- and 45-minute offsets: India, Newfoundland (with daylight time), Nepal, and the Chatham
/// Islands (45 minutes, changes at 02:45 and 03:45).
#[test]
fn half_and_three_quarter_hour_offsets() {
    assert_eq!(zone_of("IST-5:30", 2026).bias, -330);
    let newfoundland = zone_of("NST3:30NDT,M3.2.0,M11.1.0", 2026);
    assert_eq!((newfoundland.bias, newfoundland.daylight_bias), (210, -60));
    assert_eq!(zone_of("<+0545>-5:45", 2026).bias, -345);
    let chatham = zone_of("<+1245>-12:45<+1345>,M9.5.0/2:45,M4.1.0/3:45", 2026);
    assert_eq!((chatham.bias, chatham.daylight_bias), (-765, -60));
    assert_eq!(chatham.daylight_date, [0, 9, 0, 5, 2, 45, 0, 0]);
    assert_eq!(chatham.standard_date, [0, 4, 0, 1, 3, 45, 0, 0]);
}

/// Israel's rule, the Friday before the last Sunday of March (`M3.4.4/26`: the fourth Thursday at
/// 26:00), has no Windows form: it is this year's real date. 2026: Friday 27 March 02:00, the
/// last Friday (5); 2027: Friday 26 March, again the last.
#[test]
fn israel_is_this_years_real_date() {
    let tz = "IST-2IDT,M3.4.4/26,M10.5.0";
    let z = zone_of(tz, 2026);
    assert_eq!(z.bias, -120);
    assert_eq!(z.daylight_date, [0, 3, 5, 5, 2, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 10, 0, 5, 2, 0, 0, 0]);
    // Daylight time starts at 2026-03-27 02:00 local standard time (00:00 UTC).
    assert_eq!(zone::bias_at(&z, at(2026, 3, 26, 23)), -120);
    assert_eq!(zone::bias_at(&z, at(2026, 3, 27, 0)), -180);
    assert_eq!(zone_of(tz, 2027).daylight_date, [0, 3, 5, 5, 2, 0, 0, 0]);
    // A year whose Friday is not the last of its month: last Sunday 2029-03-25, Friday the 23rd
    // (the fourth Friday; the 30th is a Friday too).
    assert_eq!(zone_of(tz, 2029).daylight_date, [0, 3, 5, 4, 2, 0, 0, 0]);
}

/// Julian days: `Jn` never counts February 29, `n` does (from 0).
#[test]
fn julian_rules_are_this_years_real_dates() {
    let z = zone_of("XXX3YYY,J60,J300", 2026);
    // Day 60 of 2026 is Sunday 1 March (the first Sunday); day 300 Tuesday 27 October (the last).
    assert_eq!(z.daylight_date, [0, 3, 0, 1, 2, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 10, 2, 5, 2, 0, 0, 0]);
    // 2028 is a leap year: J60 is still 1 March (a Wednesday), day 59 counted from 0 is 29 February.
    assert_eq!(
        zone_of("XXX3YYY,J60,J300", 2028).daylight_date,
        [0, 3, 3, 1, 2, 0, 0, 0]
    );
    assert_eq!(
        zone_of("XXX3YYY,59,299", 2028).daylight_date,
        [0, 2, 2, 5, 2, 0, 0, 0]
    );
}

/// A negative time of day is the day before; a change-over pushed into another year has no form.
#[test]
fn times_outside_the_day_move_the_date() {
    // The second Sunday of March 2026 is the 8th: at -1:00 it is Saturday the 7th at 23:00.
    let z = zone_of("XXX3YYY,M3.2.0/-1,M11.1.0", 2026);
    assert_eq!(z.daylight_date, [0, 3, 6, 1, 23, 0, 0, 0]);
    let late = parse_posix_tz("XXX3YYY,M3.2.0,365/2").unwrap();
    assert!(time_zone_information(&late, 2026).is_err());
    assert!(time_zone_information(&late, 2028).is_ok());
}

#[test]
fn rules_that_cannot_be_read_are_refused() {
    for tz in [
        "X",
        "EST",
        "EST5EDT",
        "EST5EDT,M3.2.0",
        "EST5EDT,M13.2.0,M11.1.0",
        "EST5EDT,M3.6.0,M11.1.0",
        "EST5EDT,M3.2.7,M11.1.0",
        "EST5EDT,J0,M11.1.0",
        "EST5EDT,366,M11.1.0",
        "EST5EDT,M3.2.0/168,M11.1.0",
        "EST25",
        "EST5:60",
        "EST5EDT,M3.2.0,M11.1.0x",
        "<+0545-5:45",
    ] {
        assert!(parse_posix_tz(tz).is_err(), "{tz}");
    }
    // Seconds in an offset are not whole minutes.
    let zone = parse_posix_tz("LMT0:00:30").unwrap();
    assert!(time_zone_information(&zone, 2026).is_err());
}

#[test]
fn the_names_are_the_zones_abbreviations_in_utf16() {
    let zone = parse_posix_tz("<+0545>-5:45").unwrap();
    assert_eq!(zone.name, "+0545");
    let b =
        time_zone_information(&parse_posix_tz("EST5EDT,M3.2.0,M11.1.0").unwrap(), 2026).unwrap();
    assert_eq!(&b[4..12], &[b'E', 0, b'S', 0, b'T', 0, 0, 0]);
    assert_eq!(&b[0x58..0x60], &[b'E', 0, b'D', 0, b'T', 0, 0, 0]);
}

/// The lines: the answer now (daylight in July, standard in January) and one rule for every year.
#[test]
fn the_zone_lines_answer_now_and_every_year() {
    let zone = Zone::from_rule(parse_posix_tz("EST5EDT,M3.2.0,M11.1.0").unwrap());
    let july = zone_lines(&zone, at(2026, 7, 1, 12)).unwrap();
    let mut lines = july.lines();
    let now = lines.next().unwrap();
    assert!(
        now.starts_with("GetTimeZoneInformation\t\t2\t2c010000"),
        "{now}"
    );
    assert!(lines
        .next()
        .unwrap()
        .starts_with("GetTimeZoneInformationForYear\t641 786b\t1\t2c010000"));
    assert_eq!(lines.next(), None);
    let january = zone_lines(&zone, at(2026, 1, 1, 12)).unwrap();
    assert!(january.starts_with("GetTimeZoneInformation\t\t1\t"));
}

/// The rule line of a TZif file is its last line; older or broken files are refused.
#[test]
fn the_zone_files_rule_is_its_last_line() {
    let mut file = b"TZif2".to_vec();
    file.extend_from_slice(&[0, 1, 10, 255, 10, 7]);
    file.extend_from_slice(b"\nEST5EDT,M3.2.0,M11.1.0\n");
    assert_eq!(tzif_footer(&file), Ok("EST5EDT,M3.2.0,M11.1.0"));
    // Version 1 files carry a zero byte there; "1" is no version of the format and is refused too.
    for version in [0, b'1'] {
        file[4] = version;
        assert!(tzif_footer(&file).is_err());
    }
    assert!(tzif_footer(b"TZif3\x00\n\n").is_err());
    assert!(tzif_footer(b"TZif3\x00\nUTC0").is_err());
    assert!(tzif_footer(b"MZ\x00\nUTC0\n").is_err());
}

/// A whole `/proc/meminfo` in the kernel's layout from a desktop under Linux's default overcommit,
/// with more committed (`Committed_AS`, 25 GB) than memory and swap together (18 GB).
const OVERCOMMITTED_MEMINFO: &str = "\
MemTotal:       16000000 kB
MemFree:         1000000 kB
MemAvailable:    7040000 kB
Buffers:          312004 kB
Cached:          5893120 kB
SwapCached:        10240 kB
Active:          8123456 kB
Inactive:        4567890 kB
SwapTotal:       2000000 kB
SwapFree:        1500000 kB
Dirty:              1234 kB
AnonPages:       7012345 kB
Mapped:          1234567 kB
Shmem:            456789 kB
Slab:             567890 kB
CommitLimit:    10000000 kB
Committed_AS:   25000000 kB
VmallocTotal:   34359738367 kB
HugePages_Total:       0
Hugepagesize:       2048 kB
";

#[test]
fn memory_from_linux_meminfo() {
    let m = memory_from_meminfo(OVERCOMMITTED_MEMINFO).unwrap();
    assert_eq!(m.total_phys, 16_000_000 * 1024);
    assert_eq!(m.avail_phys, 7_040_000 * 1024);
    // (16,000,000 - 7,040,000) / 16,000,000 = 56 %.
    assert_eq!(m.load, 56);
    // Memory plus swap; available memory plus free swap, whatever is committed.
    assert_eq!(m.total_page_file, 18_000_000 * 1024);
    assert_eq!(m.avail_page_file, 8_540_000 * 1024);
    assert!(m.avail_page_file >= m.avail_phys && m.avail_page_file > 0x2000_0000);
    // No swap: the available page file is the available memory.
    let no_swap = OVERCOMMITTED_MEMINFO
        .replace("SwapTotal:       2000000", "SwapTotal:       0")
        .replace("SwapFree:        1500000", "SwapFree:        0");
    let m = memory_from_meminfo(&no_swap).unwrap();
    assert_eq!(
        (m.total_page_file, m.avail_page_file),
        (m.total_phys, m.avail_phys)
    );
    assert_eq!(memory_from_meminfo("MemTotal: 1 kB\n"), None);
    assert_eq!(
        memory_from_meminfo(&OVERCOMMITTED_MEMINFO.replace("SwapFree:        1500000 kB", "")),
        None
    );
    assert_eq!(
        memory_from_meminfo(&OVERCOMMITTED_MEMINFO.replace("7040000 kB", "7040000")),
        None
    );
}

/// Windows' load rounds down (measured): 55.99 % in use is 55.
#[test]
fn the_memory_load_rounds_down() {
    assert_eq!(memory_load(10_000, 4_401), 55);
    assert_eq!(memory_load(10_000, 4_400), 56);
    assert_eq!(memory_load(10_000, 0), 100);
    assert_eq!(memory_load(10_000, 20_000), 0);
    assert_eq!(memory_load(0, 0), 0);
}

#[test]
fn memory_from_macos_sysctl() {
    let m = memory_from_sysctl(16 << 30, 40, 2 << 30, 1 << 30);
    assert_eq!(m.avail_phys, (16u64 << 30) * 40 / 100);
    assert_eq!(m.load, 60);
    assert_eq!(m.total_page_file, 18 << 30);
    assert_eq!(m.avail_page_file, m.avail_phys + (1 << 30));
    let full = memory_from_sysctl(16 << 30, 250, 0, 5);
    assert_eq!(
        (full.avail_phys, full.avail_page_file),
        (16 << 30, 16 << 30)
    );
    // No swap file yet (macOS makes them on demand): the available memory itself.
    let m = memory_from_sysctl(8 << 30, 50, 0, 0);
    assert_eq!((m.total_page_file, m.avail_page_file), (8 << 30, 4 << 30));
}

/// The page file's sizes saturate rather than wrap.
#[test]
fn page_file_sizes_saturate() {
    assert_eq!(page_file(u64::MAX, 5, 10, 10), (u64::MAX, 15));
    assert_eq!(
        page_file(u64::MAX - 1, u64::MAX - 1, 10, 10),
        (u64::MAX, u64::MAX)
    );
}

/// A change-over at 24:00 (Santiago, the first Saturday at 24:00) is the Sunday at 00:00, never
/// an hour 24.
#[test]
fn a_change_over_at_twenty_four_is_the_next_day() {
    let z = zone_of("<-04>4<-03>,M9.1.6/24,M4.1.6/24", 2026);
    assert_eq!(z.bias, 240);
    // 2026: Saturday 5 September 24:00 is Sunday the 6th; Saturday 4 April, Sunday the 5th.
    assert_eq!(z.daylight_date, [0, 9, 0, 1, 0, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 4, 0, 1, 0, 0, 0, 0]);
}

/// Lord Howe Island's daylight time is half an hour.
#[test]
fn a_half_hour_daylight_shift() {
    let z = zone_of("<+1030>-10:30<+11>-11,M10.1.0,M4.1.0", 2026);
    assert_eq!((z.bias, z.daylight_bias), (-630, -30));
    assert_eq!(zone::bias_at(&z, at(2026, 1, 15, 0)), -660);
    assert_eq!(zone::bias_at(&z, at(2026, 7, 15, 0)), -630);
}

/// Nuuk: the last Sunday of March at -1:00 is Saturday 28 March 2026 at 23:00, the last Saturday.
#[test]
fn the_last_sunday_at_minus_one_is_the_last_saturday() {
    let z = zone_of("<-02>2<-01>,M3.5.0/-1,M10.5.0/0", 2026);
    assert_eq!(z.daylight_date, [0, 3, 6, 5, 23, 0, 0, 0]);
    assert_eq!(z.standard_date, [0, 10, 0, 5, 0, 0, 0, 0]);
}

/// Ireland's negative daylight time: IST (+1 h) is standard time, GMT is daylight time in winter.
/// The offsets in force are right at every moment; the answer says daylight in winter.
#[test]
fn negative_daylight_time_keeps_the_offsets() {
    let zone = parse_posix_tz("IST-1GMT0,M10.5.0,M3.5.0/1").unwrap();
    let z = ZoneInfo::from_bytes(&time_zone_information(&zone, 2026).unwrap());
    assert_eq!((z.bias, z.daylight_bias), (-60, 60));
    assert_eq!(zone::bias_at(&z, at(2026, 1, 15, 12)), 0);
    assert_eq!(zone::bias_at(&z, at(2026, 7, 15, 12)), -60);
    let zone = Zone::from_rule(zone);
    assert!(zone_lines(&zone, at(2026, 1, 15, 12))
        .unwrap()
        .starts_with("GetTimeZoneInformation\t\t2\t"));
}

/// Real zone files (IANA tzdata 2026e, public domain), kept as hex text (`xxd -p` of the files;
/// the repository holds no binary files): their bytes.
fn real_zone_bytes(name: &str) -> Vec<u8> {
    let hex = match name {
        "Gaza" => include_str!("../../data/machine/zoneinfo/Asia-Gaza.tzif.hex"),
        "Casablanca" => include_str!("../../data/machine/zoneinfo/Africa-Casablanca.tzif.hex"),
        "Dublin" => include_str!("../../data/machine/zoneinfo/Europe-Dublin.tzif.hex"),
        _ => unreachable!(),
    };
    let digits: Vec<u8> = hex.bytes().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks(2)
        .map(|d| u8::from_str_radix(std::str::from_utf8(d).unwrap(), 16).unwrap())
        .collect()
}

fn real_zone(name: &str) -> Zone {
    parse_tzif(&real_zone_bytes(name)).unwrap()
}

fn info_at(zone: &Zone, utc: i64) -> ZoneInfo {
    ZoneInfo::from_bytes(&zone_information_at(zone, utc).unwrap())
}

/// Gaza's 2025 daylight time (from 12 April 00:00 UTC to 24 October 23:00 UTC) is in its file's
/// list, not in its rule line (which would start it on 29 March): the list answers.
#[test]
fn a_zone_files_list_answers_this_year() {
    let zone = real_zone("Gaza");
    assert_eq!(zone.rule.name, "EET");
    assert_eq!(zone.changes.len(), 308);
    let z = info_at(&zone, at(2025, 7, 1, 12));
    assert_eq!((z.bias, z.daylight_bias), (-120, -60));
    assert_eq!(z.daylight_date, [0, 4, 6, 2, 2, 0, 0, 0]);
    assert_eq!(zone::bias_at(&z, at(2025, 4, 11, 23) + 3599), -120);
    assert_eq!(zone::bias_at(&z, at(2025, 4, 12, 0)), -180);
    assert_eq!(zone::bias_at(&z, at(2025, 10, 24, 22) + 3599), -180);
    assert_eq!(zone::bias_at(&z, at(2025, 10, 24, 23)), -120);
    // The rule line alone would be wrong on 1 April 2025.
    let rule = ZoneInfo::from_bytes(&time_zone_information(&zone.rule, 2025).unwrap());
    assert_eq!(zone::bias_at(&rule, at(2025, 4, 1, 12)), -180);
    assert_eq!(zone::bias_at(&z, at(2025, 4, 1, 12)), -120);
}

/// Morocco: +01 with +00 during Ramadan (daylight time in the file: 15 February to 22 March
/// 2026); after its last change-over (20 September 2026, to +00 for good) the rule line answers.
#[test]
fn a_ramadan_break_and_the_rule_after_the_list() {
    let zone = real_zone("Casablanca");
    let z = info_at(&zone, at(2026, 7, 1, 12));
    assert_eq!((z.bias, z.daylight_bias), (-60, 60));
    assert_eq!(zone::bias_at(&z, at(2026, 3, 1, 12)), 0);
    assert_eq!(zone::bias_at(&z, at(2026, 7, 1, 12)), -60);
    let after = info_at(&zone, at(2026, 11, 16, 12));
    assert_eq!((after.bias, after.daylight_date), (0, [0; 8]));
    // 2012 had two daylight periods (29 April to 20 July, 20 August to 30 September, +00 in
    // Ramadan between): in mid-September the start and end around now are taken.
    let z = info_at(&zone, at(2012, 9, 15, 12));
    assert_eq!(zone::bias_at(&z, at(2012, 9, 15, 12)), -60);
    assert_eq!(zone::bias_at(&z, at(2012, 8, 1, 12)), 0);
    assert_eq!(zone::bias_at(&z, at(2012, 10, 15, 12)), 0);
}

/// Dublin's file lists change-overs to 2037: its 2026 answer from the list is the rule line's,
/// byte for byte.
#[test]
fn a_list_and_its_rule_agree_where_both_answer() {
    let zone = real_zone("Dublin");
    let now = at(2026, 1, 15, 12);
    assert_eq!(
        zone_information_at(&zone, now).unwrap(),
        time_zone_information(&zone.rule, 2026).unwrap()
    );
    assert!(from_changes(&zone, now).unwrap().is_some());
}

/// Broken zone files are refused, not misread.
#[test]
fn broken_zone_files_are_refused() {
    let good = real_zone_bytes("Dublin");
    // Cut inside the 64-bit list.
    let mut cut = good[..good.len() / 2].to_vec();
    cut.extend_from_slice(b"\nIST-1GMT0,M10.5.0,M3.5.0/1\n");
    assert!(parse_tzif(&cut).is_err());
    assert!(parse_tzif(b"TZif2\nUTC0\n").is_err());
}

/// `TZ` as the C library takes it, and the zone files read with care: a POSIX rule; `:file` and
/// a full path; a name in the zone folder; no `TZ`, the local zone file; a missing file, UTC; a
/// folder or a file over 1 MB refused.
#[test]
fn tz_is_read_as_the_c_library_takes_it() {
    let dir = std::env::temp_dir().join(format!("nv-rs-zone-test-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("Europe")).unwrap();
    let dublin = real_zone_bytes("Dublin");
    std::fs::write(dir.join("Europe").join("Dublin"), &dublin).unwrap();
    std::fs::write(dir.join("localtime"), &dublin).unwrap();
    std::fs::write(dir.join("big"), vec![0u8; (1 << 20) + 1]).unwrap();
    let local = dir.join("localtime");
    let read = |tz: Option<&str>| read_zone(tz, &dir, &local);
    assert_eq!(read(Some("JST-9")).unwrap().rule.offset, -9 * 3600);
    assert_eq!(read(Some("Europe/Dublin")).unwrap().rule.name, "IST");
    let full = format!(":{}", dir.join("Europe").join("Dublin").display());
    assert_eq!(read(Some(&full)).unwrap().changes.len(), 228);
    assert_eq!(read(None).unwrap().rule.name, "IST");
    assert_eq!(read(Some("")).unwrap().rule.name, "IST");
    assert_eq!(read(Some("Nowhere/Town")).unwrap().rule.name, "UTC");
    assert!(read(Some("Europe"))
        .unwrap_err()
        .contains("not a regular file"));
    assert!(read(Some("big")).unwrap_err().contains("larger than 1 MB"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The folders are nv-rs's own, on drive Z:, as the game's ANSI bytes.
#[test]
fn the_folders_are_nv_rs_own_on_drive_z() {
    let lines = folder_lines(std::path::Path::new("/home/me/.local/share/nv-rs")).unwrap();
    let decoded: Vec<(String, String)> = lines
        .lines()
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert_eq!((f[0], f[2]), ("SHGetFolderPathA", "0"));
            let bytes: Vec<u8> = (0..f[3].len() / 2)
                .map(|i| u8::from_str_radix(&f[3][2 * i..2 * i + 2], 16).unwrap())
                .collect();
            (f[1].to_string(), String::from_utf8(bytes).unwrap())
        })
        .collect();
    assert_eq!(
        decoded,
        [
            ("5 0", r"Z:\home\me\.local\share\nv-rs\game\Documents"),
            (
                "1a 0",
                r"Z:\home\me\.local\share\nv-rs\game\AppData\Roaming"
            ),
            ("1c 0", r"Z:\home\me\.local\share\nv-rs\game\AppData\Local"),
        ]
        .map(|(a, b)| (a.to_string(), b.to_string()))
    );
    assert!(folder_lines(std::path::Path::new("relative/nv-rs")).is_err());
    let long = format!("/{}", "x".repeat(200));
    assert!(folder_lines(std::path::Path::new(&long)).is_err());
}

/// The whole table reads back through the Windows table's own reader with every answer in place.
#[test]
fn the_table_reads_back_with_its_answers() {
    let (table, problems) = write_table(&Inputs {
        zone: parse_posix_tz("CET-1CEST,M3.5.0,M10.5.0/3").map(Zone::from_rule),
        now: at(2026, 7, 1, 12),
        processor: fixed_processor(),
        own_folder: Ok("/home/me/.local/share/nv-rs".into()),
    });
    assert!(problems.is_empty(), "{problems:?}");
    let mut e = crate::Engine::new();
    crate::d3d9::load_adapter_table(&mut e, &table);
    assert_eq!(crate::d3d9::get_acp(&mut e), 1252);
    assert_eq!(crate::d3d9::get_oemcp(&mut e), 437);
    assert_eq!(crate::d3d9::is_valid_code_page(&mut e, 65001), 1);
    assert_eq!(crate::d3d9::system_metric(&mut e, 0x1000), Some(0));
    assert_eq!(crate::d3d9::system_metric(&mut e, 0), None);
    assert_eq!(crate::d3d9::windows_directory(&mut e), b"C:\\WINDOWS");
    assert_eq!(
        crate::d3d9::folder_path(&mut e, 0x23),
        Some((0, b"C:\\ProgramData".to_vec()))
    );
    assert_eq!(
        crate::d3d9::folder_path(&mut e, 5).unwrap().1,
        b"Z:\\home\\me\\.local\\share\\nv-rs\\game\\Documents"
    );
    assert_eq!(
        crate::d3d9::window_style(&mut e, 0x1000_0000, 0),
        Some((0x14c0_0000, 0x100))
    );
    assert_eq!(crate::d3d9::cpu_id_flag_test(&mut e), 0x0020_0000);
    let zone = ZoneInfo::from_bytes(&crate::d3d9::time_zone(&mut e));
    assert_eq!(zone.bias, -60);
    assert_eq!(
        crate::d3d9::time_zone_for_year(&mut e, 1601),
        crate::d3d9::time_zone_for_year(&mut e, 30827)
    );
    // No NVIDIA library: LoadLibraryA fails with ERROR_MOD_NOT_FOUND.
    assert_eq!(crate::nvapi::load_library(&mut e), 0);
    assert_eq!(e.kernel.last_error, 126);
}

/// A part that cannot be read is named, and the rest of the table is still made.
#[test]
fn a_missing_part_is_named_and_the_rest_is_made() {
    let (table, problems) = write_table(&Inputs {
        zone: Err("no zone file".into()),
        now: 0,
        processor: fixed_processor(),
        own_folder: Err("no HOME".into()),
    });
    assert_eq!(problems.len(), 2);
    assert!(problems[0].contains("no HOME") && problems[1].contains("no zone file"));
    assert!(!table.contains("GetTimeZoneInformation") && !table.contains("SHGetFolderPathA\t5 "));
    assert!(table.contains("CpuIdFlagTest\t\t200000\nCpuid\t0\t1 0 0 0\nCpuid\t1\t0 0 0 6808000\n"));
    assert!(table.ends_with("NvApiLoad\t\t0\t7e\nNvApiEnd\n"));
}

/// The fixed Windows answers hold nothing personal: no user folder, no adapter, no NVIDIA lines,
/// no display, only the kinds the module names.
#[test]
fn the_fixed_answers_are_only_the_named_kinds() {
    for line in WINDOWS_FIXED.lines() {
        let kind = line.split('\t').next().unwrap();
        assert!(
            [
                "GetSystemMetrics",
                "CreateWindowStyle",
                "AdjustWindowRectEx",
                "GetWindowsDirectoryA",
                "SHGetFolderPathA",
                "GetACP",
                "GetOEMCP",
                "IsValidCodePage",
                "GetCPInfo",
            ]
            .contains(&kind),
            "{line}"
        );
        if kind == "SHGetFolderPathA" {
            assert!(
                line.starts_with("SHGetFolderPathA\t23 0\t")
                    || line.starts_with("SHGetFolderPathA\t2e 0\t")
            );
        }
        if kind == "GetSystemMetrics" {
            assert_eq!(line, "GetSystemMetrics\t1000\t0");
        }
    }
}

/// On Linux and macOS: the system's own answers read (CI's runners).
#[cfg(not(windows))]
mod this_system {
    use super::*;

    #[test]
    fn this_systems_zone_and_memory_are_read() {
        let zone = this_zone().unwrap();
        assert!(
            zone_information_at(&zone, at(2026, 7, 1, 0)).is_ok(),
            "{zone:?}"
        );
        let m = this_memory().unwrap();
        assert!(m.total_phys > 0 && m.avail_phys <= m.total_phys, "{m:?}");
        assert!(m.load <= 100 && m.total_page_file >= m.total_phys, "{m:?}");
    }

    #[test]
    fn this_processor_answers_sse2() {
        let (id, _, leaf_1) = this_processor();
        assert_ne!(id, 0);
        assert_ne!(leaf_1[3] & (1 << 26), 0);
        #[cfg(not(target_arch = "x86_64"))]
        assert_eq!((id, leaf_1), (fixed_processor().0, fixed_processor().2));
    }

    /// The system's own zone files for the cases above (where the system has them): each read,
    /// answered for 2025 to 2030, and the offsets in force in July and January 2026.
    #[test]
    fn the_systems_zone_files_are_answered() {
        // (zone, minutes to add to local time for UTC on 15 July and on 15 January 2026).
        for (name, july, january) in [
            ("America/New_York", 240, 300),
            ("Asia/Jerusalem", -180, -120),
            ("Australia/Sydney", -600, -660),
            ("Asia/Tokyo", -540, -540),
            ("Asia/Kolkata", -330, -330),
            ("Asia/Kathmandu", -345, -345),
            ("Pacific/Chatham", -765, -825),
            ("America/Santiago", 240, 180),
            ("Australia/Lord_Howe", -630, -660),
            ("America/Nuuk", 60, 120),
            ("Europe/Dublin", -60, 0),
        ] {
            let path = zone_folder().join(name);
            let Ok(Ok(bytes)) = read_zone_file(&path) else {
                eprintln!("{}: not on this system", path.display());
                continue;
            };
            let zone = parse_tzif(&bytes).unwrap_or_else(|why| panic!("{name}: {why}"));
            for year in 2025..=2030 {
                zone_information_at(&zone, at(year, 7, 1, 0))
                    .unwrap_or_else(|why| panic!("{name}: {why}"));
            }
            // The offsets in force, whichever form the file has (Dublin's "negative" daylight time,
            // or Windows' own form where the system's files use it, as macOS's do).
            let z = ZoneInfo::from_bytes(&zone_information_at(&zone, at(2026, 7, 1, 0)).unwrap());
            let found = (
                zone::bias_at(&z, at(2026, 7, 15, 12)),
                zone::bias_at(&z, at(2026, 1, 15, 12)),
            );
            assert_eq!(found, (july, january), "{name}");
        }
    }

    #[test]
    fn this_machines_table_has_no_problems() {
        let (table, problems) = this_machine(Ok("/nonexistent/nv-rs".into()));
        assert!(problems.is_empty(), "{problems:?}");
        assert!(table.contains("GetTimeZoneInformation\t\t"));
    }
}
