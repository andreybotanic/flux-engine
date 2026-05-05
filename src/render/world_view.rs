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
/// Stores `GasSimulationImages` state.
pub struct GasSimulationImages {
    pub texture_f2_a: Handle<Image>,
    pub texture_f2_b: Handle<Image>,
    pub texture_f1_a: Handle<Image>,
    pub texture_f1_b: Handle<Image>,
}

#[derive(Resource, Clone, Copy)]
/// Stores `GasVisualSettings` state.
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

/// Runs `setup_simulation_images` logic.
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

include!("world_view_setup_block.rs");
include!("world_view_overlay_block.rs");
include!("world_view_tests_block.rs");
