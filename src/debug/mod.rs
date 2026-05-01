use bevy::prelude::*;

use crate::{
    input::camera::MainCamera,
    simulation::{
        do_one_substep,
        gas::{preview_next_substep, GasField, GasKind},
        BlockSyncState, GasSimulationConfig, GasSolverMode, SimulationControl, SimulationStep,
    },
    world::grid::{cell_center, is_boundary, WorldGrid, CELL_SIZE, WORLD_HEIGHT, WORLD_WIDTH},
};

#[derive(Resource, Default)]
pub struct DebugMode {
    pub active: bool,
}

#[derive(Resource, Clone, Copy)]
pub struct DebugOverlaySettings {
    pub show_diffusion_cells: bool,
    pub show_momentum_vectors: bool,
    pub preview_min_amount: f32,
    pub preview_min_flux: f32,
}

impl Default for DebugOverlaySettings {
    fn default() -> Self {
        Self {
            show_diffusion_cells: true,
            show_momentum_vectors: false,
            preview_min_amount: 0.5,
            preview_min_flux: 0.01,
        }
    }
}

#[derive(Resource, Default)]
pub struct DebugStepPreview {
    /// Each entry: (from_x, from_y, to_x, to_y)
    pub moves: Vec<(u32, u32, u32, u32)>,
}

#[derive(Resource, Default, Clone, Copy)]
pub struct DebugGasMetrics {
    pub anisotropy_score: f32,
    pub radial_wave_score: f32,
    pub mass_error_h2: f32,
    pub mass_error_o2: f32,
}

#[derive(Resource, Default, Clone, Copy)]
struct DebugMassBaseline {
    initialized: bool,
    h2: f32,
    o2: f32,
}

pub struct DebugPlugin;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct MomentumVectorGizmoConfigGroup;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugMode>()
            .init_resource::<DebugOverlaySettings>()
            .init_resource::<DebugStepPreview>()
            .init_resource::<DebugGasMetrics>()
            .init_resource::<DebugMassBaseline>()
            .init_gizmo_group::<MomentumVectorGizmoConfigGroup>()
            .add_systems(
                Update,
                (
                    configure_momentum_gizmo_line_width,
                    handle_debug_keys,
                    update_debug_preview,
                    update_debug_metrics,
                    draw_debug_overlays,
                )
                    .chain(),
            );
    }
}

fn configure_momentum_gizmo_line_width(
    mut config_store: ResMut<GizmoConfigStore>,
    mut configured: Local<bool>,
) {
    if *configured {
        return;
    }
    let (config, _) = config_store.config_mut::<MomentumVectorGizmoConfigGroup>();
    config.line.width = 1.5;
    *configured = true;
}

fn handle_debug_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut debug_mode: ResMut<DebugMode>,
    mut control: ResMut<SimulationControl>,
    config: Res<GasSimulationConfig>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
) {
    if keys.just_pressed(KeyCode::Backquote) {
        debug_mode.active = !debug_mode.active;
        if debug_mode.active {
            control.paused = true;
        }
    }

    if debug_mode.active && keys.just_pressed(KeyCode::Enter) {
        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
    }
}

fn update_debug_preview(
    debug_mode: Res<DebugMode>,
    overlay_settings: Res<DebugOverlaySettings>,
    config: Res<GasSimulationConfig>,
    gas: Res<GasField>,
    world: Res<WorldGrid>,
    block_state: Res<BlockSyncState>,
    mut preview: ResMut<DebugStepPreview>,
) {
    if !debug_mode.active
        || !config.enable_diffusion
        || !overlay_settings.show_diffusion_cells
        || !matches!(config.solver_mode, GasSolverMode::LegacyHybrid)
    {
        if !preview.moves.is_empty() {
            preview.moves.clear();
        }
        return;
    }

    // Recompute only when something relevant changed
    if !debug_mode.is_changed()
        && !overlay_settings.is_changed()
        && !gas.is_changed()
        && !block_state.is_changed()
    {
        return;
    }

    let (_, moves) = preview_next_substep(
        &gas,
        &world,
        block_state.rng_state,
        block_state.phase,
        [config.diffusion_k_h2, config.diffusion_k_o2],
        config.max_flux_fraction,
        overlay_settings.preview_min_amount,
        overlay_settings.preview_min_flux,
    );
    preview.moves = moves;
}

