//! Where an F12 report was taken (`place.txt` in the report's folder, and one line in the log): the
//! worldspace or interior cell (editor id and form id), the grid cell, the player's position and
//! view angles, the game hour, the weather and the lighting switches. Written by the host's main
//! loop between two of the game's frames when a report waits for it
//! (`engine_platform::d3d9real::report_place_wanted`); read only, nothing in the game changes.

use engine::Engine;

/// `PlayerCharacter` (the pointer), the reference's rotation (x, y, z radians) and position, its
/// parent cell (`008d6f30`).
const PLAYER: u32 = 0x011d_ea3c;
const ROTATION: u32 = 0x24;
const POSITION: u32 = 0x30;
const PARENT_CELL: u32 = 0x40;
/// A form's id; `GetEditorID` (virtual at +0x130, a C string).
const FORM_ID: u32 = 0x0c;
const GET_EDITOR_ID: u32 = 0x130;
/// `TESObjectCELL`: its flags (bit 0 interior, `00425fd0`) and worldspace (`0054ddd0`).
const CELL_FLAGS: u32 = 0x24;
const CELL_WORLD_SPACE: u32 = 0xc0;
/// The calendar (`Calendar::GetHour`, `00867da0`: its `GameHour` global at +0x0c, the global's
/// value at +0x24; 12 without one).
const CALENDAR: u32 = 0x011d_e7b8;
const CALENDAR_HOUR: u32 = 0x0c;
const GLOBAL_VALUE: u32 = 0x24;
/// `Sky::pInstance` and its current weather (+0x10, `0044edb0`).
const SKY: u32 = 0x011c_cb78;
const SKY_WEATHER: u32 = 0x10;
/// A grid cell's side in game units.
const CELL_SIZE: f32 = 4096.0;
/// The lighting switches named in the place (the process's settings).
const SWITCHES: [&str; 8] = [
    "NV_D3D9",
    "NV_WGPU_BACKEND",
    "NV_AO",
    "NV_AO_STRENGTH",
    "NV_SUN_SHADOWS",
    "NV_SUN_SHADOWS_CHARACTERS",
    "NV_SUN_LIGHT_FROM_SKY",
    "NV_PRESET",
];

/// A form as the place names it: its editor id (none: empty) and form id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Named {
    pub editor_id: String,
    pub form_id: u32,
}

/// What `place.txt` says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    /// The player's cell and whether it is an interior; its worldspace (exteriors).
    pub cell: Option<Named>,
    pub interior: bool,
    pub world_space: Option<Named>,
    /// Position (game units) and rotation (radians, x pitch and z heading as the game keeps them).
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub hour: f32,
    pub weather: Option<Named>,
    /// The lighting switches set (name, value) and the renderer's own states.
    pub switches: Vec<(String, String)>,
}

/// The exterior grid cell of a position (as the game's `coc`/`cow` count them).
pub fn grid(position: [f32; 3]) -> (i32, i32) {
    (
        (position[0] / CELL_SIZE).floor() as i32,
        (position[1] / CELL_SIZE).floor() as i32,
    )
}

/// A form's name as plain text: `EDID (0x0001abcd)`, or the id alone.
fn named(n: &Option<Named>) -> String {
    match n {
        None => "none".into(),
        Some(n) if n.editor_id.is_empty() => format!("{:#010x}", n.form_id),
        Some(n) => format!("{} ({:#010x})", n.editor_id, n.form_id),
    }
}

/// `place.txt`'s lines (`\n` ends; the report writes Windows line ends), with the console's own
/// words to stand there again (`cow`/`coc`, `player.setpos`, `player.setangle`, `set gamehour`).
pub fn text(p: &Place) -> String {
    let (gx, gy) = grid(p.position);
    let degrees = p.rotation.map(f32::to_degrees);
    let mut out = String::new();
    if p.interior {
        out += &format!("interior cell: {}\n", named(&p.cell));
    } else {
        out += &format!("worldspace: {}\n", named(&p.world_space));
        out += &format!("cell: {} at grid ({gx}, {gy})\n", named(&p.cell));
    }
    out += &format!(
        "position: {:.0} {:.0} {:.0}\n",
        p.position[0], p.position[1], p.position[2]
    );
    out += &format!(
        "view: pitch {:.1} heading {:.1} (degrees)\n",
        degrees[0], degrees[2]
    );
    out += &format!("game hour: {:.2}\n", p.hour);
    out += &format!("weather: {}\n", named(&p.weather));
    let switches: Vec<String> = p.switches.iter().map(|(k, v)| format!("{k}={v}")).collect();
    out += &format!("lighting: {}\n", switches.join(" "));
    // To stand there again from the console.
    let to = match (&p.interior, &p.cell, &p.world_space) {
        (true, Some(c), _) if !c.editor_id.is_empty() => format!("coc {}", c.editor_id),
        (false, _, Some(w)) if !w.editor_id.is_empty() => format!("cow {} {gx} {gy}", w.editor_id),
        _ => String::new(),
    };
    out += &format!(
        "console: {to}{}player.setpos x {:.0}; player.setpos y {:.0}; player.setpos z {:.0}; \
         player.setangle x {:.1}; player.setangle z {:.1}; set gamehour to {:.2}\n",
        if to.is_empty() { "" } else { "; " },
        p.position[0],
        p.position[1],
        p.position[2],
        degrees[0],
        degrees[2],
        p.hour
    );
    out
}

