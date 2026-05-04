use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    input::camera::MainCamera,
    simulation::{
        apply_gas_structures_pre_step, do_one_substep, gas::GasField, BlockSyncState,
        GasSimulationConfig, SimulationControl, SimulationStep,
    },
    world::{
        gas_structures::GasStructureGrid,
        grid::{cell_center, is_boundary, WorldGrid, CELL_SIZE, WORLD_HEIGHT, WORLD_WIDTH},
    },
};

#[derive(Resource, Default)]
pub struct DebugMode {
    pub active: bool,
}

#[derive(Resource, Clone, Copy)]
pub struct DebugOverlaySettings {
    pub show_momentum_vectors: bool,
}

impl Default for DebugOverlaySettings {
    fn default() -> Self {
        Self {
            show_momentum_vectors: false,
        }
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct DebugGasMetrics {
    pub anisotropy_score: f32,
    pub radial_wave_score: f32,
    pub mass_error_h2: f32,
    pub mass_error_o2: f32,
    pub mass_error_co2: f32,
}

#[derive(Resource, Default, Clone)]
struct DebugMassBaseline {
    initialized: bool,
    species: Vec<f32>,
    last_step: u64,
}

pub struct DebugPlugin;

#[derive(Default, Reflect, GizmoConfigGroup)]
struct MomentumVectorGizmoConfigGroup;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugMode>()
            .init_resource::<DebugOverlaySettings>()
            .init_resource::<DebugGasMetrics>()
            .init_resource::<DebugMassBaseline>()
            .init_gizmo_group::<MomentumVectorGizmoConfigGroup>()
            .add_systems(
                Update,
                (
                    configure_momentum_gizmo_line_width,
                    handle_debug_keys,
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
    structures: Res<GasStructureGrid>,
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
        let _ = apply_gas_structures_pre_step(&structures, &mut gas, &world);
        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
    }
}

fn update_debug_metrics(
    gas: Res<GasField>,
    gas_registry: Res<GasRegistry>,
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

    let totals = gas.species_totals(&world);
    let h2_index = gas_registry.index_of("h2");
    let o2_index = gas_registry.index_of("o2");
    let co2_index = gas_registry.index_of("co2");

    let external_gas_edit_without_step =
        baseline.initialized && gas.is_changed() && step.0 == baseline.last_step;
    if !baseline.initialized || step.0 == 0 || external_gas_edit_without_step {
        baseline.initialized = true;
        baseline.species = totals.clone();
    }

    metrics.mass_error_h2 = if let Some(idx) = h2_index {
        let base = baseline.species.get(idx).copied().unwrap_or(0.0);
        mass_error_value(base, totals[idx])
    } else {
        0.0
    };
    metrics.mass_error_o2 = if let Some(idx) = o2_index {
        let base = baseline.species.get(idx).copied().unwrap_or(0.0);
        mass_error_value(base, totals[idx])
    } else {
        0.0
    };
    metrics.mass_error_co2 = if let Some(idx) = co2_index {
        let base = baseline.species.get(idx).copied().unwrap_or(0.0);
        mass_error_value(base, totals[idx])
    } else {
        0.0
    };

    baseline.last_step = step.0;
}

fn mass_error_value(base: f32, current: f32) -> f32 {
    let base = base.max(0.0);
    let current = current.max(0.0);
    let diff = (current - base).abs();
    // For tiny baselines, relative error explodes and becomes uninformative.
    // In that zone, report absolute drift in "particles" instead.
    if base >= 1.0 {
        diff / base
    } else {
        diff
    }
}

fn draw_debug_overlays(
    debug_mode: Res<DebugMode>,
    overlay_settings: Res<DebugOverlaySettings>,
    gas: Res<GasField>,
    world: Res<WorldGrid>,
    camera_projection: Single<&Projection, With<MainCamera>>,
    mut momentum_gizmos: Gizmos<MomentumVectorGizmoConfigGroup>,
) {
    if !debug_mode.active {
        return;
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

#[cfg(test)]
mod tests {
    use super::mass_error_value;

    #[test]
    fn mass_error_is_relative_for_non_zero_baseline() {
        let err = mass_error_value(100.0, 110.0);
        assert!((err - 0.1).abs() < 1e-6);
    }

    #[test]
    fn mass_error_is_absolute_for_zero_baseline() {
        let err = mass_error_value(0.0, 42.0);
        assert!((err - 42.0).abs() < 1e-6);
    }

    #[test]
    fn mass_error_is_absolute_for_tiny_baseline() {
        let err = mass_error_value(1e-5, 1.8);
        assert!((err - (1.8 - 1e-5)).abs() < 1e-6);
    }
}
