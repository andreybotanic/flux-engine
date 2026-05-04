use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::render_asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

use crate::{
    config::{CellTypeVisualConfig, GasMainViewVisualConfig, GasRegistry},
    editor::StructureEditState,
    input::camera::MainCamera,
    save::WorldLoadState,
    simulation::{
        gas::{GasField, HYDROGEN_GPU_STORAGE_MAX_PARTICLES},
        SimulationStep,
    },
    ui::panels::PanelManager,
    world::grid::{
        cell_center, is_boundary, world_dimensions, world_to_cell, CellKind, CellMaterial,
        WorldGrid, CELL_SIZE, WORLD_HEIGHT, WORLD_WIDTH,
    },
    world::gas_structures::{GasStructureCell, GasStructureGrid},
    world::WorldCellChanged,
};

const BOARD_MAIN_COLOR: Color = Color::srgba(0.96, 0.96, 0.96, 0.88);
const BOARD_GAS_COLOR: Color = Color::srgba(0.80, 0.81, 0.82, 0.78);
const BACKDROP_MAIN_COLOR: Color = Color::srgba(0.96, 0.96, 0.96, 1.0);
const BACKDROP_GAS_COLOR: Color = Color::srgba(0.84, 0.84, 0.85, 1.0);
const GRID_LINE_COLOR: Color = Color::srgba(0.29, 0.32, 0.35, 1.0);
const CURSOR_GRID_MAX_ALPHA: f32 = 0.24;
const CURSOR_GRID_RADIUS_CELLS: i32 = 4;
const CURSOR_GRID_FADE_RADIUS: f32 = 3.9;
const BACKDROP_TILE_SIZE: f32 = 256.0;

#[derive(Resource, Clone)]
pub struct GasSimulationImages {
    pub texture_f2_a: Handle<Image>,
    pub texture_f2_b: Handle<Image>,
    pub texture_f1_a: Handle<Image>,
    pub texture_f1_b: Handle<Image>,
}

#[derive(Resource, Clone, Copy)]
pub struct GasVisualSettings {
    pub gamma: f32,
    pub max_particles_for_max_color: u32,
}

impl Default for GasVisualSettings {
    fn default() -> Self {
        Self {
            gamma: 1.0,
            max_particles_for_max_color: 1000,
        }
    }
}

fn gas_visual_intensity(
    particles: f32,
    gamma: f32,
    max_particles_for_max_color: f32,
    min_particles: f32,
    min_intensity: f32,
) -> f32 {
    if particles <= 0.0 {
        return 0.0;
    }

    let max_particles = max_particles_for_max_color.max(1.0);
    let clamped = particles.clamp(min_particles, max_particles);
    let norm = if max_particles <= min_particles {
        1.0
    } else {
        ((clamped - min_particles) / (max_particles - min_particles)).clamp(0.0, 1.0)
    };
    let base = min_intensity + (1.0 - min_intensity) * norm;
    base.powf(gamma.clamp(0.0, 10.0))
}

pub fn setup_simulation_images(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image_f2_a = images.add(build_seeded_image_f2());
    let image_f2_b = images.add(build_seeded_image_f2());
    let image_f1_a = images.add(build_seeded_image_f1());
    let image_f1_b = images.add(build_seeded_image_f1());

    commands.insert_resource(GasSimulationImages {
        texture_f2_a: image_f2_a,
        texture_f2_b: image_f2_b,
        texture_f1_a: image_f1_a,
        texture_f1_b: image_f1_b,
    });
}

fn build_seeded_image_f2() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: WORLD_WIDTH,
            height: WORLD_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 16],
        TextureFormat::Rgba32Float,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let wall = if is_boundary(x, y) { 1.0 } else { 0.0 };
            let particles = 0.0f32;
            let storage_linear =
                (particles / HYDROGEN_GPU_STORAGE_MAX_PARTICLES as f32).clamp(0.0, 1.0);
            let visual = 0.05;
            let color = Color::linear_rgba(visual, wall, storage_linear, 1.0);
            let _ = image.set_color_at(x, y, color);
        }
    }

    image
}

fn build_seeded_image_f1() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: WORLD_WIDTH,
            height: WORLD_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 16],
        TextureFormat::Rgba32Float,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    image
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    Main,
    Gas,
}

#[derive(Component)]
pub(crate) struct BoardLayer;

#[derive(Component)]
pub(crate) struct BackdropLayer;

#[derive(Component)]
pub(crate) struct GasOverlaySprite;