/// A guest C string (at most 256 bytes).
fn guest_text(e: &mut Engine, at: u32) -> String {
    if at == 0 {
        return String::new();
    }
    let bytes: Vec<u8> = (0..256)
        .map(|i| e.mem.u8(at.wrapping_add(i)))
        .take_while(|&b| b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The form at `form` named (none for a null pointer).
fn form(e: &mut Engine, form: u32) -> Option<Named> {
    if form == 0 {
        return None;
    }
    let name = e.vcall(form, GET_EDITOR_ID, &[]).u32();
    Some(Named {
        editor_id: guest_text(e, name),
        form_id: e.mem.u32(form.wrapping_add(FORM_ID)),
    })
}

/// The place read from the game (none before there is a player).
pub fn read(e: &mut Engine) -> Option<Place> {
    let player = e.mem.u32(PLAYER);
    if player == 0 {
        return None;
    }
    let position = [0, 4, 8].map(|o| e.mem.f32(player + POSITION + o));
    let rotation = [0, 4, 8].map(|o| e.mem.f32(player + ROTATION + o));
    let cell_at = e.mem.u32(player + PARENT_CELL);
    let interior = cell_at != 0 && e.mem.u8(cell_at + CELL_FLAGS) & 1 != 0;
    let world_at = if cell_at == 0 || interior {
        0
    } else {
        e.mem.u32(cell_at + CELL_WORLD_SPACE)
    };
    let global = e.mem.u32(CALENDAR + CALENDAR_HOUR);
    let hour = if global == 0 {
        12.0
    } else {
        e.mem.f32(global + GLOBAL_VALUE)
    };
    let sky = e.mem.u32(SKY);
    let weather_at = if sky == 0 {
        0
    } else {
        e.mem.u32(sky + SKY_WEATHER)
    };
    let mut switches: Vec<(String, String)> = SWITCHES
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| ((*k).to_string(), v)))
        .collect();
    switches.push((
        "new lighting".into(),
        d3d9_wgpu::scene_light::enabled().to_string(),
    ));
    switches.push((
        "sun shadows".into(),
        d3d9_wgpu::sun_shadow::enabled().to_string(),
    ));
    switches.push((
        "characters cast".into(),
        d3d9_wgpu::sun_shadow::characters().to_string(),
    ));
    Some(Place {
        cell: form(e, cell_at),
        interior,
        world_space: form(e, world_at),
        position,
        rotation,
        hour,
        weather: form(e, weather_at),
        switches,
    })
}

/// Between two of the game's frames: when an F12 report waits for its place, it is written.
pub fn note(e: &mut Engine) {
    if !engine_platform::d3d9real::report_place_wanted() {
        return;
    }
    let text = read(e).map_or_else(|| "no player yet\n".to_string(), |p| text(&p));
    engine_platform::d3d9real::set_report_place(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quarry() -> Place {
        Place {
            cell: Some(Named {
                editor_id: String::new(),
                form_id: 0x000d_e4c2,
            }),
            interior: false,
            world_space: Some(Named {
                editor_id: "WastelandNV".into(),
                form_id: 0x000d_a726,
            }),
            position: [-39000.4, 18200.0, 5800.0],
            rotation: [0.2618, 0.0, std::f32::consts::FRAC_PI_2],
            hour: 17.5,
            weather: Some(Named {
                editor_id: "WeatherClear".into(),
                form_id: 0x0000_015e,
            }),
            switches: vec![("NV_AO".into(), "1".into())],
        }
    }

    #[test]
    fn the_grid_cell_counts_as_cow_does() {
        assert_eq!(grid([-39000.0, 18200.0, 0.0]), (-10, 4));
        assert_eq!(grid([0.0, 4095.9, 0.0]), (0, 0));
        assert_eq!(grid([-0.5, 4096.0, 0.0]), (-1, 1));
    }

    #[test]
    fn an_exterior_place_names_the_worldspace_the_grid_and_how_to_stand_there() {
        let t = text(&quarry());
        let lines: Vec<&str> = t.lines().collect();
        assert_eq!(lines[0], "worldspace: WastelandNV (0x000da726)");
        assert_eq!(lines[1], "cell: 0x000de4c2 at grid (-10, 4)");
        assert_eq!(lines[2], "position: -39000 18200 5800");
        assert_eq!(lines[3], "view: pitch 15.0 heading 90.0 (degrees)");
        assert_eq!(lines[4], "game hour: 17.50");
        assert_eq!(lines[5], "weather: WeatherClear (0x0000015e)");
        assert_eq!(lines[6], "lighting: NV_AO=1");
        assert_eq!(
            lines[7],
            "console: cow WastelandNV -10 4; player.setpos x -39000; player.setpos y 18200; \
             player.setpos z 5800; player.setangle x 15.0; player.setangle z 90.0; set gamehour to 17.50"
        );
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn an_interior_place_names_its_cell_and_coc() {
        let mut p = quarry();
        p.interior = true;
        p.world_space = None;
        p.cell = Some(Named {
            editor_id: "GSDocMitchellHouse".into(),
            form_id: 0x0010_4c1c,
        });
        let t = text(&p);
        assert!(t.starts_with("interior cell: GSDocMitchellHouse (0x00104c1c)\n"));
        assert!(t.contains("console: coc GSDocMitchellHouse; player.setpos x -39000"));
        assert!(!t.contains("worldspace"));
    }

    #[test]
    fn a_form_without_an_editor_id_is_its_id_and_none_is_none() {
        let mut p = quarry();
        p.weather = None;
        p.world_space = Some(Named {
            editor_id: String::new(),
            form_id: 0x0100_0d7e,
        });
        let t = text(&p);
        assert!(t.contains("worldspace: 0x01000d7e\n"));
        assert!(t.contains("weather: none\n"));
        // No editor id: no cow line, the positions still.
        assert!(t.contains("console: player.setpos x -39000"));
    }
}
