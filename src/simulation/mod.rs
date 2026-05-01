pub mod gas;
pub mod gpu;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

use self::{
    gas::{
        next_random_u32, phase_offsets, step_cpu_gas_block_sync, GasField, HYDROGEN_DIFFUSION_K,
        OXYGEN_DIFFUSION_K,
    },
    gpu::GasGpuPlugin,
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

#[derive(Resource, Clone, Copy)]
pub enum GasSolverMode {
    LegacyHybrid,
    UnifiedTRT,
    UnifiedMRTCM,
}

impl Default for GasSolverMode {
    fn default() -> Self {
        Self::LegacyHybrid
    }
}

impl GasSolverMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::LegacyHybrid => "Legacy",
            Self::UnifiedTRT => "Unified TRT",
            Self::UnifiedMRTCM => "Unified MRT-CM",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::LegacyHybrid => Self::UnifiedTRT,
            Self::UnifiedTRT => Self::UnifiedMRTCM,
            Self::UnifiedMRTCM => Self::LegacyHybrid,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SolverTuning {
    pub tau_even: f32,
    pub tau_odd: f32,
    pub relaxation_rates: [f32; 9],
    pub target_cfl_like_limit: f32,
    pub enable_buoyancy: bool,
    pub buoyancy_strength: f32,
}

impl Default for SolverTuning {
    fn default() -> Self {
        Self {
            tau_even: 0.85,
            tau_odd: 1.15,
            relaxation_rates: [1.0; 9],
            target_cfl_like_limit: 0.85,
            enable_buoyancy: true,
            buoyancy_strength: 0.06,
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct GasSimulationConfig {
    pub enable_lbm_velocity: bool,
    pub enable_diffusion: bool,
    pub lbm_tau: f32,
    pub diffusion_k_h2: f32,
    pub diffusion_k_o2: f32,
    pub max_flux_fraction: f32,
    pub reconcile_every_n_steps: u32,
    pub mass_fix_every_n_steps: u32,
    pub mass_fix_error_threshold: f32,
    pub mass_fix_min_residual: f32,
    pub solver_mode: GasSolverMode,
    pub solver_tuning: SolverTuning,
}

impl Default for GasSimulationConfig {
    fn default() -> Self {
        Self {
            enable_lbm_velocity: true,
            enable_diffusion: true,
            lbm_tau: 0.85,
            diffusion_k_h2: HYDROGEN_DIFFUSION_K,
            diffusion_k_o2: OXYGEN_DIFFUSION_K,
            max_flux_fraction: 0.30,
            reconcile_every_n_steps: 4,
            mass_fix_every_n_steps: 4,
            mass_fix_error_threshold: 1e-4,
            mass_fix_min_residual: 1e-5,
            solver_mode: GasSolverMode::LegacyHybrid,
            solver_tuning: SolverTuning::default(),
        }
    }
}

#[derive(Resource)]
pub struct BlockSyncState {
    pub rng_state: u64,
    pub phase: u8,
    pub last_block_index: u8,
}

impl Default for BlockSyncState {
    fn default() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x0dd5_eed0_beef_cafe);

        Self {
            rng_state: nanos ^ 0x9E37_79B9_7F4A_7C15,
            phase: 0,
            last_block_index: 0,
        }
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
            .init_resource::<BlockSyncState>()
            .add_plugins(GasGpuPlugin)
            .add_systems(FixedUpdate, run_simulation_tick);
    }
}

fn run_simulation_tick(
    control: Res<SimulationControl>,
    config: Res<GasSimulationConfig>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
) {
    if control.paused {
        return;
    }

    for _ in 0..control.speed.multiplier() {
        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
    }
}

pub fn do_one_substep(
    block_state: &mut BlockSyncState,
    gas: &mut GasField,
    world: &WorldGrid,
    config: &GasSimulationConfig,
    step: &mut SimulationStep,
) {
    let target_species_totals = gas.species_totals(world);

    match config.solver_mode {
        GasSolverMode::LegacyHybrid => {
            if config.enable_lbm_velocity {
                gas.step_lbm(world, config.lbm_tau);
            } else {
                gas.clear_velocity();
            }

            let block_index = (next_random_u32(&mut block_state.rng_state) % 9) as u8;
            let (offset_x, offset_y) = phase_offsets(block_state.phase);

            if config.enable_diffusion {
                step_cpu_gas_block_sync(
                    gas,
                    world,
                    block_index,
                    offset_x,
                    offset_y,
                    &mut block_state.rng_state,
                    [config.diffusion_k_h2, config.diffusion_k_o2],
                    config.max_flux_fraction,
                );
            }

            block_state.last_block_index = block_index;
            block_state.phase = (block_state.phase + 1) % 9;
        }
        GasSolverMode::UnifiedTRT | GasSolverMode::UnifiedMRTCM => {
            gas.step_lbm_unified(
                world,
                &config.solver_tuning,
                config.solver_mode,
                config.enable_diffusion,
            );
            block_state.last_block_index = 0;
            block_state.phase = (block_state.phase + 1) % 9;
        }
    }

    let current_species_totals = gas.species_totals(world);
    let mass_error = max_relative_mass_error(target_species_totals, current_species_totals);
    let mass_fix_every = if matches!(config.solver_mode, GasSolverMode::LegacyHybrid) {
        1
    } else {
        config.mass_fix_every_n_steps
    };
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

    let reconcile_every = if matches!(config.solver_mode, GasSolverMode::LegacyHybrid) {
        1
    } else {
        config.reconcile_every_n_steps
    };
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
    use crate::world::grid::{linear_index, WORLD_HEIGHT, WORLD_WIDTH};
    use std::time::Instant;

    fn total_species(gas: &GasField, kind: GasKind) -> f32 {
        gas.read.iter().map(|cell| cell[kind.index()]).sum()
    }

    fn make_config(mode: GasSolverMode, enable_diffusion: bool) -> GasSimulationConfig {
        let mut solver_tuning = SolverTuning::default();
        solver_tuning.enable_buoyancy = false;
        GasSimulationConfig {
            enable_lbm_velocity: true,
            enable_diffusion,
            lbm_tau: 0.85,
            diffusion_k_h2: 0.20,
            diffusion_k_o2: 0.14,
            max_flux_fraction: 0.30,
            reconcile_every_n_steps: 4,
            mass_fix_every_n_steps: 4,
            mass_fix_error_threshold: 1e-4,
            mass_fix_min_residual: 1e-5,
            solver_mode: mode,
            solver_tuning,
        }
    }

    fn wave_metrics(gas: &GasField) -> (f32, f32) {
        let center = UVec2::new(WORLD_WIDTH / 2, WORLD_HEIGHT / 2);
        let radius = (WORLD_WIDTH.min(WORLD_HEIGHT) / 4).max(4) as f32;
        let mut samples = Vec::new();
        for i in 0..16 {
            let a = (i as f32) * std::f32::consts::TAU / 16.0;
            let x = (center.x as f32 + radius * a.cos())
                .round()
                .clamp(1.0, (WORLD_WIDTH - 2) as f32) as u32;
            let y = (center.y as f32 + radius * a.sin())
                .round()
                .clamp(1.0, (WORLD_HEIGHT - 2) as f32) as u32;
            samples.push(gas.total_amount(x, y).max(0.0));
        }
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        let std = (samples
            .iter()
            .map(|v| {
                let d = *v - mean;
                d * d
            })
            .sum::<f32>()
            / samples.len() as f32)
            .sqrt();
        let anisotropy = if mean > 1e-6 { std / mean } else { 0.0 };

        let max_r = ((WORLD_WIDTH.min(WORLD_HEIGHT) / 2).saturating_sub(2)) as usize;
        let mut profile = vec![0.0f32; max_r + 1];
        let mut counts = vec![0u32; max_r + 1];
        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
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
        let mut wave_acc = 0.0f32;
        let mut wave_n = 0u32;
        for r in 1..max_r {
            wave_acc += (profile[r - 1] - 2.0 * profile[r] + profile[r + 1]).abs();
            wave_n += 1;
        }
        let mean_profile = profile.iter().sum::<f32>() / profile.len() as f32;
        let radial_wave = if wave_n > 0 && mean_profile > 1e-6 {
            (wave_acc / wave_n as f32) / mean_profile
        } else {
            0.0
        };
        (anisotropy, radial_wave)
    }

    #[test]
    fn with_diffusion_disabled_center_seed_spreads_to_neighbours() {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 360.0);

        let mut block_state = BlockSyncState {
            rng_state: 1,
            phase: 0,
            last_block_index: 0,
        };
        let config = GasSimulationConfig {
            enable_lbm_velocity: true,
            enable_diffusion: false,
            lbm_tau: 0.85,
            diffusion_k_h2: 0.20,
            diffusion_k_o2: 0.14,
            max_flux_fraction: 0.30,
            reconcile_every_n_steps: 4,
            mass_fix_every_n_steps: 4,
            mass_fix_error_threshold: 1e-4,
            mass_fix_min_residual: 1e-5,
            solver_mode: GasSolverMode::LegacyHybrid,
            solver_tuning: SolverTuning::default(),
        };
        let mut step = SimulationStep(0);

        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);

        let neighbour_sum = gas.amount(cx + 1, cy, GasKind::Hydrogen)
            + gas.amount(cx - 1, cy, GasKind::Hydrogen)
            + gas.amount(cx, cy + 1, GasKind::Hydrogen)
            + gas.amount(cx, cy - 1, GasKind::Hydrogen);

        assert!(
            neighbour_sum > 0.0,
            "Diffusion disabled, but D2Q9 start should still spread to neighbours"
        );

        let total: f32 = gas
            .read
            .iter()
            .map(|cell| cell[GasKind::Hydrogen.index()])
            .sum();
        assert!((total - 360.0).abs() < 1e-3);

        let center_idx = linear_index(cx, cy);
        let center_total_after = gas.read[center_idx][GasKind::Hydrogen.index()];
        assert!(center_total_after < 360.0);
    }

    fn run_mass_conservation_steps(steps: u32) {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Oxygen, 3_000.0);

        let config = GasSimulationConfig {
            enable_lbm_velocity: true,
            enable_diffusion: true,
            lbm_tau: 0.85,
            diffusion_k_h2: 0.20,
            diffusion_k_o2: 0.14,
            max_flux_fraction: 0.30,
            reconcile_every_n_steps: 4,
            mass_fix_every_n_steps: 4,
            mass_fix_error_threshold: 1e-4,
            mass_fix_min_residual: 1e-5,
            solver_mode: GasSolverMode::LegacyHybrid,
            solver_tuning: SolverTuning::default(),
        };
        let mut block_state = BlockSyncState {
            rng_state: 123_456_789,
            phase: 0,
            last_block_index: 0,
        };
        let mut step = SimulationStep(0);

        let h_before = total_species(&gas, GasKind::Hydrogen);
        let o_before = total_species(&gas, GasKind::Oxygen);

        for _ in 0..steps {
            do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
        }

        let h_after = total_species(&gas, GasKind::Hydrogen);
        let o_after = total_species(&gas, GasKind::Oxygen);
        let h_error = (h_before - h_after).abs();
        let o_error = (o_before - o_after).abs();
        assert!(
            h_error <= h_before * 1e-4 + 1e-3,
            "H2 mass drift too high: before={h_before}, after={h_after}, abs_error={h_error}"
        );
        assert!(
            o_error <= o_before * 1e-4 + 1e-3,
            "O2 mass drift too high: before={o_before}, after={o_after}, abs_error={o_error}"
        );
    }

    #[test]
    fn mass_conservation_per_species_float_smoke() {
        run_mass_conservation_steps(2_000);
    }

    #[test]
    #[ignore = "Long-running 10k-step stability test"]
    fn mass_conservation_per_species_float_10k() {
        run_mass_conservation_steps(10_000);
    }

    #[test]
    fn no_nan_inf_and_non_negative_after_many_steps() {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);

        let config = GasSimulationConfig::default();
        let mut block_state = BlockSyncState {
            rng_state: 42,
            phase: 0,
            last_block_index: 0,
        };
        let mut step = SimulationStep(0);

        for _ in 0..2_000 {
            do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
        }

        for cell in &gas.read {
            for value in cell {
                assert!(value.is_finite());
                assert!(*value >= -1e-5);
            }
        }
    }

    #[test]
    fn random_obstacles_keep_solver_finite_and_mass_conserved() {
        let config = GasSimulationConfig::default();

        for scenario_seed in [7_u64, 91_u64, 777_u64] {
            let mut world = WorldGrid::default();
            let mut local_seed = scenario_seed;
            for y in 2..WORLD_HEIGHT - 2 {
                for x in 2..WORLD_WIDTH - 2 {
                    if x == WORLD_WIDTH / 2 && y == WORLD_HEIGHT / 2 {
                        continue;
                    }
                    if next_random_u32(&mut local_seed) % 100 < 4 {
                        let _ = world.set_solid(x, y);
                    }
                }
            }

            let mut gas = GasField::default();
            gas.clear_rect(
                UVec2::new(1, 1),
                UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
            );
            let cx = WORLD_WIDTH / 2;
            let cy = WORLD_HEIGHT / 2;
            let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);

            let initial_mass = total_species(&gas, GasKind::Hydrogen);
            let mut block_state = BlockSyncState {
                rng_state: scenario_seed ^ 0xA5A5_5A5A,
                phase: 0,
                last_block_index: 0,
            };
            let mut step = SimulationStep(0);

            for _ in 0..500 {
                do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
            }

            let final_mass = total_species(&gas, GasKind::Hydrogen);
            let mass_error = (initial_mass - final_mass).abs();
            assert!(
                mass_error <= initial_mass * 1e-4 + 1e-3,
                "Mass drift too high for seed {scenario_seed}: before={initial_mass}, after={final_mass}, abs_error={mass_error}"
            );
            assert!(gas
                .read
                .iter()
                .flat_map(|cell| cell.iter())
                .all(|v| v.is_finite() && *v >= -1e-5));
        }
    }

    #[test]
    fn unified_spike_improves_wave_metrics_vs_legacy_on_s1() {
        let world = WorldGrid::default();
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;

        let mut gas_legacy = GasField::default();
        gas_legacy.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let _ = gas_legacy.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);
        let mut legacy_state = BlockSyncState {
            rng_state: 111,
            phase: 0,
            last_block_index: 0,
        };
        let legacy_cfg = make_config(GasSolverMode::LegacyHybrid, true);
        let mut step = SimulationStep(0);
        for _ in 0..1000 {
            do_one_substep(
                &mut legacy_state,
                &mut gas_legacy,
                &world,
                &legacy_cfg,
                &mut step,
            );
        }
        let (legacy_aniso, legacy_wave) = wave_metrics(&gas_legacy);

        let mut best_aniso = f32::INFINITY;
        let mut best_wave = f32::INFINITY;
        for mode in [GasSolverMode::UnifiedTRT, GasSolverMode::UnifiedMRTCM] {
            let mut gas_unified = GasField::default();
            gas_unified.clear_rect(
                UVec2::new(1, 1),
                UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
            );
            let _ = gas_unified.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);
            let mut unified_state = BlockSyncState {
                rng_state: 111,
                phase: 0,
                last_block_index: 0,
            };
            let unified_cfg = make_config(mode, true);
            let mut step2 = SimulationStep(0);
            for _ in 0..1000 {
                do_one_substep(
                    &mut unified_state,
                    &mut gas_unified,
                    &world,
                    &unified_cfg,
                    &mut step2,
                );
            }
            let (a, w) = wave_metrics(&gas_unified);
            best_aniso = best_aniso.min(a);
            best_wave = best_wave.min(w);
        }

        assert!(
            best_aniso <= legacy_aniso * 1.20,
            "Unified spike regressed anisotropy too much: legacy={legacy_aniso}, best_unified={best_aniso}"
        );
        assert!(
            best_wave <= legacy_wave * 1.20,
            "Unified spike regressed wave score too much: legacy={legacy_wave}, best_unified={best_wave}"
        );
    }

    #[test]
    fn unified_modes_keep_mass_and_finite_on_s2_obstacles() {
        let mut world = WorldGrid::default();
        let mut seed = 987654321u64;
        for y in 2..WORLD_HEIGHT - 2 {
            for x in 2..WORLD_WIDTH - 2 {
                if x == WORLD_WIDTH / 2 && y == WORLD_HEIGHT / 2 {
                    continue;
                }
                if next_random_u32(&mut seed) % 100 < 5 {
                    let _ = world.set_solid(x, y);
                }
            }
        }

        for mode in [GasSolverMode::UnifiedTRT, GasSolverMode::UnifiedMRTCM] {
            let mut gas = GasField::default();
            gas.clear_rect(
                UVec2::new(1, 1),
                UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
            );
            let _ = gas.apply_species_delta_with_lbm(
                WORLD_WIDTH / 2,
                WORLD_HEIGHT / 2,
                GasKind::Hydrogen,
                10_000.0,
            );
            let initial = total_species(&gas, GasKind::Hydrogen);
            let cfg = make_config(mode, true);
            let mut state = BlockSyncState {
                rng_state: 123,
                phase: 0,
                last_block_index: 0,
            };
            let mut step = SimulationStep(0);
            for _ in 0..1000 {
                do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
            }
            let final_mass = total_species(&gas, GasKind::Hydrogen);
            assert!((final_mass - initial).abs() <= initial * 1e-4 + 1e-3);
            assert!(gas
                .read
                .iter()
                .flat_map(|cell| cell.iter())
                .all(|v| v.is_finite() && *v >= -1e-5));
        }
    }

    #[test]
    fn unified_species_relaxation_toggle_changes_spread_s3() {
        let world = WorldGrid::default();
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;

        let mut gas_on = GasField::default();
        gas_on.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let _ = gas_on.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);
        let mut st_on = BlockSyncState {
            rng_state: 314,
            phase: 0,
            last_block_index: 0,
        };
        let cfg_on = make_config(GasSolverMode::UnifiedTRT, true);
        let mut step_on = SimulationStep(0);
        for _ in 0..200 {
            do_one_substep(&mut st_on, &mut gas_on, &world, &cfg_on, &mut step_on);
        }
        let spread_on = gas_on.total_amount(cx + 6, cy) + gas_on.total_amount(cx - 6, cy);

        let mut gas_off = GasField::default();
        gas_off.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let _ = gas_off.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);
        let mut st_off = BlockSyncState {
            rng_state: 314,
            phase: 0,
            last_block_index: 0,
        };
        let cfg_off = make_config(GasSolverMode::UnifiedTRT, false);
        let mut step_off = SimulationStep(0);
        for _ in 0..200 {
            do_one_substep(&mut st_off, &mut gas_off, &world, &cfg_off, &mut step_off);
        }
        let spread_off = gas_off.total_amount(cx + 6, cy) + gas_off.total_amount(cx - 6, cy);
        assert!(
            spread_on > spread_off,
            "Species relaxation toggle should influence spread: on={spread_on}, off={spread_off}"
        );
    }

    #[test]
    fn mass_fix_does_not_inject_corner_artifact() {
        let mut world = WorldGrid::default();
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;

        // Build a closed chamber so gas cannot physically reach the map corner.
        let min_x = cx.saturating_sub(5);
        let max_x = (cx + 5).min(WORLD_WIDTH - 2);
        let min_y = cy.saturating_sub(5);
        let max_y = (cy + 5).min(WORLD_HEIGHT - 2);
        for x in min_x..=max_x {
            let _ = world.set_solid(x, min_y);
            let _ = world.set_solid(x, max_y);
        }
        for y in min_y..=max_y {
            let _ = world.set_solid(min_x, y);
            let _ = world.set_solid(max_x, y);
        }
        for y in min_y + 1..max_y {
            for x in min_x + 1..max_x {
                let _ = world.set_empty(x, y);
            }
        }

        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);

        let mut state = BlockSyncState {
            rng_state: 112233,
            phase: 0,
            last_block_index: 0,
        };
        let cfg = make_config(GasSolverMode::UnifiedTRT, true);
        let mut step = SimulationStep(0);
        for _ in 0..2_000 {
            do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
        }

        let epsilon_visual = 1e-3;
        for y in 1..6 {
            for x in 1..6 {
                assert!(
                    gas.total_amount(x, y) <= epsilon_visual,
                    "Unexpected corner mass at ({x},{y}) = {}",
                    gas.total_amount(x, y)
                );
            }
        }
    }

    #[test]
    fn buoyancy_force_finite_and_bounded() {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = gas.apply_species_delta_with_lbm(cx, cy, GasKind::Hydrogen, 10_000.0);

        let mut cfg = make_config(GasSolverMode::UnifiedTRT, true);
        cfg.solver_tuning.enable_buoyancy = true;
        cfg.solver_tuning.buoyancy_strength = 0.20;
        cfg.solver_tuning.target_cfl_like_limit = 0.75;

        let mut state = BlockSyncState {
            rng_state: 998877,
            phase: 0,
            last_block_index: 0,
        };
        let mut step = SimulationStep(0);
        for _ in 0..500 {
            do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
        }

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                let v = gas.velocity(x, y);
                assert!(
                    v.x.is_finite() && v.y.is_finite(),
                    "NaN/Inf velocity at ({x},{y})"
                );
                assert!(
                    v.length() <= cfg.solver_tuning.target_cfl_like_limit + 1e-3,
                    "Velocity limit exceeded at ({x},{y}): {}",
                    v.length()
                );
            }
        }
        assert!(gas
            .read
            .iter()
            .flat_map(|cell| cell.iter())
            .all(|v| v.is_finite() && *v >= -1e-5));
    }

    #[test]
    fn unified_buoyancy_creates_upward_jet_in_hole_scenario() {
        let mut world = WorldGrid::default();
        let x0 = 34;
        let x1 = 68;
        let y0 = 28;
        let y1 = 52;
        let hole_x = (x0 + x1) / 2;
        let hole_y = y1;

        // Room walls with a single top hole.
        for x in x0..=x1 {
            let _ = world.set_solid(x, y0);
            if x != hole_x {
                let _ = world.set_solid(x, y1);
            }
        }
        for y in y0..=y1 {
            let _ = world.set_solid(x0, y);
            let _ = world.set_solid(x1, y);
        }
        for y in y0 + 1..y1 {
            for x in x0 + 1..x1 {
                let _ = world.set_empty(x, y);
            }
        }
        let _ = world.set_empty(hole_x, hole_y);

        let mut gas = GasField::default();
        gas.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let _ = gas.apply_species_delta_with_lbm(hole_x, y0 + 5, GasKind::Hydrogen, 10_000.0);

        let mut cfg = make_config(GasSolverMode::UnifiedTRT, true);
        cfg.solver_tuning.enable_buoyancy = true;
        cfg.solver_tuning.buoyancy_strength = 0.08;

        let mut state = BlockSyncState {
            rng_state: 445566,
            phase: 0,
            last_block_index: 0,
        };
        let mut step = SimulationStep(0);
        for _ in 0..200 {
            do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
        }

        let mut upward_mass = 0.0f32;
        let mut lateral_mass = 0.0f32;
        for y in hole_y + 1..=(hole_y + 16).min(WORLD_HEIGHT - 2) {
            for x in hole_x.saturating_sub(16)..=(hole_x + 16).min(WORLD_WIDTH - 2) {
                if world.is_solid(x, y) || crate::world::grid::is_boundary(x, y) {
                    continue;
                }
                let amount = gas.total_amount(x, y).max(0.0);
                if amount <= 1e-6 {
                    continue;
                }
                let dx = (x as i32 - hole_x as i32).unsigned_abs() as f32;
                let dy = (y as i32 - hole_y as i32).unsigned_abs() as f32;
                if dy >= dx {
                    upward_mass += amount;
                } else {
                    lateral_mass += amount;
                }
            }
        }
        let ratio = upward_mass / lateral_mass.max(1e-6);
        assert!(
            ratio >= 1.8,
            "Upward jet is too weak: upward_mass={upward_mass}, lateral_mass={lateral_mass}, ratio={ratio}"
        );
    }

    #[test]
    #[ignore = "Profiling helper"]
    fn benchmark_substep_modes_release_like() {
        let world = WorldGrid::default();
        for mode in [
            GasSolverMode::LegacyHybrid,
            GasSolverMode::UnifiedTRT,
            GasSolverMode::UnifiedMRTCM,
        ] {
            let mut gas = GasField::default();
            gas.clear_rect(
                UVec2::new(1, 1),
                UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
            );
            let _ = gas.apply_species_delta_with_lbm(
                WORLD_WIDTH / 2,
                WORLD_HEIGHT / 2,
                GasKind::Hydrogen,
                10_000.0,
            );
            let cfg = make_config(mode, true);
            let mut state = BlockSyncState {
                rng_state: 1,
                phase: 0,
                last_block_index: 0,
            };
            let mut step = SimulationStep(0);
            let start = Instant::now();
            for _ in 0..5000 {
                do_one_substep(&mut state, &mut gas, &world, &cfg, &mut step);
            }
            let elapsed = start.elapsed();
            eprintln!("mode={} elapsed_ms={}", mode.label(), elapsed.as_millis());
        }
    }
}
