//! The Caravan menu's table on screen (`cellview::caravan`, run by
//! `game_menus::caravan`). Drawn as the game's `Draw3DElements`
//! (`00740f30`) draws it: the depth cleared, every model in one pass seen by
//! the table's camera, after the image space pass and under the menus'
//! pictures; here into the HUD's picture by a camera of its own before the
//! HUD's (which then blends over it, `game_menus::compose_hud_over_scene`).
//!
//! Each frame every piece is put where its model's pose has it (a hidden
//! node hides it; the deck screen's models only while that screen's in, the
//! game's only once the game is), the money on its spots, the card
//! textures the menu set on their shapes, the table's lights where its
//! camera rig has them.
//!
//! The camera keeps the file's frustum: its 45° across is kept and the
//! height follows the window (the file's is 16:9's), where the game keeps
//! both (stretched on another shape of screen).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::camera::{CameraOutputMode, Exposure, RenderTarget};
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::renderer::RenderDevice;
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;
use cellview::caravan::{CaravanModels, Part};
use cellview::space;
use nif::math::Transform as NifTransform;

use crate::game_menus::caravan::CaravanScreen;
use crate::game_menus::{GameMenus, OpenMenu};
use crate::hud::HudLayer;
use crate::lighting::GameLitMaterial;
use crate::GameFiles;

/// The render layer the table's pieces are drawn on.
const TABLE_LAYER: usize = 28;

/// A piece of the table: its model (index into the models' parts), the
/// money piece it belongs to, its mesh and shape, where its mesh is
/// centred.
#[derive(Component)]
struct TablePiece {
    part: usize,
    money: Option<usize>,
    shape: String,
    center: Vec3,
    material: Handle<GameLitMaterial>,
}

/// What's on screen.
struct Shown {
    models: Arc<CaravanModels>,
    camera: Entity,
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<GameLitMaterial>>,
    images: Vec<Handle<Image>>,
    textures: Vec<Option<Handle<Image>>>,
    /// Card textures by path.
    cards: HashMap<String, Option<Handle<Image>>>,
    /// The money pieces with pieces made.
    money: usize,
    texture_changes: u64,
    lights: Vec<cellview::LightData>,
}

#[derive(Resource, Default)]
pub struct TableShown {
    shown: Option<Shown>,
    layer: Option<Handle<Image>>,
}

pub struct CaravanTablePlugin;

impl Plugin for CaravanTablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TableShown>()
            .add_systems(Update, show_table.after(crate::menus::run_menus));
    }
}

/// The open Caravan menu, if any.
fn caravan(menus: &GameMenus) -> Option<&CaravanScreen> {
    menus.screen.as_deref()?.open.iter().find_map(|m| match m {
        OpenMenu::Caravan(c) => Some(&**c),
        _ => None,
    })
}

/// Where the camera puts the player's first grid's `Select1_01:0` across
/// the screen (0 to 1; `NiCamera::WorldPtToScreenPt` `00a6fc50`), once
/// the hands are dealt (state 2's end).
pub fn track_x(c: &CaravanScreen) -> Option<f32> {
    use world::caravan::menu::{state, Screen};
    if c.game.screen != Screen::Game || c.game.state == state::CAMERA_TO_GAME {
        return None;
    }
    let spot = c.models.grids.first()?.get(&(0, 0))?;
    let table = c.table.poses.get(&Part::Table).cloned().unwrap_or_default();
    let camera = c.models.camera(&table)?;
    let f = c.models.frustum?;
    // Into the camera's space: x ahead, y up, z right.
    let local = camera.inverse().apply_point(spot.center);
    if local[0] <= 1e-5 {
        return None;
    }
    let across = local[2] / local[0];
    Some((across - f.left) / (f.right - f.left))
}

/// The camera's Bevy transform from its place in the game's space
/// (Gamebryo cameras look along their +x with +y up).
fn camera_transform(t: &NifTransform) -> Transform {
    let origin = t.apply_point([0.0; 3]);
    let along = |v: [f32; 3]| {
        let p = t.apply_point(v);
        Vec3::from(space::direction([
            p[0] - origin[0],
            p[1] - origin[1],
            p[2] - origin[2],
        ]))
        .normalize_or_zero()
    };
    let forward = along([1.0, 0.0, 0.0]);
    let up = along([0.0, 1.0, 0.0]);
    Transform::from_translation(Vec3::from(space::point(origin))).looking_to(forward, up)
}

