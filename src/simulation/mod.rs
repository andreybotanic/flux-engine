pub mod backend;
pub mod gas;
pub mod gpu_solver;
pub mod perf_model;

use std::time::{Duration, Instant};

use bevy::prelude::*;

use self::{
    backend::{SimulationBackend, SimulationBackendConfig, WorldSizeConfig},
    gas::GasField,
    gpu_solver::{GpuGasSolver, GpuStepTimings, GpuTransferMode},
};
use crate::world::grid::WorldGrid;

#[derive(Resource, Clone, Default, bevy::render::extract_resource::ExtractResource)]
pub struct SimulationStep(pub u64);

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimulationSpeed {
    #[default]
    X1,
    X2,
    X5,
}

impl SimulationSpeed {
    pub fn multiplier(self) -> u32 {
        match self {
            Self::X1 => 1,
            Self::X2 => 2,
            Self::X5 => 5,
        }
    }

    pub fn faster(self) -> Self {
        match self {
            Self::X1 => Self::X2,
            Self::X2 => Self::X5,
            Self::X5 => Self::X5,
        }
    }

    pub fn slower(self) -> Self {
        match self {
            Self::X1 => Self::X1,
            Self::X2 => Self::X1,
            Self::X5 => Self::X2,
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct SimulationControl {
    pub paused: bool,
    pub speed: SimulationSpeed,
}

#[derive(Clone, Copy)]
pub struct SolverTuning {
    pub tau_even: f32,
    pub tau_odd: f32,
    pub target_cfl_like_limit: f32,
    pub enable_buoyancy: bool,
    pub buoyancy_strength: f32,
    pub buoyancy_window_radius: u8,
    pub buoyancy_window_sigma: f32,
    pub buoyancy_gain: f32,
    pub buoyancy_alpha: f32,
    pub buoyancy_force_cap: f32,
}

impl Default for SolverTuning {
    fn default() -> Self {
        Self {
            tau_even: 0.85,
            tau_odd: 1.15,
            target_cfl_like_limit: 0.85,
            enable_buoyancy: true,
            buoyancy_strength: 0.12,
            buoyancy_window_radius: 2,
            buoyancy_window_sigma: 1.2,
            buoyancy_gain: 2.2,
            buoyancy_alpha: 0.9,
            buoyancy_force_cap: 0.20,
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct GasSimulationConfig {
    pub enable_lbm_velocity: bool,
    pub enable_species_relaxation: bool,
    pub reconcile_every_n_steps: u32,
    pub mass_fix_every_n_steps: u32,
    pub mass_fix_error_threshold: f32,
    pub mass_fix_min_residual: f32,
    pub solver_tuning: SolverTuning,
}

impl Default for GasSimulationConfig {
    fn default() -> Self {
        Self {
            enable_lbm_velocity: true,
            enable_species_relaxation: true,
            reconcile_every_n_steps: 4,
            mass_fix_every_n_steps: 4,
            mass_fix_error_threshold: 1e-4,
            mass_fix_min_residual: 1e-5,
            solver_tuning: SolverTuning::default(),
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct SimulationRateConfig {
    pub target_hz: u32,
}

impl Default for SimulationRateConfig {
    fn default() -> Self {
        Self { target_hz: 30 }
    }
}

#[derive(Resource)]
pub struct SimulationPerfStats {
    pub last_step_ms: f32,
    pub avg_step_ms: f32,
    pub actual_hz: f32,
    pub target_hz_effective: f32,
    pub last_gpu_compute_ms: f32,
    pub last_upload_to_gpu_ms: f32,
    pub last_readback_from_gpu_ms: f32,
    pub last_step_total_ms: f32,
    window_started_at: Instant,
    window_steps: u32,
}

impl Default for SimulationPerfStats {
    fn default() -> Self {
        Self {
            last_step_ms: 0.0,
            avg_step_ms: 0.0,
            actual_hz: 0.0,
            target_hz_effective: 30.0,
            last_gpu_compute_ms: 0.0,
            last_upload_to_gpu_ms: 0.0,
            last_readback_from_gpu_ms: 0.0,
            last_step_total_ms: 0.0,
            window_started_at: Instant::now(),
            window_steps: 0,
        }
    }
}

#[derive(Resource)]
pub struct BlockSyncState;

impl Default for BlockSyncState {
    fn default() -> Self {
        Self
    }
}

#[derive(Resource, Default)]
pub struct GpuRuntimeState {
    solver: Option<GpuGasSolver>,
    needs_full_upload: bool,
    steps_since_readback: u32,
}

impl Default for SimulationControl {
    fn default() -> Self {
        Self {
            paused: true,
            speed: SimulationSpeed::X1,
        }
    }
}

pub struct GasSimulationPlugin;

impl Plugin for GasSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GasField>()
            .init_resource::<WorldSizeConfig>()
            .init_resource::<SimulationStep>()
            .init_resource::<SimulationControl>()
            .init_resource::<SimulationBackendConfig>()
            .init_resource::<GasSimulationConfig>()
            .init_resource::<SimulationRateConfig>()
            .init_resource::<SimulationPerfStats>()
            .init_resource::<BlockSyncState>()
            .init_resource::<GpuRuntimeState>()
            .add_systems(Update, apply_fixed_rate_config)
            .add_systems(Update, mark_gpu_state_dirty)
            .add_systems(Update, mark_gpu_state_dirty_from_gas_edits)
            .add_systems(FixedUpdate, run_simulation_tick);
    }
}

fn apply_fixed_rate_config(config: Res<SimulationRateConfig>, mut fixed_time: ResMut<Time<Fixed>>) {
    if !config.is_changed() {
        return;
    }
    let hz = config.target_hz.clamp(1, 1000) as f64;
    fixed_time.set_timestep_hz(hz);
}

fn mark_gpu_state_dirty(
    backend: Res<SimulationBackendConfig>,
    world: Res<WorldGrid>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    if backend.backend != SimulationBackend::Gpu {
        return;
    }
    // Avoid re-uploading GPU state during active simulation ticks; otherwise GPU progress
    // may be overwritten by stale CPU buffers every frame. External editor/world changes
    // are applied primarily while paused.
    if world.is_changed() {
        gpu_state.needs_full_upload = true;
    }
}

fn mark_gpu_state_dirty_from_gas_edits(
    backend: Res<SimulationBackendConfig>,
    control: Res<SimulationControl>,
    gas: Res<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    if backend.backend != SimulationBackend::Gpu {
        return;
    }
    if control.paused && gas.is_changed() {
        gpu_state.needs_full_upload = true;
    }
}

fn run_simulation_tick(
    control: Res<SimulationControl>,
    mut backend: ResMut<SimulationBackendConfig>,
    rate: Res<SimulationRateConfig>,
    config: Res<GasSimulationConfig>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
    mut perf: ResMut<SimulationPerfStats>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    let speed_mult = control.speed.multiplier() as f32;
    perf.target_hz_effective = rate.target_hz.clamp(1, 1000) as f32 * speed_mult;

    if control.paused {
        return;
    }

    for _ in 0..control.speed.multiplier() {
        let started_at = Instant::now();
        match backend.backend {
            SimulationBackend::Cpu => {
                do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
                perf.last_gpu_compute_ms = 0.0;
                perf.last_upload_to_gpu_ms = 0.0;
                perf.last_readback_from_gpu_ms = 0.0;
                perf.last_step_total_ms = 0.0;
            }
            SimulationBackend::Gpu => {
                let timings =
                    do_one_substep_gpu(&mut gpu_state, &mut gas, &world, &config, &mut step);
                match timings {
                    Ok(t) => {
                        perf.last_gpu_compute_ms = t.compute_gpu_ms;
                        perf.last_upload_to_gpu_ms = t.upload_to_gpu_ms;
                        perf.last_readback_from_gpu_ms = t.readback_from_gpu_ms;
                        perf.last_step_total_ms = t.step_total_ms;
                    }
                    Err(err) => {
                        bevy::log::warn!(
                            "GPU simulation backend unavailable, switching to CPU fallback: {}",
                            err
                        );
                        backend.backend = SimulationBackend::Cpu;
                        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
                        perf.last_gpu_compute_ms = 0.0;
                        perf.last_upload_to_gpu_ms = 0.0;
                        perf.last_readback_from_gpu_ms = 0.0;
                        perf.last_step_total_ms = 0.0;
                    }
                }
            }
        }
        let elapsed_ms = started_at.elapsed().as_secs_f32() * 1000.0;
        perf.last_step_ms = elapsed_ms;
        perf.avg_step_ms = if perf.avg_step_ms <= f32::EPSILON {
            elapsed_ms
        } else {
            perf.avg_step_ms * 0.9 + elapsed_ms * 0.1
        };
        perf.window_steps = perf.window_steps.saturating_add(1);
    }

    let window_elapsed = perf.window_started_at.elapsed();
    if window_elapsed >= Duration::from_millis(500) {
        let seconds = window_elapsed.as_secs_f32().max(1e-6);
        perf.actual_hz = perf.window_steps as f32 / seconds;
        perf.window_steps = 0;
        perf.window_started_at = Instant::now();
    }
}

fn do_one_substep_gpu(
    gpu_state: &mut GpuRuntimeState,
    gas: &mut GasField,
    world: &WorldGrid,
    config: &GasSimulationConfig,
    step: &mut SimulationStep,
) -> Result<GpuStepTimings, String> {
    const GPU_RUNTIME_READBACK_EVERY_STEPS: u32 = 1;

    if gpu_state.solver.is_none() {
        let (solver, _) = GpuGasSolver::from_cpu_state(world, gas)?;
        gpu_state.solver = Some(solver);
        gpu_state.needs_full_upload = false;
        gpu_state.steps_since_readback = 0;
    }

    let solver = gpu_state
        .solver
        .as_mut()
        .ok_or("GPU solver is not initialized".to_string())?;

    let mut upload_ms = 0.0f32;
    if gpu_state.needs_full_upload {
        let host_state = gas.to_gpu_host_state(world);
        upload_ms = solver.upload_state(&host_state)?;
        gpu_state.needs_full_upload = false;
        gpu_state.steps_since_readback = 0;
    }

    let (width, height) = solver.current_dimensions();
    let params = GpuGasSolver::params_from_config(config, width, height, step.0);
    let mut timings = solver.step(params, GpuTransferMode::RuntimeTransfer)?;
    timings.upload_to_gpu_ms = upload_ms;

    gpu_state.steps_since_readback = gpu_state.steps_since_readback.saturating_add(1);
    if gpu_state.steps_since_readback >= GPU_RUNTIME_READBACK_EVERY_STEPS {
        let rb_started = Instant::now();
        let state = solver.readback_state()?;
        timings.readback_from_gpu_ms = rb_started.elapsed().as_secs_f32() * 1000.0;
        gas.apply_gpu_host_state(&state);
        gpu_state.steps_since_readback = 0;
    } else {
        timings.readback_from_gpu_ms = 0.0;
    }
    timings.step_total_ms += timings.readback_from_gpu_ms + upload_ms;
    step.0 += 1;
    Ok(timings)
}

pub fn do_one_substep(
    _block_state: &mut BlockSyncState,
    gas: &mut GasField,
    world: &WorldGrid,
    config: &GasSimulationConfig,
    step: &mut SimulationStep,
) {
    let target_species_totals = gas.species_totals(world);

    gas.step_lbm_unified(
        world,
        &config.solver_tuning,
        config.enable_species_relaxation,
        config.enable_lbm_velocity,
    );

    let current_species_totals = gas.species_totals(world);
    let mass_error = max_relative_mass_error(target_species_totals, current_species_totals);
    let mass_fix_every = config.mass_fix_every_n_steps;
    let periodic_mass_fix = mass_fix_every > 0 && (step.0 + 1) % u64::from(mass_fix_every) == 0;
    let event_mass_fix = mass_error > config.mass_fix_error_threshold.max(0.0);
    let did_mass_fix = periodic_mass_fix || event_mass_fix;
    if did_mass_fix {
        gas.renormalize_species_mass(
            world,
            target_species_totals,
            config.mass_fix_min_residual.max(0.0),
        );
    }

    gas.recompute_total_density_buffer(world);

    let reconcile_every = config.reconcile_every_n_steps;
    let periodic_reconcile = reconcile_every > 0 && (step.0 + 1) % u64::from(reconcile_every) == 0;
    if config.enable_lbm_velocity && (periodic_reconcile || event_mass_fix) {
        gas.reconcile_lbm_from_species(world);
    }

    step.0 += 1;
}

fn max_relative_mass_error(target_totals: [f32; 2], current_totals: [f32; 2]) -> f32 {
    let mut max_error = 0.0f32;
    for i in 0..target_totals.len() {
        let target = target_totals[i].max(0.0);
        let current = current_totals[i].max(0.0);
        let abs_error = (target - current).abs();
        let rel = if target > 1e-6 {
            abs_error / target
        } else {
            abs_error
        };
        max_error = max_error.max(rel);
    }
    max_error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::gas::GasKind;
    use crate::simulation::gpu_solver::{GpuGasSolver, GpuTransferMode};
    use crate::simulation::perf_model::PerfGasState;
    use crate::world::grid::{WORLD_HEIGHT, WORLD_WIDTH};

    fn total_species(gas: &GasField, kind: GasKind) -> f32 {
        gas.read.iter().map(|cell| cell[kind.index()]).sum()
    }

    fn fill_random_species_seeded(gas: &mut GasField, seed: u64, max_amount: f32) {
        let mut state = seed;
        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let r0 = ((state >> 32) as u32) as f32 / u32::MAX as f32;
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let r1 = ((state >> 32) as u32) as f32 / u32::MAX as f32;

                if r0 > 0.75 {
                    let h2 = (r0 - 0.75) / 0.25 * max_amount;
                    let _ = gas.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, h2);
                }
                if r1 > 0.80 {
                    let o2 = (r1 - 0.80) / 0.20 * max_amount;
                    let _ = gas.apply_species_delta_with_lbm(x, y, GasKind::Oxygen, o2);
                }
            }
        }
    }

    fn seed_diagonal_corner_squares(
        gas: &mut GasField,
        square_size: u32,
        per_cell_amount: f32,
    ) {
        let inset = 4u32;
        let left_x0 = inset;
        let left_y0 = inset;
        let right_x0 = WORLD_WIDTH - inset - square_size;
        let right_y0 = WORLD_HEIGHT - inset - square_size;

        for y in left_y0..left_y0 + square_size {
            for x in left_x0..left_x0 + square_size {
                let _ = gas.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, per_cell_amount);
            }
        }

        for y in right_y0..right_y0 + square_size {
            for x in right_x0..right_x0 + square_size {
                let _ = gas.apply_species_delta_with_lbm(x, y, GasKind::Oxygen, per_cell_amount);
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FieldParityMetrics {
        total_density_rel_l2: f32,
        h2_rel_l2: f32,
        o2_rel_l2: f32,
        velocity_rmse: f32,
    }

    fn field_parity_metrics(cpu: &GasField, gpu: &GasField) -> FieldParityMetrics {
        let mut sum_total_diff_sq = 0.0f32;
        let mut sum_total_ref_sq = 0.0f32;
        let mut sum_h2_diff_sq = 0.0f32;
        let mut sum_h2_ref_sq = 0.0f32;
        let mut sum_o2_diff_sq = 0.0f32;
        let mut sum_o2_ref_sq = 0.0f32;
        let mut sum_vel_diff_sq = 0.0f32;
        let mut count = 0u32;

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                let cpu_total = cpu.total_amount(x, y).max(0.0);
                let gpu_total = gpu.total_amount(x, y).max(0.0);
                let total_diff = cpu_total - gpu_total;
                sum_total_diff_sq += total_diff * total_diff;
                sum_total_ref_sq += cpu_total * cpu_total;

                let cpu_h2 = cpu.amount(x, y, GasKind::Hydrogen).max(0.0);
                let gpu_h2 = gpu.amount(x, y, GasKind::Hydrogen).max(0.0);
                let h2_diff = cpu_h2 - gpu_h2;
                sum_h2_diff_sq += h2_diff * h2_diff;
                sum_h2_ref_sq += cpu_h2 * cpu_h2;

                let cpu_o2 = cpu.amount(x, y, GasKind::Oxygen).max(0.0);
                let gpu_o2 = gpu.amount(x, y, GasKind::Oxygen).max(0.0);
                let o2_diff = cpu_o2 - gpu_o2;
                sum_o2_diff_sq += o2_diff * o2_diff;
                sum_o2_ref_sq += cpu_o2 * cpu_o2;

                let dv = cpu.velocity(x, y) - gpu.velocity(x, y);
                sum_vel_diff_sq += dv.length_squared();
                count += 1;
            }
        }

        FieldParityMetrics {
            total_density_rel_l2: (sum_total_diff_sq / (sum_total_ref_sq + 1e-6)).sqrt(),
            h2_rel_l2: (sum_h2_diff_sq / (sum_h2_ref_sq + 1e-6)).sqrt(),
            o2_rel_l2: (sum_o2_diff_sq / (sum_o2_ref_sq + 1e-6)).sqrt(),
            velocity_rmse: (sum_vel_diff_sq / count.max(1) as f32).sqrt(),
        }
    }

    #[test]
    fn simulation_hz_clamp_1_1000() {
        let lo = SimulationRateConfig { target_hz: 0 }
            .target_hz
            .clamp(1, 1000);
        let hi = SimulationRateConfig { target_hz: 5000 }
            .target_hz
            .clamp(1, 1000);
        assert_eq!(lo, 1);
        assert_eq!(hi, 1000);
    }

    #[test]
    fn unified_only_runtime_path() {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 5_000.0);

        let mut step = SimulationStep(0);
        let mut state = BlockSyncState::default();
        let cfg = GasSimulationConfig::default();
        for _ in 0..200 {
            do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
        }

        let total = total_species(&gas, GasKind::Hydrogen);
        assert!(total.is_finite() && total > 0.0);
        assert!(gas
            .read
            .iter()
            .flat_map(|cell| cell.iter())
            .all(|v| v.is_finite() && *v >= -1e-5));
    }

    #[test]
    fn no_nan_inf_long_run() {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);

        let mut step = SimulationStep(0);
        let mut state = BlockSyncState::default();
        let cfg = GasSimulationConfig::default();
        for _ in 0..4_000 {
            do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
        }

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                for kind in GasKind::ALL {
                    let amount = gas.amount(x, y, kind);
                    assert!(amount.is_finite());
                    assert!(amount >= -1e-4);
                }
                let v = gas.velocity(x, y);
                assert!(v.x.is_finite() && v.y.is_finite());
            }
        }
    }

