use std::collections::HashMap;

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    input::camera::MainCamera,
    simulation::{
        gas::{GasField, HYDROGEN_GPU_STORAGE_MAX_PARTICLES},
        gpu::GasSimulationImages,
        SimulationStep,
    },
    world::grid::{
        cell_center, is_boundary, world_dimensions, world_to_cell, CellKind, CellMaterial,
        WorldGrid, CELL_SIZE, WORLD_HEIGHT, WORLD_WIDTH,
    },
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
const GAS_VISUAL_MIN_PARTICLES: f32 = 1.0;
const GAS_VISUAL_MIN_INTENSITY: f32 = 0.05;
const BACKDROP_TILE_SIZE: f32 = 256.0;

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

fn gas_visual_intensity(particles: f32, gamma: f32, max_particles_for_max_color: u32) -> f32 {
    if particles <= 0.0 {
        return 0.0;
    }

    let max_particles = max_particles_for_max_color.max(1) as f32;
    let clamped = particles.clamp(GAS_VISUAL_MIN_PARTICLES, max_particles);
    let norm = if max_particles <= GAS_VISUAL_MIN_PARTICLES {
        1.0
    } else {
        ((clamped - GAS_VISUAL_MIN_PARTICLES) / (max_particles - GAS_VISUAL_MIN_PARTICLES))
            .clamp(0.0, 1.0)
    };
    let base = GAS_VISUAL_MIN_INTENSITY + (1.0 - GAS_VISUAL_MIN_INTENSITY) * norm;
    base.powf(gamma.clamp(0.0, 10.0))
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
}

#[derive(Resource, Default)]
pub(crate) struct WallEntities {
    by_cell: HashMap<(u32, u32), Entity>,
}

pub fn setup_world_view(
    mut commands: Commands,
    simulation_images: Res<GasSimulationImages>,
    world: Res<WorldGrid>,
    asset_server: Res<AssetServer>,
) {
    let visuals = WorldVisualAssets {
        backdrop_noise: asset_server.load("sprites/world/backdrop_noise.png"),
        brick: asset_server.load("sprites/world/tile_brick.png"),
        metal: asset_server.load("sprites/world/tile_metal.png"),
        boundary: asset_server.load("sprites/world/tile_boundary.png"),
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
        BoardLayer,
    ));

    commands.spawn((
        Sprite {
            image: simulation_images.texture_a.clone(),
            custom_size: Some(world_size),
            color: Color::srgba(1.0, 0.25, 0.1, 0.88),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
        Visibility::Hidden,
        GasOverlaySprite,
    ));

    let mut wall_entities = WallEntities::default();
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            if let CellKind::Solid(material) = world.cell(x, y) {
                let entity = spawn_wall_sprite(&mut commands, &visuals, x, y, material);
                wall_entities.by_cell.insert((x, y), entity);
            }
        }
    }
    commands.insert_resource(wall_entities);
}

fn spawn_wall_sprite(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    x: u32,
    y: u32,
    material: CellMaterial,
) -> Entity {
    let (image, main_tint, gas_tint) = match material {
        CellMaterial::Boundary => (
            visuals.boundary.clone(),
            Color::srgb(0.90, 0.90, 0.91),
            Color::srgb(0.66, 0.66, 0.67),
        ),
        CellMaterial::Brick => (
            visuals.brick.clone(),
            Color::srgb(0.99, 0.99, 0.99),
            Color::srgb(0.72, 0.72, 0.74),
        ),
        CellMaterial::Metal => (
            visuals.metal.clone(),
            Color::srgb(0.99, 0.99, 0.99),
            Color::srgb(0.71, 0.71, 0.73),
        ),
    };

    commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(Vec2::splat(CELL_SIZE)),
                color: main_tint,
                ..default()
            },
            Transform::from_translation(cell_center(x, y).extend(0.5)),
            WallVisual {
                main_tint,
                gas_tint,
            },
        ))
        .id()
}

pub fn sync_wall_visuals(
    mut commands: Commands,
    world: Res<WorldGrid>,
    visuals: Res<WorldVisualAssets>,
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
                let entity = spawn_wall_sprite(&mut commands, &visuals, x, y, material);
                wall_entities.by_cell.insert(key, entity);
            }
            (CellKind::Empty, Some(entity)) => {
                commands.entity(entity).despawn();
                wall_entities.by_cell.remove(&key);
            }
            (CellKind::Solid(material), Some(entity)) => {
                commands.entity(entity).despawn();
                let next_entity = spawn_wall_sprite(&mut commands, &visuals, x, y, material);
                wall_entities.by_cell.insert(key, next_entity);
            }
            _ => {}
        }
    }
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
    mut sprite_sets: ParamSet<(
        Query<&mut Sprite, With<BoardLayer>>,
        Query<&mut Sprite, (With<BackdropLayer>, Without<BoardLayer>)>,
        Query<(&WallVisual, &mut Sprite)>,
    )>,
    mut gas_query: Query<&mut Visibility, With<GasOverlaySprite>>,
) {
    if !overlay_mode.is_changed() {
        return;
    }

    let (board_color, backdrop_color, gas_visibility) = match *overlay_mode {
        OverlayMode::Main => (BOARD_MAIN_COLOR, BACKDROP_MAIN_COLOR, Visibility::Hidden),
        OverlayMode::Gas => (BOARD_GAS_COLOR, BACKDROP_GAS_COLOR, Visibility::Visible),
    };

    for mut board in &mut sprite_sets.p0() {
        board.color = board_color;
    }
    for mut backdrop in &mut sprite_sets.p1() {
        backdrop.color = backdrop_color;
    }
    for mut visibility in &mut gas_query {
        *visibility = gas_visibility;
    }
    for (wall_visual, mut sprite) in &mut sprite_sets.p2() {
        sprite.color = match *overlay_mode {
            OverlayMode::Main => wall_visual.main_tint,
            OverlayMode::Gas => wall_visual.gas_tint,
        };
    }
}

pub fn draw_cursor_grid_overlay(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut gizmos: Gizmos,
) {
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

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
    visual_settings: Res<GasVisualSettings>,
    simulation_images: Res<GasSimulationImages>,
    mut images: ResMut<Assets<Image>>,
    mut gas_query: Query<&mut Sprite, With<GasOverlaySprite>>,
) {
    if !step.is_changed() && !visual_settings.is_changed() {
        return;
    }

    for image_handle in [&simulation_images.texture_a, &simulation_images.texture_b] {
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
                        visual_settings.max_particles_for_max_color,
                    );
                    Color::linear_rgba(visual, 0.0, storage_linear, 1.0)
                };

                let _ = image.set_color_at(x, texture_y, color);
            }
        }
    }

    let texture = if step.0 % 2 == 0 {
        simulation_images.texture_a.clone()
    } else {
        simulation_images.texture_b.clone()
    };

    for mut sprite in &mut gas_query {
        sprite.image = texture.clone();
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
