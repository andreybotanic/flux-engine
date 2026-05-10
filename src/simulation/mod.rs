pub mod backend;
pub mod discrete_step;
pub mod gas;
pub mod gpu_solver;
pub mod parity;

use std::time::{Duration, Instant};

use bevy::prelude::*;

use self::{
    backend::{SimulationBackend, SimulationBackendConfig, WorldSizeConfig},
    gas::GasField,
    gpu_solver::{GpuGasSolver, GpuStepTimings},
};
use crate::{config::GasRegistry, save::WorldLoadState, world::grid::WorldGrid};

#[derive(Resource, Clone, Default, bevy::render::extract_resource::ExtractResource)]
/// Stores `SimulationStep` state.
pub struct SimulationStep(pub u64);

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimulationSpeed {
    #[default]
    X1,
    X2,
    X5,
}

impl SimulationSpeed {
    /// Runs `multiplier` logic.
    pub fn multiplier(self) -> u32 {
        match self {
            Self::X1 => 1,
            Self::X2 => 2,
            Self::X5 => 5,
        }
    }

    /// Runs `faster` logic.
    pub fn faster(self) -> Self {
        match self {
            Self::X1 => Self::X2,
            Self::X2 => Self::X5,
            Self::X5 => Self::X5,
        }
    }

    /// Runs `slower` logic.
    pub fn slower(self) -> Self {
        match self {
            Self::X1 => Self::X1,
            Self::X2 => Self::X1,
            Self::X5 => Self::X2,
        }
    }
}

#[derive(Resource, Clone, Copy)]
/// Stores `SimulationControl` state.
pub struct SimulationControl {
    pub paused: bool,
    pub speed: SimulationSpeed,
}

#[derive(Clone, Copy)]
/// Stores `SolverTuning` state.
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
            buoyancy_strength: 0.22,
            buoyancy_window_radius: 2,
            buoyancy_window_sigma: 1.2,
            buoyancy_gain: 3.2,
            buoyancy_alpha: 1.0,
            buoyancy_force_cap: 0.30,
        }
    }
}

#[derive(Resource, Clone, Copy)]
/// Stores `GasSimulationConfig` state.
pub struct GasSimulationConfig {
    pub enable_lbm_velocity: bool,
    pub enable_species_relaxation: bool,
    pub reconcile_every_n_steps: u32,
    pub mass_fix_every_n_steps: u32,
    pub mass_fix_error_threshold: f32,
    pub mass_fix_min_residual: f32,
    pub thermal_motion_scale: f32,
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
            thermal_motion_scale: 0.08,
            solver_tuning: SolverTuning::default(),
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// FixedUpdate extension points used by plugin-owned simulation behavior.
pub enum SimulationSet {
    PluginPreStep,
    CellGasStep,
}

#[derive(Resource, Clone, Copy)]
/// Stores `SimulationRateConfig` state.
pub struct SimulationRateConfig {
    pub target_hz: u32,
}

impl Default for SimulationRateConfig {
    fn default() -> Self {
        Self { target_hz: 30 }
    }
}

#[derive(Resource)]
/// Stores `SimulationPerfStats` state.
pub struct SimulationPerfStats {
    pub last_step_ms: f32,
    pub avg_step_ms: f32,
    pub actual_hz: f32,
    pub target_hz_effective: f32,
    pub last_pipe_step_ms: f32,
    pub avg_pipe_step_ms: f32,
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
            last_pipe_step_ms: 0.0,
            avg_pipe_step_ms: 0.0,
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
/// Stores `BlockSyncState` state.
pub struct BlockSyncState;

impl Default for BlockSyncState {
    fn default() -> Self {
        Self
    }
}

#[derive(Resource, Default)]
/// Stores `GpuRuntimeState` state.
pub struct GpuRuntimeState {
    solver: Option<GpuGasSolver>,
    needs_full_upload: bool,
    steps_since_readback: u32,
}

const GPU_RUNTIME_READBACK_INTERVAL: u32 = 1;

impl GpuRuntimeState {
    /// Marks GPU state dirty so the next cell-gas GPU step uploads CPU state first.
    pub fn mark_needs_full_upload(&mut self) {
        self.needs_full_upload = true;
    }

    /// Drops the current GPU solver so the next world run recreates buffers from CPU state.
    pub fn reset_solver(&mut self) {
        self.solver = None;
        self.needs_full_upload = false;
        self.steps_since_readback = 0;
    }
}

impl Default for SimulationControl {
    fn default() -> Self {
        Self {
            paused: true,
            speed: SimulationSpeed::X1,
        }
    }
}

/// Stores `GasSimulationPlugin` state.
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
            .add_systems(Startup, initialize_gas_state_from_registry)
            .add_systems(Update, apply_fixed_rate_config)
            .add_systems(Update, mark_gpu_state_dirty)
            .add_systems(Update, mark_gpu_state_dirty_from_gas_edits)
            .configure_sets(
                FixedUpdate,
                SimulationSet::PluginPreStep.before(SimulationSet::CellGasStep),
            )
            .add_systems(
                FixedUpdate,
                run_simulation_tick.in_set(SimulationSet::CellGasStep),
            );
    }
}

include!("runtime_tick_block.rs");
include!("simulation_tests_block.rs");