    #[test]
    fn hole_jet_direction_with_context_buoyancy() {
        let mut world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        let left = 42;
        let right = 60;
        let bottom = 12;
        let top = 28;
        let hole_x = (left + right) / 2;

        for y in bottom..=top {
            for x in left..=right {
                let is_wall = x == left || x == right || y == bottom || y == top;
                if !is_wall {
                    continue;
                }
                if y == top && x == hole_x {
                    continue;
                }
                let _ = world.set_solid(x, y);
            }
        }
        let chimney_left = hole_x.saturating_sub(1);
        let chimney_right = (hole_x + 1).min(WORLD_WIDTH - 2);
        for y in top + 1..=(top + 18).min(WORLD_HEIGHT - 2) {
            let _ = world.set_solid(chimney_left, y);
            let _ = world.set_solid(chimney_right, y);
        }

        for y in bottom + 1..top {
            for x in left + 1..right {
                let _ = gas.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, 120.0);
            }
        }

        let mut step = SimulationStep(0);
        let mut state = BlockSyncState::default();
        let mut cfg = GasSimulationConfig::default();
        cfg.solver_tuning.enable_buoyancy = true;
        cfg.solver_tuning.buoyancy_strength = 0.25;
        cfg.solver_tuning.buoyancy_window_radius = 2;
        cfg.solver_tuning.buoyancy_window_sigma = 1.2;
        cfg.solver_tuning.buoyancy_gain = 2.0;
        cfg.solver_tuning.buoyancy_alpha = 0.75;
        cfg.solver_tuning.buoyancy_force_cap = 0.3;

