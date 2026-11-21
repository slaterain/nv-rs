//! Share 1a's rows (the engines and the console; their bits in `engine_core::extend::stewie`'s
//! `world.rs`), in PLAN.md's order. Only share 1a adds rows here; the other shares add theirs in
//! `super::forms`, `super::ai` and `super::menus`.

use super::Row;
use engine_core::extend::stewie;

/// Share 1a's rows built.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::LOD_DATA_MISSING_WARNING,
        function: "patchAddMissingLODWarning",
        plan: "S0050",
        key: "bAddMissingLODWarning",
        tweaks: None,
        summary: "a reference with distant LOD that a plugin disabled no longer crashes the distant trees; Tweaks' console line names it and its plugin",
    },
    Row {
        bit: stewie::SUN_LIGHT_DIRECTION,
        function: "patchFixSunDirection",
        plan: "S0145",
        key: "bFixSunDirection",
        tweaks: None,
        summary: "changes the look: the sun's light comes from the side of the sky its disc is on (north or south), not the mirror side; nothing with the new lighting's light from the sky",
    },
    Row {
        bit: stewie::NAV_MESH_TELEPORT_DOOR,
        function: "NavMeshTeleportDoorFix",
        plan: "S0193",
        key: "bNavMeshTeleportDoorFix",
        tweaks: None,
        summary: "an NPC's path through a door whose other side has no way back no longer crashes: that door is not used",
    },
    Row {
        bit: stewie::STAIR_MATERIAL,
        function: "StairMaterialFix",
        plan: "S0216",
        key: "bStairMaterialFix",
        tweaks: None,
        summary: "changes how stairs walk: collision marked as stairs (123 of the game's own meshes) counts every riser as a step, so characters no longer catch on risers or get pushed back",
    },
    Row {
        bit: stewie::NAV_MESH_INFO_RESOLVE,
        function: "NavMeshInfoResolveFix",
        plan: "S0235",
        key: "bNavMeshInfoResolveFix",
        tweaks: None,
        summary: "a place whose navigation mesh cannot be worked out no longer crashes the pathing: it has none",
    },
    Row {
        bit: stewie::SWITCH_NODE_BOUNDS,
        function: "SwitchNodeBoundsFix",
        plan: "S0237",
        key: "bSwitchNodeBoundsFix",
        tweaks: None,
        summary: "a model with a switch or LOD node set to update only its shown part no longer crashes; its bounds follow the shown part",
    },
    Row {
        bit: stewie::SKINNED_DECAL_SUBTEXTURE,
        function: "SkinnedDecalSubtextureFix",
        plan: "S0238",
        key: "bSkinnedDecalSubtextureFix",
        tweaks: Some(("INI", "bSkinnedDecalSubtextureFix")),
        summary: "changes the look: blood and impact marks on bodies show the variant picked for them, not always the first one",
    },
    Row {
        bit: stewie::NAV_MESH_OBSTACLE_DOORS_ONLY,
        function: "NavMeshObstacleDoorTypeFix",
        plan: "S0242",
        key: "bNavMeshObstacleDoorTypeFix",
        tweaks: None,
        summary: "changes pathing slightly: only doors count as doors opening and closing for NPCs' paths",
    },
];