#[allow(clippy::too_many_arguments)]
fn show_table(
    mut commands: Commands,
    game: Res<GameFiles>,
    settings: Res<crate::Settings>,
    menus: Res<GameMenus>,
    mut shown: ResMut<TableShown>,
    hud_layer: Option<Res<HudLayer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GameLitMaterial>>,
    mut images: ResMut<Assets<Image>>,
    device: Option<Res<RenderDevice>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut pieces: Query<(&TablePiece, &mut Transform, &mut Visibility)>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<TablePiece>)>,
) {
    let open = caravan(&menus);
    // While the table's drawn the HUD's camera lays its pictures over it
    // (blended): `game_menus::compose_hud_over_scene`, for every menu with
    // a 3D scene.
    let TableShown { shown, layer } = &mut *shown;
    let stale = match (&*shown, open) {
        (Some(s), Some(c)) => !Arc::ptr_eq(&s.models, &c.models),
        (Some(_), None) => true,
        _ => false,
    };
    if stale {
        if let Some(s) = shown.take() {
            for e in s.entities {
                commands.entity(e).despawn();
            }
            for m in s.meshes {
                meshes.remove(&m);
            }
            for m in s.materials {
                materials.remove(&m);
            }
            for i in s.images {
                images.remove(&i);
            }
        }
    }
    let Some(c) = open else {
        return;
    };
    let size = windows
        .single()
        .map(|w| UVec2::new(w.physical_width(), w.physical_height()))
        .unwrap_or(UVec2::new(1920, 1080))
        .max(UVec2::ONE);
    let compressed = device
        .as_ref()
        .is_none_or(|d| d.features().contains(WgpuFeatures::TEXTURE_COMPRESSION_BC));
    let models = c.models.clone();
    let table_pose = c.table.poses.get(&Part::Table).cloned().unwrap_or_default();
    let lights = models.lights_now(&table_pose);
    if shown.is_none() {
        let target = crate::lockpick::menu_layer(
            &mut commands,
            hud_layer.as_deref(),
            layer,
            &mut images,
            size,
        );
        let frustum = models.frustum;
        let (fov, near, far) = frustum
            .map_or((45f32.to_radians() * 9.0 / 16.0, 1.0, 5000.0), |f| {
                (2.0 * f.top.atan(), f.near, f.far)
            });
        let camera = commands
            .spawn((
                Camera3d::default(),
                Camera {
                    target: RenderTarget::from(target),
                    // Before the HUD's camera (−4), which draws the menus'
                    // pictures over it.
                    order: -5,
                    hdr: true,
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    // The picture cleared, the table written into it.
                    output_mode: CameraOutputMode::Write {
                        blend_state: None,
                        clear_color: ClearColorConfig::Custom(Color::NONE),
                    },
                    ..default()
                },
                Tonemapping::None,
                DebandDither::Disabled,
                Projection::from(PerspectiveProjection {
                    fov,
                    near: near * space::METERS_PER_UNIT,
                    far: far * space::METERS_PER_UNIT,
                    ..default()
                }),
                Exposure {
                    ev100: crate::START_EV100,
                },
                Transform::IDENTITY,
                RenderLayers::layer(TABLE_LAYER),
            ))
            .id();
        let textures: Vec<Option<Handle<Image>>> = models
            .scene
            .textures
            .iter()
            .map(|t| crate::upload_texture(&mut images, t, compressed, settings.anisotropy))
            .collect();
        let mut s = Shown {
            models: models.clone(),
            camera,
            entities: vec![camera],
            meshes: Vec::new(),
            materials: Vec::new(),
            images: textures.iter().flatten().cloned().collect(),
            textures,
            cards: HashMap::new(),
            money: 0,
            texture_changes: u64::MAX,
            lights: lights.clone(),
        };
        // Every model's pieces but the money's (made per piece below).
        let draws: Vec<usize> = (0..models.scene.draws.len()).collect();
        spawn_pieces(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut s,
            &draws,
            None,
            &lights,
            settings.brightness,
        );
        *shown = Some(s);
    }
    let Some(s) = shown.as_mut() else {
        return;
    };
    // New money: its kind's pieces.
    while s.money < c.table.money.len() {
        let part = c.table.money[s.money].part;
        let index = models.parts.iter().position(|&p| p == part);
        let draws: Vec<usize> = models
            .scene
            .draws
            .iter()
            .enumerate()
            .filter(|(_, d)| Some(d.reference as usize) == index.map(|i| i + 1))
            .map(|(i, _)| i)
            .collect();
        let money = s.money;
        spawn_pieces(
            &mut commands,
            &mut meshes,
            &mut materials,
            s,
            &draws,
            Some(money),
            &lights,
            settings.brightness,
        );
        s.money += 1;
    }
    // The camera.
    if let Some(t) = models.camera(&table_pose) {
        if let Ok(mut transform) = cameras.get_mut(s.camera) {
            let wanted = camera_transform(&t);
            if *transform != wanted {
                *transform = wanted;
            }
        }
    }
    // The lights move with the camera rig.
    if s.lights != lights {
        s.lights = lights.clone();
        let lighting = crate::lockpick::menu_lighting(&lights, settings.brightness);
        for m in &s.materials {
            if let Some(mat) = materials.get_mut(m) {
                mat.extension.lighting = lighting;
            }
        }
    }
    // Card textures.
    let textures_changed = s.texture_changes != c.table.texture_changes;
    s.texture_changes = c.table.texture_changes;
    // The pieces.
    for (piece, mut transform, mut visibility) in &mut pieces {
        let part = models.parts[piece.part];
        let attached = match part {
            Part::Table => true,
            Part::Deck | Part::Available => c.table.deck_models,
            Part::Bill(_) | Part::Coin(_) => piece.money.is_some(),
            _ => c.table.game_models,
        };
        let pose = match piece.money {
            Some(i) => c.table.money.get(i).map(|m| &m.pose),
            None => c.table.poses.get(&part),
        };
        let default = cellview::caravan::Pose::default();
        let pose = pose.unwrap_or(&default);
        let model = &models.models[piece.part];
        let moved = attached
            .then(|| pose.shape_move(model, &piece.shape))
            .flatten();
        let wanted_visibility = if moved.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted_visibility {
            *visibility = wanted_visibility;
        }
        if let Some(m) = moved {
            let matrix = cellview::column_major(&m);
            let wanted = Transform::from_matrix(
                Mat4::from_cols_array(&space::matrix(&matrix))
                    * Mat4::from_translation(piece.center),
            );
            if *transform != wanted {
                *transform = wanted;
            }
        }
        if textures_changed {
            let key = (part, piece.shape.to_ascii_lowercase());
            if let Some(path) = c.table.textures.get(&key) {
                let handle = s
                    .cards
                    .entry(path.clone())
                    .or_insert_with(|| {
                        cellview::caravan::card_texture(&game.0.assets, path).and_then(|t| {
                            crate::upload_texture(&mut images, &t, compressed, settings.anisotropy)
                        })
                    })
                    .clone();
                if let (Some(h), Some(mat)) = (handle, materials.get_mut(&piece.material)) {
                    if mat.base.base_color_texture.as_ref() != Some(&h) {
                        mat.base.base_color_texture = Some(h);
                    }
                }
            }
        }
    }
}

