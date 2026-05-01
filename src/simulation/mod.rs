pub mod gas;
pub mod gpu;

use std::time::{Duration, Instant};

use bevy::prelude::*;

use self::{gas::GasField, gpu::GasGpuPlugin};
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
    pub buoyancy_ref_mass: f32,
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
            buoyancy_strength: 0.06,
            buoyancy_ref_mass: 29.0,
            buoyancy_gain: 2.0,
            buoyancy_alpha: 0.75,
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
            .init_resource::<SimulationStep>()
            .init_resource::<SimulationControl>()
            .init_resource::<GasSimulationConfig>()
            .init_resource::<SimulationRateConfig>()
            .init_resource::<SimulationPerfStats>()
            .init_resource::<BlockSyncState>()
            .add_plugins(GasGpuPlugin)
            .add_systems(Update, apply_fixed_rate_config)
            .add_systems(FixedUpdate, run_simulation_tick);
    }
}

fn apply_fixed_rate_config(
    config: Res<SimulationRateConfig>,
    mut fixed_time: ResMut<Time<Fixed>>,
) {
    if !config.is_changed() {
        return;
    }
    let hz = config.target_hz.clamp(1, 1000) as f64;
    fixed_time.set_timestep_hz(hz);
}

fn run_simulation_tick(
    control: Res<SimulationControl>,
    rate: Res<SimulationRateConfig>,
    config: Res<GasSimulationConfig>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
    mut perf: ResMut<SimulationPerfStats>,
) {
    let speed_mult = control.speed.multiplier() as f32;
    perf.target_hz_effective = rate.target_hz.clamp(1, 1000) as f32 * speed_mult;

    if control.paused {
        return;
    }

    for _ in 0..control.speed.multiplier() {
        let started_at = Instant::now();
        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
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
    use crate::world::grid::{WORLD_HEIGHT, WORLD_WIDTH};

    fn total_species(gas: &GasField, kind: GasKind) -> f32 {
        gas.read.iter().map(|cell| cell[kind.index()]).sum()
    }

    #[test]
    fn simulation_hz_clamp_1_1000() {
        let lo = SimulationRateConfig { target_hz: 0 }.target_hz.clamp(1, 1000);
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
}