        for _ in 0..220 {
            do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
        }

        let mut upward_flux = 0.0f32;
        let mut lateral_flux = 0.0f32;
        for y in top + 1..=(top + 18).min(WORLD_HEIGHT - 2) {
            for x in left.saturating_sub(8)..=(right + 8).min(WORLD_WIDTH - 2) {
                if world.is_solid(x, y) {
                    continue;
                }
                let rho = gas.total_amount(x, y).max(0.0);
                let v = gas.velocity(x, y);
                if x == hole_x {
                    upward_flux += rho * v.y.max(0.0);
                } else {
                    lateral_flux += rho * v.x.abs();
                }
            }
        }

        let ratio = upward_flux / (lateral_flux + 1e-6);
        assert!(
            ratio > 1.0,
            "Expected upward jet dominance, got ratio={ratio:.3}, upward={upward_flux:.3}, lateral={lateral_flux:.3}"
        );
    }

    #[test]
    fn cpu_gpu_parity_with_tolerance_on_102_world() {
        if std::env::var("RUN_GPU_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping GPU parity test (set RUN_GPU_TESTS=1 to enable)");
            return;
        }

        let world = WorldGrid::default();
        let mut gas_cpu = GasField::default();
        gas_cpu.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas_cpu.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 8_000.0);
        let _ = gas_cpu.apply_species_delta_with_lbm(cx + 1, cy, GasKind::Oxygen, 2_500.0);

        let mut gas_gpu = gas_cpu.clone();
        let mut gpu_solver = match GpuGasSolver::from_cpu_state(&world, &gas_gpu) {
            Ok((solver, _)) => solver,
            Err(err) => {
                eprintln!("Skipping GPU parity test: GPU unavailable ({err})");
                return;
            }
        };

        let cfg = GasSimulationConfig::default();
        let mut cpu_state = BlockSyncState::default();
        let mut cpu_step = SimulationStep(0);
        let mut gpu_step = 0u64;

        for _ in 0..120 {
            do_one_substep(&mut cpu_state, &mut gas_cpu, &world, &cfg, &mut cpu_step);

            let (w, h) = gpu_solver.current_dimensions();
            let params = GpuGasSolver::params_from_config(&cfg, w, h, gpu_step);
            let _ = gpu_solver
                .step(params, GpuTransferMode::RuntimeTransfer)
                .expect("GPU step failed");
            let rb = gpu_solver.readback_state().expect("GPU readback failed");
            gas_gpu.apply_gpu_host_state(&rb);
            gpu_step += 1;
        }

        let cpu_h2 = total_species(&gas_cpu, GasKind::Hydrogen);
        let gpu_h2 = total_species(&gas_gpu, GasKind::Hydrogen);
        let cpu_o2 = total_species(&gas_cpu, GasKind::Oxygen);
        let gpu_o2 = total_species(&gas_gpu, GasKind::Oxygen);

        let rel_h2 = (cpu_h2 - gpu_h2).abs() / cpu_h2.max(1e-6);
        let rel_o2 = (cpu_o2 - gpu_o2).abs() / cpu_o2.max(1e-6);
        assert!(
            rel_h2 < 0.03,
            "H2 totals diverged too much: cpu={cpu_h2}, gpu={gpu_h2}, rel={rel_h2}"
        );
        assert!(
            rel_o2 < 0.03,
            "O2 totals diverged too much: cpu={cpu_o2}, gpu={gpu_o2}, rel={rel_o2}"
        );

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                for kind in GasKind::ALL {
                    let a = gas_cpu.amount(x, y, kind);
                    let b = gas_gpu.amount(x, y, kind);
                    assert!(a.is_finite() && b.is_finite());
                    assert!(a >= -1e-4 && b >= -1e-4);
                }
            }
        }
    }

    #[test]
    fn cpu_gpu_initial_state_deterministic_on_upload() {
        if std::env::var("RUN_GPU_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping GPU deterministic upload test (set RUN_GPU_TESTS=1 to enable)");
            return;
        }

        let world = WorldGrid::default();
        let gas_cpu = GasField::default();
        let host = gas_cpu.to_gpu_host_state(&world);

        let solver = match GpuGasSolver::new(WORLD_WIDTH, WORLD_HEIGHT) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Skipping deterministic upload test: GPU unavailable ({err})");
                return;
            }
        };
        let mut solver = solver;
        solver.upload_state(&host).expect("GPU upload failed");
        let rb = solver.readback_state().expect("GPU readback failed");

        assert_eq!(host.species.len(), rb.species.len());
        assert_eq!(host.lbm_flat.len(), rb.lbm_flat.len());
        assert_eq!(host.total_density.len(), rb.total_density.len());
        assert_eq!(host.velocity.len(), rb.velocity.len());

        for (a, b) in host.species.iter().zip(rb.species.iter()) {
            assert!((a[0] - b[0]).abs() <= 1e-6);
            assert!((a[1] - b[1]).abs() <= 1e-6);
        }
        for (a, b) in host.lbm_flat.iter().zip(rb.lbm_flat.iter()) {
            assert!((a - b).abs() <= 1e-6);
        }
        for (a, b) in host.total_density.iter().zip(rb.total_density.iter()) {
            assert!((a - b).abs() <= 1e-6);
        }
        for (a, b) in host.velocity.iter().zip(rb.velocity.iter()) {
            assert!((a[0] - b[0]).abs() <= 1e-6);
            assert!((a[1] - b[1]).abs() <= 1e-6);
        }
    }

    #[test]
    fn cpu_gpu_parity_random_seeded_mass_on_102_world() {
        if std::env::var("RUN_GPU_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping random GPU parity test (set RUN_GPU_TESTS=1 to enable)");
            return;
        }

        let world = WorldGrid::default();
        let mut gas_cpu = GasField::default();
        gas_cpu.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        fill_random_species_seeded(&mut gas_cpu, 0x5EED_BAAD_F00D_1234, 150.0);

        let mut gas_gpu = gas_cpu.clone();
        let mut gpu_solver = match GpuGasSolver::from_cpu_state(&world, &gas_gpu) {
            Ok((solver, _)) => solver,
            Err(err) => {
                eprintln!("Skipping random GPU parity test: GPU unavailable ({err})");
                return;
            }
        };

        let cfg = GasSimulationConfig::default();
        let mut cpu_state = BlockSyncState::default();
        let mut cpu_step = SimulationStep(0);
        let mut gpu_step = 0u64;

        for _ in 0..60 {
            do_one_substep(&mut cpu_state, &mut gas_cpu, &world, &cfg, &mut cpu_step);

            let (w, h) = gpu_solver.current_dimensions();
            let params = GpuGasSolver::params_from_config(&cfg, w, h, gpu_step);
            let _ = gpu_solver
                .step(params, GpuTransferMode::RuntimeTransfer)
                .expect("GPU step failed");
            let rb = gpu_solver.readback_state().expect("GPU readback failed");
            gas_gpu.apply_gpu_host_state(&rb);
            gpu_step += 1;
        }

        let cpu_h2 = total_species(&gas_cpu, GasKind::Hydrogen);
        let gpu_h2 = total_species(&gas_gpu, GasKind::Hydrogen);
        let cpu_o2 = total_species(&gas_cpu, GasKind::Oxygen);
        let gpu_o2 = total_species(&gas_gpu, GasKind::Oxygen);

        let rel_h2 = (cpu_h2 - gpu_h2).abs() / cpu_h2.max(1e-6);
        let rel_o2 = (cpu_o2 - gpu_o2).abs() / cpu_o2.max(1e-6);
        assert!(
            rel_h2 < 0.05,
            "Random-seeded H2 totals diverged too much: cpu={cpu_h2}, gpu={gpu_h2}, rel={rel_h2}"
        );
        assert!(
            rel_o2 < 0.05,
            "Random-seeded O2 totals diverged too much: cpu={cpu_o2}, gpu={gpu_o2}, rel={rel_o2}"
        );
    }

    #[test]
    fn cpu_gpu_parity_diagonal_squares_density_velocity_on_102_world() {
        if std::env::var("RUN_GPU_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping strict GPU parity test (set RUN_GPU_TESTS=1 to enable)");
            return;
        }

        let world = WorldGrid::default();
        let mut gas_cpu = GasField::default();
        gas_cpu.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        seed_diagonal_corner_squares(&mut gas_cpu, 15, 100.0);

        let mut gas_gpu = gas_cpu.clone();
        let mut gpu_solver = match GpuGasSolver::from_cpu_state(&world, &gas_gpu) {
            Ok((solver, _)) => solver,
            Err(err) => {
                eprintln!("Skipping strict GPU parity test: GPU unavailable ({err})");
                return;
            }
        };

        let cfg = GasSimulationConfig::default();
        let mut cpu_state = BlockSyncState::default();
        let mut cpu_step = SimulationStep(0);
        let mut gpu_step = 0u64;

        let mut metrics_step_100 = None;
        let mut metrics_step_200 = None;

        for i in 0..200u32 {
            do_one_substep(&mut cpu_state, &mut gas_cpu, &world, &cfg, &mut cpu_step);

            let (w, h) = gpu_solver.current_dimensions();
            let params = GpuGasSolver::params_from_config(&cfg, w, h, gpu_step);
            gpu_solver
                .step(params, GpuTransferMode::RuntimeTransfer)
                .expect("GPU step failed");
            let rb = gpu_solver.readback_state().expect("GPU readback failed");
            gas_gpu.apply_gpu_host_state(&rb);
            gpu_step += 1;

            if i == 99 {
                metrics_step_100 = Some(field_parity_metrics(&gas_cpu, &gas_gpu));
            } else if i == 199 {
                metrics_step_200 = Some(field_parity_metrics(&gas_cpu, &gas_gpu));
            }
        }

        let m100 = metrics_step_100.expect("missing metrics for step 100");
        let m200 = metrics_step_200.expect("missing metrics for step 200");
        eprintln!("Strict parity step 100 metrics: {:?}", m100);
        eprintln!("Strict parity step 200 metrics: {:?}", m200);

        let density_rel_l2_limit_step_100 = 1e-3;
        let density_rel_l2_limit_step_200 = 1e-3;
        let species_rel_l2_limit_step_100 = 1e-3;
        let species_rel_l2_limit_step_200 = 1e-3;
        let velocity_rmse_limit = 1e-3;

        assert!(
            m100.total_density_rel_l2 <= density_rel_l2_limit_step_100,
            "Step 100 total density mismatch too high: {:?}, limits: density<= {}",
            m100,
            density_rel_l2_limit_step_100
        );
        assert!(
            m100.h2_rel_l2 <= species_rel_l2_limit_step_100
                && m100.o2_rel_l2 <= species_rel_l2_limit_step_100,
            "Step 100 species mismatch too high: {:?}, limits: H2/O2 <= {}",
            m100,
            species_rel_l2_limit_step_100
        );
        assert!(
            m100.velocity_rmse <= velocity_rmse_limit,
            "Step 100 velocity mismatch too high: {:?}, limit: velocity_rmse <= {}",
            m100,
            velocity_rmse_limit
        );

        assert!(
            m200.total_density_rel_l2 <= density_rel_l2_limit_step_200,
            "Step 200 total density mismatch too high: {:?}, limits: density<= {}",
            m200,
            density_rel_l2_limit_step_200
        );
        assert!(
            m200.h2_rel_l2 <= species_rel_l2_limit_step_200
                && m200.o2_rel_l2 <= species_rel_l2_limit_step_200,
            "Step 200 species mismatch too high: {:?}, limits: H2/O2 <= {}",
            m200,
            species_rel_l2_limit_step_200
        );
        assert!(
            m200.velocity_rmse <= velocity_rmse_limit,
            "Step 200 velocity mismatch too high: {:?}, limit: velocity_rmse <= {}",
            m200,
            velocity_rmse_limit
        );
    }

    #[test]
    fn cpu_smoke_large_worlds_502_and_1002() {
        let cfg = GasSimulationConfig::default();
        for (w, h) in [(502u32, 502u32), (1002u32, 1002u32)] {
            let mut state = PerfGasState::seeded(w, h);
            for _ in 0..5 {
                state.do_one_substep(&cfg);
            }
            let totals = state.species_totals();
            assert!(totals[0].is_finite() && totals[1].is_finite());
            assert!(state
                .species_read
                .iter()
                .flat_map(|cell| cell.iter())
                .all(|v| v.is_finite() && *v >= -1e-4));
        }
    }

    #[test]
    fn gpu_smoke_large_worlds_502_and_1002() {
        if std::env::var("RUN_GPU_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping GPU smoke tests (set RUN_GPU_TESTS=1 to enable)");
            return;
        }

        let cfg = GasSimulationConfig::default();
        for (w, h) in [(502u32, 502u32), (1002u32, 1002u32)] {
            let mut state = PerfGasState::seeded(w, h);
            let mut solver = match GpuGasSolver::new(w, h) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("Skipping GPU smoke for {w}x{h}: {err}");
                    return;
                }
            };
            solver
                .upload_state(&state.to_gpu_host_state())
                .expect("GPU upload failed");
            for _ in 0..5 {
                let params = GpuGasSolver::params_from_config(&cfg, w, h, state.step);
                solver
                    .step(params, GpuTransferMode::ForcedFullReadback)
                    .expect("GPU step failed");
                let rb = solver.readback_state().expect("GPU readback failed");
                state.apply_gpu_host_state(&rb);
                state.step += 1;
            }
            let totals = state.species_totals();
            assert!(totals[0].is_finite() && totals[1].is_finite());
            assert!(state
                .species_read
                .iter()
                .flat_map(|cell| cell.iter())
                .all(|v| v.is_finite() && *v >= -1e-4));
        }
    }
}