fn update_debug_metrics(
    gas: Res<GasField>,
    world: Res<WorldGrid>,
    step: Res<SimulationStep>,
    mut metrics: ResMut<DebugGasMetrics>,
    mut baseline: ResMut<DebugMassBaseline>,
) {
    if !gas.is_changed() && !step.is_changed() {
        return;
    }

    let center = UVec2::new(WORLD_WIDTH / 2, WORLD_HEIGHT / 2);
    let radius = (WORLD_WIDTH.min(WORLD_HEIGHT) / 4).max(4) as f32;
    const ANGLES: usize = 16;
    let mut angular_samples = Vec::new();
    for i in 0..ANGLES {
        let a = (i as f32) * std::f32::consts::TAU / (ANGLES as f32);
        let x = center.x as f32 + radius * a.cos();
        let y = center.y as f32 + radius * a.sin();
        let xi = x.round().clamp(1.0, (WORLD_WIDTH - 2) as f32) as u32;
        let yi = y.round().clamp(1.0, (WORLD_HEIGHT - 2) as f32) as u32;
        if world.is_solid(xi, yi) || is_boundary(xi, yi) {
            continue;
        }
        angular_samples.push(gas.total_amount(xi, yi).max(0.0));
    }
    let mean = if angular_samples.is_empty() {
        0.0
    } else {
        angular_samples.iter().sum::<f32>() / angular_samples.len() as f32
    };
    let std = if angular_samples.is_empty() {
        0.0
    } else {
        let var = angular_samples
            .iter()
            .map(|v| {
                let d = *v - mean;
                d * d
            })
            .sum::<f32>()
            / angular_samples.len() as f32;
        var.sqrt()
    };
    metrics.anisotropy_score = if mean > 1e-6 { std / mean } else { 0.0 };

    let max_r = ((WORLD_WIDTH.min(WORLD_HEIGHT) / 2).saturating_sub(2)) as usize;
    let mut profile = vec![0.0f32; max_r + 1];
    let mut counts = vec![0u32; max_r + 1];
    for y in 1..WORLD_HEIGHT - 1 {
        for x in 1..WORLD_WIDTH - 1 {
            if world.is_solid(x, y) || is_boundary(x, y) {
                continue;
            }
            let dx = x as i32 - center.x as i32;
            let dy = y as i32 - center.y as i32;
            let r = (((dx * dx + dy * dy) as f32).sqrt().round() as usize).min(max_r);
            profile[r] += gas.total_amount(x, y).max(0.0);
            counts[r] += 1;
        }
    }
    for r in 0..=max_r {
        if counts[r] > 0 {
            profile[r] /= counts[r] as f32;
        }
    }
    let mut wave_acc = 0.0;
    let mut wave_n = 0u32;
    for r in 1..max_r {
        wave_acc += (profile[r - 1] - 2.0 * profile[r] + profile[r + 1]).abs();
        wave_n += 1;
    }
    let mean_profile = if profile.is_empty() {
        0.0
    } else {
        profile.iter().sum::<f32>() / profile.len() as f32
    };
    metrics.radial_wave_score = if wave_n > 0 && mean_profile > 1e-6 {
        (wave_acc / wave_n as f32) / mean_profile
    } else {
        0.0
    };

    let h2_total: f32 = gas
        .read
        .iter()
        .map(|cell| cell[GasKind::Hydrogen.index()])
        .sum();
    let o2_total: f32 = gas
        .read
        .iter()
        .map(|cell| cell[GasKind::Oxygen.index()])
        .sum();

    if !baseline.initialized || step.0 == 0 {
        baseline.initialized = true;
        baseline.h2 = h2_total;
        baseline.o2 = o2_total;
    }

    metrics.mass_error_h2 = if baseline.h2 > 1e-6 {
        ((h2_total - baseline.h2) / baseline.h2).abs()
    } else {
        0.0
    };
    metrics.mass_error_o2 = if baseline.o2 > 1e-6 {
        ((o2_total - baseline.o2) / baseline.o2).abs()
    } else {
        0.0
    };
}

