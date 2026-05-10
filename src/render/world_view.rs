use std::collections::HashMap;

use bevy::{
    image::ImageSampler,
    prelude::*,
    render::mesh::Mesh,
    render::render_asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    window::PrimaryWindow,
};

use crate::{
    config::{CellTypeVisualConfig, GasMainViewVisualConfig, GasRegistry},
    editor::StructureEditState,
    input::camera::MainCamera,
    render::pipe_highlight_material::PipeHighlightRenderAssets,
    save::WorldLoadState,
    simulation::{
        gas::{GasField, HYDROGEN_GPU_STORAGE_MAX_PARTICLES},
        pipes::{pipe_cell_display_blocks_with_transfers, PipeFlowVisualState, PipeGasField},
        GasSimulationConfig, SimulationStep,
    },
    ui::panels::PanelManager,
    world::grid::{
        cell_center, is_boundary, linear_index, world_dimensions, world_to_cell, CellKind,
        CellMaterial, WorldGrid, CELL_SIZE, WORLD_HEIGHT, WORLD_WIDTH,
    },
    world::structures::{
        PlacedStructure, PlacedStructureId, PlacedStructureMap, StructureKind, StructureRotation,
    },
    world::WorldCellChanged,
};

const BOARD_MAIN_COLOR: Color = Color::srgba(0.96, 0.96, 0.96, 0.88);
const BOARD_GAS_COLOR: Color = Color::srgba(0.80, 0.81, 0.82, 0.78);
const BOARD_PIPE_COLOR: Color = Color::srgba(0.10, 0.12, 0.14, 0.96);
const BACKDROP_MAIN_COLOR: Color = Color::srgba(0.96, 0.96, 0.96, 1.0);
const BACKDROP_GAS_COLOR: Color = Color::srgba(0.84, 0.84, 0.85, 1.0);
const BACKDROP_PIPE_COLOR: Color = Color::srgba(0.08, 0.09, 0.10, 1.0);
const GRID_LINE_COLOR: Color = Color::srgba(0.29, 0.32, 0.35, 1.0);
const CURSOR_GRID_MAX_ALPHA: f32 = 0.24;
const CURSOR_GRID_RADIUS_CELLS: i32 = 4;
const CURSOR_GRID_FADE_RADIUS: f32 = 3.9;
const BACKDROP_TILE_SIZE: f32 = 256.0;
const OUTER_BORDER_LAYERS: u32 = 4;
const WORLD_FADE_WIDTH_CELLS: f32 = 4.0;
const WORLD_FADE_ALPHA_MAX: f32 = 1.0;
pub(crate) const WORLD_PREVIEW_TARGET_SIZE_PX: u32 = 512;

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

fn build_world_fade_mask_image(world_size: Vec2) -> Image {
    let fade_width_world = (CELL_SIZE * WORLD_FADE_WIDTH_CELLS).max(1.0);
    let total_width_world = (world_size.x.max(1.0) + fade_width_world * 2.0).max(2.0);
    let total_height_world = (world_size.y.max(1.0) + fade_width_world * 2.0).max(2.0);
    let width = total_width_world.ceil() as u32;
    let height = total_height_world.ceil() as u32;
    let mut data = vec![0_u8; (width as usize) * (height as usize) * 4];
    let inner_min_x = fade_width_world;
    let inner_max_x = inner_min_x + world_size.x.max(0.0);
    let inner_min_y = fade_width_world;
    let inner_max_y = inner_min_y + world_size.y.max(0.0);

    for y in 0..height {
        for x in 0..width {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = if px < inner_min_x {
                inner_min_x - px
            } else if px > inner_max_x {
                px - inner_max_x
            } else {
                0.0
            };
            let dy = if py < inner_min_y {
                inner_min_y - py
            } else if py > inner_max_y {
                py - inner_max_y
            } else {
                0.0
            };
            let outward_distance = dx.max(dy);
            let alpha = world_fade_alpha(outward_distance, fade_width_world);
            let pixel_index = ((y * width + x) as usize) * 4;
            data[pixel_index] = 0;
            data[pixel_index + 1] = 0;
            data[pixel_index + 2] = 0;
            data[pixel_index + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

fn world_fade_alpha(distance_from_world: f32, fade_width_world: f32) -> f32 {
    if distance_from_world <= 0.0 {
        return 0.0;
    }
    if fade_width_world <= f32::EPSILON {
        return WORLD_FADE_ALPHA_MAX;
    }

    let t = (distance_from_world / fade_width_world).clamp(0.0, 1.0);
    let smoothstep = t * t * (3.0 - 2.0 * t);
    smoothstep.clamp(0.0, WORLD_FADE_ALPHA_MAX)
}

pub(crate) fn preview_world_extent() -> Vec2 {
    world_dimensions() + Vec2::splat(CELL_SIZE * WORLD_FADE_WIDTH_CELLS * 2.0)
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    Main,
    Gas,
    Pipes,
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

#[derive(Component)]
pub(crate) struct OuterBorderVisual {
    main_tint: Color,
    gas_tint: Color,
}

#[derive(Component)]
pub(crate) struct WorldFadeMaskLayer;

#[derive(Resource, Clone)]
pub(crate) struct WorldVisualAssets {
    backdrop_noise: Handle<Image>,
    brick: Handle<Image>,
    metal: Handle<Image>,
    boundary: Handle<Image>,
    source: Handle<Image>,
    sink: Handle<Image>,
    bridge: Handle<Image>,
    pipe_masks: Vec<Handle<Image>>,
    vent_world: Handle<Image>,
    vent_overlay: Handle<Image>,
    pipe_highlight: PipeHighlightRenderAssets,
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

#[derive(Component)]
pub(crate) struct PipeWorldVisual {
    appearance_z: f32,
    main_tint: Color,
    gas_tint: Color,
    pipe_tint: Color,
}

#[derive(Component)]
pub(crate) struct VentWorldVisual;

#[derive(Component)]
pub(crate) struct PipeGasOverlayVisual;

#[derive(Component)]
pub(crate) struct PipeGasOverlayBorderVisual;

#[derive(Component)]
pub(crate) struct PipeVentOverlayVisual;

#[derive(Component)]
pub(crate) struct PipeHighlightOverlayVisual;

#[derive(Component)]
pub(crate) struct PipeFlowPacketVisual;

#[derive(Resource, Default)]
pub(crate) struct PipeEntities {
    pipes: HashMap<(u32, u32), Entity>,
    bridges: HashMap<PlacedStructureId, Entity>,
    pipe_highlights: HashMap<(u32, u32), Entity>,
    vents: HashMap<(u32, u32), Entity>,
    gas_overlays: HashMap<(u32, u32), [Entity; 2]>,
    gas_overlay_borders: HashMap<(u32, u32), [Entity; 2]>,
    vent_overlays: HashMap<(u32, u32), Entity>,
    flow_packets: Vec<Entity>,
}

include!("world_view_setup_block.rs");
include!("world_view_overlay_block.rs");
include!("world_view_cursor_highlight_block.rs");
include!("world_view_tests_block.rs");
