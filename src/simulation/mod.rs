pub mod backend;
pub mod gas;
pub mod gpu_solver;
pub mod perf_model;

use std::time::{Duration, Instant};

use bevy::prelude::*;

use self::{
    backend::{SimulationBackend, SimulationBackendConfig, WorldSizeConfig},
    gas::GasField,
    gpu_solver::{GpuGasSolver, GpuStepTimings},
};
use crate::{config::GasRegistry, save::WorldLoadState, world::grid::WorldGrid};

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
    pub velocity_damping: f32,
    pub species_eq_blend: f32,
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
            velocity_damping: 0.08,
            species_eq_blend: 0.28,
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
        app.init_resource::<WorldSizeConfig>()
            .init_resource::<SimulationStep>()
            .init_resource::<SimulationControl>()
            .init_resource::<SimulationBackendConfig>()
            .init_resource::<GasSimulationConfig>()
            .init_resource::<SimulationRateConfig>()
            .init_resource::<SimulationPerfStats>()
            .init_resource::<BlockSyncState>()
            .init_resource::<GpuRuntimeState>()
            .add_systems(Startup, initialize_gas_field_from_registry)
            .add_systems(Update, apply_fixed_rate_config)
            .add_systems(Update, mark_gpu_state_dirty)
            .add_systems(Update, mark_gpu_state_dirty_from_gas_edits)
            .add_systems(FixedUpdate, run_simulation_tick);
    }
}

fn initialize_gas_field_from_registry(mut commands: Commands, registry: Res<GasRegistry>) {
    commands.insert_resource(GasField::from_registry(&registry));
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
    mut control: ResMut<SimulationControl>,
    mut backend: ResMut<SimulationBackendConfig>,
    world_load_state: Res<WorldLoadState>,
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

    if !world_load_state.has_world {
        control.paused = true;
        return;
    }

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
    _gpu_state: &mut GpuRuntimeState,
    _gas: &mut GasField,
    _world: &WorldGrid,
    _config: &GasSimulationConfig,
    _step: &mut SimulationStep,
) -> Result<GpuStepTimings, String> {
    return Err(
        "GPU backend is temporarily disabled for dynamic gas-count simulation; switching to CPU."
            .to_string(),
    );
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
    let mass_error = max_relative_mass_error(&target_species_totals, &current_species_totals);
    let mass_fix_every = config.mass_fix_every_n_steps;
    let periodic_mass_fix = mass_fix_every > 0 && (step.0 + 1) % u64::from(mass_fix_every) == 0;
    let event_mass_fix = mass_error > config.mass_fix_error_threshold.max(0.0);
    let did_mass_fix = periodic_mass_fix || event_mass_fix;
    if did_mass_fix {
        gas.renormalize_species_mass(
            world,
            &target_species_totals,
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

fn max_relative_mass_error(target_totals: &[f32], current_totals: &[f32]) -> f32 {
    let mut max_error = 0.0f32;
    let len = target_totals.len().max(current_totals.len());
    for i in 0..len {
        let target = target_totals.get(i).copied().unwrap_or(0.0).max(0.0);
        let current = current_totals.get(i).copied().unwrap_or(0.0).max(0.0);
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