#[derive(Component)]
pub(crate) struct GasMainOverlaySprite;

#[derive(Component)]
pub(crate) struct WallVisual {
    main_tint: Color,
    gas_tint: Color,
}

#[derive(Resource, Clone)]
pub(crate) struct WorldVisualAssets {
    backdrop_noise: Handle<Image>,
    brick: Handle<Image>,
    metal: Handle<Image>,
    boundary: Handle<Image>,
    source: Handle<Image>,
    sink: Handle<Image>,
}

#[derive(Resource, Default)]
pub(crate) struct WallEntities {
    by_cell: HashMap<(u32, u32), Entity>,
}

#[derive(Component)]
struct GasStructureVisual;

#[derive(Component)]
pub(crate) struct GasStructureEditHighlight;

#[derive(Resource, Default)]
pub(crate) struct GasStructureEntities {
    by_cell: HashMap<(u32, u32), Entity>,
}

pub fn setup_world_view(
    mut commands: Commands,
    simulation_images: Res<GasSimulationImages>,
    world: Res<WorldGrid>,
    structures: Res<GasStructureGrid>,
    world_load_state: Res<WorldLoadState>,
    asset_server: Res<AssetServer>,
    cell_visuals: Res<CellTypeVisualConfig>,
) {
    let show_world = world_load_state.has_world;
    let visuals = WorldVisualAssets {
        backdrop_noise: asset_server.load("sprites/world/backdrop_noise.png"),
        brick: asset_server.load("sprites/world/tile_brick.png"),
        metal: asset_server.load("sprites/world/tile_metal.png"),
        boundary: asset_server.load("sprites/world/tile_boundary.png"),
        source: asset_server.load("sprites/world/tile_gas_source.png"),
        sink: asset_server.load("sprites/world/tile_gas_sink.png"),
    };
    commands.insert_resource(visuals.clone());

    let world_size = world_dimensions();

    commands.spawn((
        Sprite {
            image: visuals.backdrop_noise.clone(),
            custom_size: Some(world_size + Vec2::splat(CELL_SIZE * 6.0)),
            color: BACKDROP_MAIN_COLOR,
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: BACKDROP_TILE_SIZE,
            },
            ..default()
        },
        Transform::from_xyz(14.0, -18.0, -2.0),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        BackdropLayer,
    ));

    commands.spawn((
        Sprite {
            image: visuals.backdrop_noise.clone(),
            custom_size: Some(world_size),
            color: BOARD_MAIN_COLOR,
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: BACKDROP_TILE_SIZE,
            },
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        BoardLayer,
    ));

    commands.spawn((
        Sprite {
            image: simulation_images.texture_f2_a.clone(),
            custom_size: Some(world_size),
            color: Color::srgba(1.0, 0.25, 0.1, 0.88),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
        Visibility::Hidden,
        GasOverlaySprite,
    ));

    commands.spawn((
        Sprite {
            image: simulation_images.texture_f1_a.clone(),
            custom_size: Some(world_size),
            color: Color::WHITE,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.8),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        GasMainOverlaySprite,
    ));

    let mut wall_entities = WallEntities::default();
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            if let CellKind::Solid(material) = world.cell(x, y) {
                let entity = spawn_wall_sprite(
                    &mut commands,
                    &visuals,
                    &cell_visuals,
                    x,
                    y,
                    material,
                    show_world,
                );
                wall_entities.by_cell.insert((x, y), entity);
            }
        }
    }
    commands.insert_resource(wall_entities);

    let mut structure_entities = GasStructureEntities::default();
    for (x, y, structure) in structures.iter_cells() {
        let entity = spawn_gas_structure_sprite(&mut commands, &visuals, x, y, structure, show_world);
        structure_entities.by_cell.insert((x, y), entity);
    }
    commands.insert_resource(structure_entities);
    commands.spawn((
        Sprite::from_color(
            Color::srgba(1.0, 0.93, 0.30, 0.36),
            Vec2::splat(CELL_SIZE - 2.0),
        ),
        Transform::from_xyz(0.0, 0.0, 0.91),
        Visibility::Hidden,
        GasStructureEditHighlight,
    ));
}

