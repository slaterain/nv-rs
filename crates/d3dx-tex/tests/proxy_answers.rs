//! The crate against the 32-bit `d3dx9_38.dll` on devices that take other formats than the
//! reference machine's: `data/proxy_32bit.tsv` (`proxy_32bit_double.tsv` on a thread at 53 bits),
//! captured with `scripts/d3dx-tex-verify --capture-proxy` through `d3dx-measure serve proxy` (a
//! proxy of a real device whose `CheckDeviceFormat` answers from given lists). Six devices: one
//! with only the 16-bit and DXT formats (a cube list of its own), one taking every format of D3DX's
//! table, four with random lists. For each, D3DX's pick for every format of its table as a texture
//! and as a cube, and the textures it makes of the synthetic files there: conversions into
//! R5G6B5 and A1R5G5B5 with the dither, X4R4G4B4 kept, the cube list used for cube maps.

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap()
}

fn list(s: &str) -> Vec<u32> {
    s.split(',').filter(|w| !w.is_empty()).map(hex).collect()
}

#[test]
fn d3dx_on_other_devices_is_the_32_bit_dlls() {
    proxy(
        include_str!("../data/proxy_32bit.tsv"),
        d3dx_tex::Precision::Single,
    );
}

#[test]
fn d3dx_on_other_devices_is_the_32_bit_dlls_at_double_precision() {
    proxy(
        include_str!("../data/proxy_32bit_double.tsv"),
        d3dx_tex::Precision::Double,
    );
}

fn proxy(data: &str, precision: d3dx_tex::Precision) {
    let files: Vec<d3dx_tex::synthetic::File> = d3dx_tex::synthetic::files()
        .into_iter()
        .chain(d3dx_tex::synthetic::formats())
        .collect();
    let mut caps: Option<d3dx_tex::Caps> = None;
    let (mut picks, mut made, mut left) = (0, 0, 0);
    // (device, file, texture format) of every texture made.
    let mut textures = std::collections::HashSet::new();
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "device" => {
                caps = Some(d3dx_tex::Caps {
                    texture_formats: list(f[2]),
                    cube_formats: list(f[3]),
                    ..d3dx_tex::Caps::default()
                });
            }
            "pick" => {
                let c = caps.as_ref().expect("a device first");
                let format = hex(f[2]);
                for (takes, hr, pick) in [
                    (&c.texture_formats, hex(f[3]), hex(f[4])),
                    (&c.cube_formats, hex(f[5]), hex(f[6])),
                ] {
                    let ours = d3dx_tex::choose(format, |x| takes.contains(&x));
                    if hr == 0 {
                        assert_eq!(ours, pick, "device {}: {format:#x}", f[1]);
                    } else {
                        assert_eq!((ours, hr), (0, d3dx_tex::D3DERR_NOTAVAILABLE), "{line}");
                    }
                    picks += 1;
                }
            }
            "file" => {
                let c = caps.as_ref().expect("a device first");
                let file = files
                    .iter()
                    .find(|x| x.name == f[2])
                    .expect("a synthetic file");
                let hr = hex(f[4]);
                let ours = if file.cube {
                    d3dx_tex::create_cube_texture_from_file_in_memory(&file.data, c, precision)
                } else {
                    d3dx_tex::create_texture_from_file_in_memory(&file.data, c, precision)
                };
                let at = format!("device {}: {}", f[1], file.name);
                match ours {
                    Err(d3dx_tex::Error::D3dx(ours)) => assert_eq!(ours, hr, "{at}"),
                    // A texture format this crate does not make: the DLL made one.
                    Err(d3dx_tex::Error::NotProvided(d3dx_tex::Untranslated::PixelFormat, why)) => {
                        assert_eq!(hr, 0, "{at}: {why}");
                        let theirs = hex(f[7].split(',').next().unwrap());
                        assert!(!d3dx_tex::filter::makes(theirs), "{at}: {why}");
                        left += 1;
                    }
                    Err(d3dx_tex::Error::NotProvided(_, why)) => {
                        panic!("{at}: not provided: {why}")
                    }
                    Ok(t) => {
                        assert_eq!(hr, 0, "{at}");
                        assert_eq!(
                            (t.levels.to_string(), t.faces.to_string()),
                            (f[5].to_string(), f[6].to_string()),
                            "{at}"
                        );
                        let ours: Vec<String> = t
                            .surfaces
                            .iter()
                            .map(|s| {
                                format!(
                                    "{:x},{},{},{:016x}",
                                    s.format,
                                    s.width,
                                    s.height,
                                    d3dx_tex::synthetic::fnv(&s.bytes)
                                )
                            })
                            .collect();
                        assert_eq!(ours.join(";"), f[7], "{at}");
                        textures.insert((
                            f[1].to_string(),
                            file.name.clone(),
                            t.surfaces[0].format,
                        ));
                        made += 1;
                    }
                }
            }
            other => panic!("a line of kind {other}"),
        }
    }
    // Six devices, every format of D3DX's table as a texture and as a cube; most files made.
    assert_eq!(picks, 6 * 57 * 2);
    assert!(made > 3 * left, "{made} made, {left} left to the caller");
    // What only these devices reach: on the 16-bit one, A8R8G8B8 made A1R5G5B5 and X8R8G8B8 made
    // R5G6B5 (the dithered encoders); on the one taking everything, X4R4G4B4 kept.
    for (device, file, format) in [
        ("0", "a8r8g8b8-32-one", 25),
        ("0", "x8r8g8b8-32x8-one", 23),
        ("1", "x4r4g4b4-32-one", 30),
        ("1", "x4r4g4b4-64x1-edges-one", 30),
    ] {
        let key = (device.to_string(), file.to_string(), format);
        assert!(textures.contains(&key), "{key:?} not made");
    }
}