/// Pieces for draws (of the models, or of one money piece).
#[allow(clippy::too_many_arguments)]
fn spawn_pieces(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GameLitMaterial>,
    s: &mut Shown,
    draws: &[usize],
    money: Option<usize>,
    lights: &[cellview::LightData],
    brightness: f32,
) {
    let models = s.models.clone();
    let lighting = crate::lockpick::menu_lighting(lights, brightness);
    for &d in draws {
        let draw = &models.scene.draws[d];
        let Some(part) = (draw.reference as usize).checked_sub(1) else {
            continue;
        };
        // The money's templates are drawn only as money.
        if money.is_none() && matches!(models.parts.get(part), Some(Part::Bill(_) | Part::Coin(_)))
        {
            continue;
        }
        let data = &models.scene.meshes[draw.mesh];
        let center = crate::sort_center(data).unwrap_or([0.0; 3]);
        let mesh = meshes.add(crate::game_mesh_around(data, center));
        let material = materials.add(crate::lit_material(data, &s.textures, lighting));
        s.meshes.push(mesh.clone());
        s.materials.push(material.clone());
        let e = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                Visibility::Hidden,
                RenderLayers::layer(TABLE_LAYER),
                crate::shared_light::MenuLit,
                TablePiece {
                    part,
                    money,
                    shape: data.shape_name.clone(),
                    center: Vec3::from(center),
                    material,
                },
            ))
            .id();
        s.entities.push(e);
    }
}