fn spawn_wall_sprite(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    cell_visuals: &CellTypeVisualConfig,
    x: u32,
    y: u32,
    material: CellMaterial,
    show_world: bool,
) -> Entity {
    let image = match material {
        CellMaterial::Boundary => visuals.boundary.clone(),
        CellMaterial::Brick => visuals.brick.clone(),
        CellMaterial::Metal => visuals.metal.clone(),
    };
    let main_tint = cell_visuals.main_tint(material);
    let gas_tint = cell_visuals.gas_tint(material);

    commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(Vec2::splat(CELL_SIZE)),
                color: main_tint,
                ..default()
            },
            Transform::from_translation(cell_center(x, y).extend(0.5)),
            if show_world {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            WallVisual {
                main_tint,
                gas_tint,
            },
        ))
        .id()
}

fn spawn_gas_structure_sprite(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    x: u32,
    y: u32,
    structure: GasStructureCell,
    show_world: bool,
) -> Entity {
    let image = match structure {
        GasStructureCell::Source { .. } => visuals.source.clone(),
        GasStructureCell::Sink { .. } => visuals.sink.clone(),
    };
    commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(Vec2::splat(CELL_SIZE)),
                color: Color::WHITE,
                ..default()
            },
            Transform::from_translation(cell_center(x, y).extend(0.9)),
            if show_world {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            GasStructureVisual,
        ))
        .id()
}

pub(crate) fn sync_wall_visuals(
    mut commands: Commands,
    world: Res<WorldGrid>,
    visuals: Res<WorldVisualAssets>,
    cell_visuals: Res<CellTypeVisualConfig>,
    mut wall_entities: ResMut<WallEntities>,
    mut changes: EventReader<WorldCellChanged>,
) {
    for change in changes.read() {
        let x = change.cell.x;
        let y = change.cell.y;
        let key = (x, y);
        let current_cell = world.cell(x, y);

        match (current_cell, wall_entities.by_cell.get(&key).copied()) {
            (CellKind::Solid(material), None) => {
                let entity =
                    spawn_wall_sprite(&mut commands, &visuals, &cell_visuals, x, y, material, true);
                wall_entities.by_cell.insert(key, entity);
            }
            (CellKind::Empty, Some(entity)) => {
                commands.entity(entity).despawn();
                wall_entities.by_cell.remove(&key);
            }
            (CellKind::Solid(material), Some(entity)) => {
                commands.entity(entity).despawn();
                let next_entity =
                    spawn_wall_sprite(&mut commands, &visuals, &cell_visuals, x, y, material, true);
                wall_entities.by_cell.insert(key, next_entity);
            }
            _ => {}
        }
    }
}

pub(crate) fn sync_gas_structure_visuals(
    mut commands: Commands,
    structures: Res<GasStructureGrid>,
    world_load_state: Res<WorldLoadState>,
    visuals: Res<WorldVisualAssets>,
    mut structure_entities: ResMut<GasStructureEntities>,
) {
    if !structures.is_changed() && !world_load_state.is_changed() {
        return;
    }

    for entity in structure_entities.by_cell.values().copied() {
        commands.entity(entity).despawn();
    }
    structure_entities.by_cell.clear();

    if !world_load_state.has_world {
        return;
    }

    for (x, y, structure) in structures.iter_cells() {
        let entity =
            spawn_gas_structure_sprite(&mut commands, &visuals, x, y, structure, world_load_state.has_world);
        structure_entities.by_cell.insert((x, y), entity);
    }
}

pub(crate) fn sync_structure_edit_highlight(
    world_load_state: Res<WorldLoadState>,
    structure_edit: Res<StructureEditState>,
    structures: Res<GasStructureGrid>,
    mut highlight: Single<(&mut Transform, &mut Visibility), With<GasStructureEditHighlight>>,
) {
    let (transform, visibility) = &mut *highlight;
    if !world_load_state.has_world {
        **visibility = Visibility::Hidden;
        return;
    }
    let Some(cell) = structure_edit.selected_cell else {
        **visibility = Visibility::Hidden;
        return;
    };
    if structures.cell(cell.x, cell.y).is_none() {
        **visibility = Visibility::Hidden;
        return;
    }
    **transform = Transform::from_translation(cell_center(cell.x, cell.y).extend(0.91));
    **visibility = Visibility::Visible;
}

pub fn update_overlay_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut overlay_mode: ResMut<OverlayMode>,
) {
    if input.just_pressed(KeyCode::F1) {
        *overlay_mode = OverlayMode::Main;
    }
    if input.just_pressed(KeyCode::F2) {
        *overlay_mode = OverlayMode::Gas;
    }
}

