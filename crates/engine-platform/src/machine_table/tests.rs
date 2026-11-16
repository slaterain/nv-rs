use super::*;

fn zone_of(tz: &str, year: i64) -> ZoneInfo {
    let zone = parse_posix_tz(tz).unwrap();
    ZoneInfo::from_bytes(&time_zone_information(&zone, year).unwrap())
}

/// Seconds since 1970 of a UTC date and hour.
fn at(year: i64, month: u32, day: u32, hour: i64) -> i64 {
    zone::days_from_civil(year, month, day) * 86_400 + hour * 3600
}

/// US Eastern gives what Windows' "Eastern Standard Time" answers on this PC (Bias 300, daylight
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
    let lines = zone_lines(&zone, at(2026, 7, 1, 0)).unwrap();
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
    let zone = parse_posix_tz("EST5EDT,M3.2.0,M11.1.0").unwrap();
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
    file[4] = 0;
    assert!(tzif_footer(&file).is_err());
    assert!(tzif_footer(b"TZif3\x00\n\n").is_err());
    assert!(tzif_footer(b"TZif3\x00\nUTC0").is_err());
    assert!(tzif_footer(b"MZ\x00\nUTC0\n").is_err());
}

#[test]
fn memory_from_linux_meminfo() {
    let text = "MemTotal:       16000000 kB\nMemFree:         1000000 kB\n\
                MemAvailable:    7040000 kB\nSwapTotal:       2000000 kB\n\
                Committed_AS:   12000000 kB\n";
    let m = memory_from_meminfo(text).unwrap();
    assert_eq!(m.total_phys, 16_000_000 * 1024);
    assert_eq!(m.avail_phys, 7_040_000 * 1024);
    // (16,000,000 - 7,040,000) / 16,000,000 = 56 %.
    assert_eq!(m.load, 56);
    assert_eq!(m.total_page_file, 18_000_000 * 1024);
    assert_eq!(m.avail_page_file, 6_000_000 * 1024);
    // More committed than the limit: nothing available, not a wrapped number.
    let over = text.replace("12000000", "19000000");
    assert_eq!(memory_from_meminfo(&over).unwrap().avail_page_file, 0);
    assert_eq!(memory_from_meminfo("MemTotal: 1 kB\n"), None);
    assert_eq!(
        memory_from_meminfo(&text.replace(" kB\nSwap", "\nSwap")),
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
    assert_eq!(memory_from_sysctl(16 << 30, 250, 0, 5).avail_phys, 16 << 30);
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
        zone: parse_posix_tz("CET-1CEST,M3.5.0,M10.5.0/3"),
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
        assert!(time_zone_information(&zone, 2026).is_ok(), "{zone:?}");
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

    /// The system's own zone files for the cases above (where the system has them).
    #[test]
    fn the_systems_zone_files_are_answered() {
        for name in [
            "America/New_York",
            "Asia/Jerusalem",
            "Australia/Sydney",
            "Asia/Tokyo",
            "Asia/Kolkata",
            "Asia/Kathmandu",
            "Pacific/Chatham",
        ] {
            let path = zone_folder().join(name);
            let Ok(bytes) = std::fs::read(&path) else {
                eprintln!("{}: not on this system", path.display());
                continue;
            };
            let zone = tzif_footer(&bytes).and_then(parse_posix_tz);
            let zone = zone.unwrap_or_else(|why| panic!("{name}: {why}"));
            for year in 2025..=2030 {
                time_zone_information(&zone, year).unwrap_or_else(|why| panic!("{name}: {why}"));
            }
        }
    }

    #[test]
    fn this_machines_table_has_no_problems() {
        let (table, problems) = this_machine(Ok("/nonexistent/nv-rs".into()));
        assert!(problems.is_empty(), "{problems:?}");
        assert!(table.contains("GetTimeZoneInformation\t\t"));
    }
}