fn draw_debug_overlays(
    debug_mode: Res<DebugMode>,
    overlay_settings: Res<DebugOverlaySettings>,
    preview: Res<DebugStepPreview>,
    gas: Res<GasField>,
    world: Res<WorldGrid>,
    camera_projection: Single<&Projection, With<MainCamera>>,
    mut gizmos: Gizmos,
    mut momentum_gizmos: Gizmos<MomentumVectorGizmoConfigGroup>,
) {
    if !debug_mode.active {
        return;
    }

    if overlay_settings.show_diffusion_cells {
        let cell_size = Vec2::splat(CELL_SIZE - 1.0);

        for &(fx, fy, tx, ty) in &preview.moves {
            let from_center = cell_center(fx, fy);
            let to_center = cell_center(tx, ty);

            // Green outline: cell that will emit particles
            gizmos.rect_2d(
                Isometry2d::from_translation(from_center),
                cell_size,
                Color::srgba(0.15, 1.0, 0.15, 0.9),
            );

            // Blue outline: neighbor that will receive particles
            gizmos.rect_2d(
                Isometry2d::from_translation(to_center),
                cell_size,
                Color::srgba(0.15, 0.55, 1.0, 0.9),
            );
        }
    }

    if !overlay_settings.show_momentum_vectors {
        return;
    }

    let world_units_per_pixel = match *camera_projection {
        Projection::Orthographic(ref orthographic) => orthographic.scale.max(1e-6),
        _ => 1.0,
    };
    // Make the arrow/dot threshold zoom-aware:
    // zoom in (scale < 1) -> smaller threshold -> more arrows become visible.
    // zoom out (scale > 1) -> larger threshold -> less micro-noise.
    let min_arrow_len_px = (2.0f32 * world_units_per_pixel).max(0.35);
    let dot_radius_world = world_units_per_pixel * 0.6;
    let arrow_scale = CELL_SIZE * 0.85;
    let arrow_head_len = CELL_SIZE * 0.25;
    let arrow_head_width = CELL_SIZE * 0.14;
    let color = Color::srgba(0.98, 0.90, 0.22, 0.92);

    for y in 1..WORLD_HEIGHT - 1 {
        for x in 1..WORLD_WIDTH - 1 {
            if is_boundary(x, y) || world.is_solid(x, y) {
                continue;
            }
            let mass = gas.total_amount(x, y).max(0.0);
            if mass <= 1e-6 {
                continue;
            }

            let momentum = gas.velocity(x, y) * mass;
            let momentum_len = momentum.length();
            if momentum_len <= 1e-4 {
                continue;
            }

            let dir = momentum / momentum_len;
            // Saturating map keeps arrows readable across many orders of magnitude.
            let normalized = (momentum_len / (momentum_len + 1.0)).clamp(0.0, 1.0);
            let start = cell_center(x, y);
            let tail_len_world = normalized * arrow_scale;
            let tail_len_px = tail_len_world / world_units_per_pixel;
            if tail_len_px < min_arrow_len_px {
                momentum_gizmos.circle_2d(start, dot_radius_world, color);
                continue;
            }
            let end = start + dir * tail_len_world;
            momentum_gizmos.line_2d(start, end, color);

            let head_len = arrow_head_len * normalized;
            let head_width = arrow_head_width * normalized;
            let perp = Vec2::new(-dir.y, dir.x);
            let left = end - dir * head_len + perp * head_width;
            let right = end - dir * head_len - perp * head_width;
            momentum_gizmos.line_2d(end, left, color);
            momentum_gizmos.line_2d(end, right, color);
        }
    }
}