pub fn apply_overlay_mode(
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    mut sprite_sets: ParamSet<(
        Query<
            (&mut Sprite, &mut Visibility),
            (
                With<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            (&mut Sprite, &mut Visibility),
            (
                With<BackdropLayer>,
                Without<BoardLayer>,
                Without<WallVisual>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            (&WallVisual, &mut Sprite, &mut Visibility),
            (
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            &mut Visibility,
            (
                With<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
            ),
        >,
        Query<
            &mut Visibility,
            (
                With<GasMainOverlaySprite>,
                Without<GasOverlaySprite>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
            ),
        >,
    )>,
) {
    if !overlay_mode.is_changed() && !world_load_state.is_changed() {
        return;
    }

    let show_world = world_load_state.has_world;
    let (board_color, backdrop_color, gas_visibility, gas_main_visibility) = match *overlay_mode {
        OverlayMode::Main => (
            BOARD_MAIN_COLOR,
            BACKDROP_MAIN_COLOR,
            Visibility::Hidden,
            Visibility::Visible,
        ),
        OverlayMode::Gas => (
            BOARD_GAS_COLOR,
            BACKDROP_GAS_COLOR,
            Visibility::Visible,
            Visibility::Hidden,
        ),
    };

    for (mut board, mut visibility) in &mut sprite_sets.p0() {
        board.color = board_color;
        *visibility = if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut backdrop, mut visibility) in &mut sprite_sets.p1() {
        backdrop.color = backdrop_color;
        *visibility = if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut sprite_sets.p3() {
        *visibility = if show_world {
            gas_visibility
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut sprite_sets.p4() {
        *visibility = if show_world {
            gas_main_visibility
        } else {
            Visibility::Hidden
        };
    }
    for (wall_visual, mut sprite, mut visibility) in &mut sprite_sets.p2() {
        sprite.color = match *overlay_mode {
            OverlayMode::Main => wall_visual.main_tint,
            OverlayMode::Gas => wall_visual.gas_tint,
        };
        *visibility = if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn draw_cursor_grid_overlay(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    world_load_state: Res<WorldLoadState>,
    panels: Option<Res<PanelManager>>,
    mut gizmos: Gizmos,
) {
    if !world_load_state.has_world {
        return;
    }

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    if panels
        .as_ref()
        .map(|panel_manager| panel_manager.is_cursor_over_any_panel(cursor_pos))
        .unwrap_or(false)
    {
        return;
    }

    let (camera, camera_transform) = *camera_query;
    let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };
    let Some(cursor_cell) = world_to_cell(cursor_world_pos) else {
        return;
    };

    for dy in -CURSOR_GRID_RADIUS_CELLS..=CURSOR_GRID_RADIUS_CELLS {
        for dx in -CURSOR_GRID_RADIUS_CELLS..=CURSOR_GRID_RADIUS_CELLS {
            let x = cursor_cell.x as i32 + dx;
            let y = cursor_cell.y as i32 + dy;
            if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
                continue;
            }

            let center = cell_center(x as u32, y as u32);
            let cell_distance = (center - cursor_world_pos).length() / CELL_SIZE;
            let fade = grid_fade(cell_distance, CURSOR_GRID_FADE_RADIUS);
            if fade <= 0.01 {
                continue;
            }

            let color = GRID_LINE_COLOR.with_alpha(CURSOR_GRID_MAX_ALPHA * fade);
            gizmos.rect_2d(
                Isometry2d::from_translation(center),
                Vec2::splat(CELL_SIZE),
                color,
            );
        }
    }
}

pub fn sync_gas_display_texture(
    step: Res<SimulationStep>,
    gas: Res<GasField>,
    gas_registry: Res<GasRegistry>,
    visual_settings: Res<GasVisualSettings>,
    main_view_settings: Res<GasMainViewVisualConfig>,
    simulation_images: Res<GasSimulationImages>,
    mut images: ResMut<Assets<Image>>,
    mut gas_query: Query<&mut Sprite, (With<GasOverlaySprite>, Without<GasMainOverlaySprite>)>,
    mut gas_main_query: Query<&mut Sprite, (With<GasMainOverlaySprite>, Without<GasOverlaySprite>)>,
) {
    if !step.is_changed()
        && !gas.is_changed()
        && !gas_registry.is_changed()
        && !visual_settings.is_changed()
        && !main_view_settings.is_changed()
    {
        return;
    }

    for image_handle in [
        &simulation_images.texture_f2_a,
        &simulation_images.texture_f2_b,
    ] {
        let Some(image) = images.get_mut(image_handle) else {
            continue;
        };

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let texture_y = WORLD_HEIGHT - 1 - y;
                let color = if is_boundary(x, y) {
                    Color::linear_rgba(0.0, 1.0, 0.0, 1.0)
                } else {
                    // Keep overlay semantics consistent with HUD "particles" values.
                    // Tiny float residuals from solver should not light up cells as non-zero gas.
                    let particles = gas.total_amount_rounded(x, y) as f32;
                    let storage_linear =
                        (particles / HYDROGEN_GPU_STORAGE_MAX_PARTICLES as f32).clamp(0.0, 1.0);
                    let visual = gas_visual_intensity(
                        particles,
                        visual_settings.gamma,
                        visual_settings.max_particles_for_max_color as f32,
                        1.0,
                        0.05,
                    );
                    Color::linear_rgba(visual, 0.0, storage_linear, 1.0)
                };

                let _ = image.set_color_at(x, texture_y, color);
            }
        }
    }

    let texture = if step.0 % 2 == 0 {
        simulation_images.texture_f2_a.clone()
    } else {
        simulation_images.texture_f2_b.clone()
    };

    for mut sprite in &mut gas_query {
        sprite.image = texture.clone();
    }

    for image_handle in [
        &simulation_images.texture_f1_a,
        &simulation_images.texture_f1_b,
    ] {
        let Some(image) = images.get_mut(image_handle) else {
            continue;
        };
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let texture_y = WORLD_HEIGHT - 1 - y;
                let color = if is_boundary(x, y) {
                    Color::linear_rgba(0.0, 0.0, 0.0, 0.0)
                } else {
                    let total = gas.total_amount(x, y).max(0.0);
                    if total <= 1e-6 {
                        Color::linear_rgba(0.0, 0.0, 0.0, 0.0)
                    } else {
                        let mut weighted_rgb = Vec3::ZERO;
                        for gas_index in 0..gas.gas_count() {
                            let amount = gas.amount(x, y, gas_index).max(0.0);
                            if amount <= 1e-6 {
                                continue;
                            }
                            if let Some(gas_def) = gas_registry.get(gas_index) {
                                let rgb = Vec3::from_array(gas_def.color);
                                weighted_rgb += rgb * amount;
                            }
                        }
                        let mix_rgb = if weighted_rgb.length_squared() <= f32::EPSILON {
                            Vec3::ZERO
                        } else {
                            weighted_rgb / total.max(1e-6)
                        };
                        let visual = gas_visual_intensity(
                            total,
                            visual_settings.gamma,
                            main_view_settings.max_particles_for_max_intensity,
                            main_view_settings.min_particles,
                            main_view_settings.min_intensity,
                        );
                        let rgb = (mix_rgb * visual).clamp(Vec3::ZERO, Vec3::ONE);
                        let alpha = (visual * main_view_settings.alpha).clamp(0.0, 1.0);
                        Color::linear_rgba(rgb.x, rgb.y, rgb.z, alpha)
                    }
                };
                let _ = image.set_color_at(x, texture_y, color);
            }
        }
    }

    let f1_texture = if step.0 % 2 == 0 {
        simulation_images.texture_f1_a.clone()
    } else {
        simulation_images.texture_f1_b.clone()
    };
    for mut sprite in &mut gas_main_query {
        sprite.image = f1_texture.clone();
    }
}

fn grid_fade(distance_cells: f32, fade_radius_cells: f32) -> f32 {
    if fade_radius_cells <= f32::EPSILON || distance_cells >= fade_radius_cells {
        return 0.0;
    }
    if distance_cells <= 0.0 {
        return 1.0;
    }

    let t = (distance_cells / fade_radius_cells).clamp(0.0, 1.0);
    let smooth = 1.0 - t * t;
    smooth * smooth
}

#[cfg(test)]
mod tests {
    use super::grid_fade;

    #[test]
    fn grid_fade_is_full_at_center_and_zero_beyond_radius() {
        assert!((grid_fade(0.0, 8.0) - 1.0).abs() < 1e-6);
        assert_eq!(grid_fade(8.0, 8.0), 0.0);
        assert_eq!(grid_fade(9.5, 8.0), 0.0);
    }

    #[test]
    fn grid_fade_decreases_smoothly_with_distance() {
        let near = grid_fade(1.0, 8.0);
        let mid = grid_fade(4.0, 8.0);
        let far = grid_fade(7.0, 8.0);
        assert!(near > mid && mid > far && far > 0.0);
    }
}
