use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::render_asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};

use crate::{
    simulation::{
        gas::{GasField, HYDROGEN_GPU_STORAGE_MAX_PARTICLES},
        SimulationStep,
    },
    world::grid::{
        cell_center, is_boundary, world_dimensions, world_origin, CellKind, WorldGrid, CELL_SIZE,
        WORLD_HEIGHT, WORLD_WIDTH,
    },
    world::WorldCellChanged,
};

const BOARD_MAIN_COLOR: Color = Color::srgb(0.06, 0.09, 0.12);
const BOARD_GAS_COLOR: Color = Color::srgb(0.18, 0.18, 0.18);
const BACKDROP_MAIN_COLOR: Color = Color::srgba(0.10, 0.14, 0.18, 0.85);
const BACKDROP_GAS_COLOR: Color = Color::srgba(0.12, 0.12, 0.12, 0.85);
const GRID_LINE_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.09);
const GAS_VISUAL_MIN_PARTICLES: f32 = 1.0;
const GAS_VISUAL_MIN_INTENSITY: f32 = 0.05;

#[derive(Resource, Clone)]
pub struct GasSimulationImages {
    pub texture_a: Handle<Image>,
    pub texture_b: Handle<Image>,
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

pub fn setup_simulation_images(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image_a = images.add(build_seeded_image());
    let image_b = images.add(build_seeded_image());

    commands.insert_resource(GasSimulationImages {
        texture_a: image_a,
        texture_b: image_b,
    });
}

fn build_seeded_image() -> Image {
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
            let visual = GAS_VISUAL_MIN_INTENSITY;
            let color = Color::linear_rgba(visual, wall, storage_linear, 1.0);
            let _ = image.set_color_at(x, y, color);
        }
    }

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
pub(crate) struct WallVisual {
    main: Color,
    gas: Color,
}

#[derive(Resource, Default)]
pub(crate) struct WallEntities {
    by_cell: HashMap<(u32, u32), Entity>,
}

pub fn setup_world_view(
    mut commands: Commands,
    simulation_images: Res<GasSimulationImages>,
    world: Res<WorldGrid>,
) {
    let world_size = world_dimensions();

    commands.spawn((
        Sprite::from_color(
            BACKDROP_MAIN_COLOR,
            world_size + Vec2::splat(CELL_SIZE * 4.0),
        ),
        Transform::from_xyz(14.0, -18.0, -2.0),
        BackdropLayer,
    ));

    commands.spawn((
        Sprite::from_color(BOARD_MAIN_COLOR, world_size),
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
            if world.cell(x, y) != CellKind::Solid {
                continue;
            }
            let entity = spawn_wall_sprite(&mut commands, x, y);
            wall_entities.by_cell.insert((x, y), entity);
        }
    }
    commands.insert_resource(wall_entities);

    // Semi-transparent grid lines
    let origin = world_origin();
    let world_w = WORLD_WIDTH as f32 * CELL_SIZE;
    let world_h = WORLD_HEIGHT as f32 * CELL_SIZE;
    let grid_z = 1.5_f32;

    for i in 0..=WORLD_WIDTH {
        let x = origin.x + i as f32 * CELL_SIZE;
        commands.spawn((
            Sprite::from_color(GRID_LINE_COLOR, Vec2::new(1.0, world_h)),
            Transform::from_xyz(x, 0.0, grid_z),
        ));
    }
    for i in 0..=WORLD_HEIGHT {
        let y = origin.y + i as f32 * CELL_SIZE;
        commands.spawn((
            Sprite::from_color(GRID_LINE_COLOR, Vec2::new(world_w, 1.0)),
            Transform::from_xyz(0.0, y, grid_z),
        ));
    }
}

fn spawn_wall_sprite(commands: &mut Commands, x: u32, y: u32) -> Entity {
    let pattern_offset = if (x + y) % 2 == 0 { 0.05 } else { -0.05 };
    let main = Color::srgb(
        0.34 + pattern_offset,
        0.38 + pattern_offset,
        0.42 + pattern_offset,
    );
    let gas = Color::srgb(
        0.48 + pattern_offset,
        0.48 + pattern_offset,
        0.48 + pattern_offset,
    );

    commands
        .spawn((
            Sprite::from_color(main, Vec2::splat(CELL_SIZE - 1.0)),
            Transform::from_translation(cell_center(x, y).extend(0.5)),
            WallVisual { main, gas },
        ))
        .id()
}

pub fn sync_wall_visuals(
    mut commands: Commands,
    world: Res<WorldGrid>,
    mut wall_entities: ResMut<WallEntities>,
    mut changes: EventReader<WorldCellChanged>,
) {
    for change in changes.read() {
        let x = change.cell.x;
        let y = change.cell.y;
        let key = (x, y);
        let is_solid = world.cell(x, y) == CellKind::Solid;

        match (is_solid, wall_entities.by_cell.get(&key).copied()) {
            (true, None) => {
                let entity = spawn_wall_sprite(&mut commands, x, y);
                wall_entities.by_cell.insert(key, entity);
            }
            (false, Some(entity)) => {
                commands.entity(entity).despawn();
                wall_entities.by_cell.remove(&key);
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
            OverlayMode::Main => wall_visual.main,
            OverlayMode::Gas => wall_visual.gas,
        };
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
