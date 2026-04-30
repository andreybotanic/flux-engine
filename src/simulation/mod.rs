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
pub struct GasSimulationConfig {
    pub enable_lbm_velocity: bool,
    pub enable_diffusion: bool,
    pub lbm_tau: f32,
    pub diffusion_k_h2: f32,
    pub diffusion_k_o2: f32,
    pub max_flux_fraction: f32,
    pub reconcile_every_n_steps: u32,
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
            reconcile_every_n_steps: 1,
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

    gas.renormalize_species_mass(world, target_species_totals);
    gas.recompute_total_density_buffer(world);
    if config.enable_lbm_velocity
        && config.reconcile_every_n_steps > 0
        && step.0 % u64::from(config.reconcile_every_n_steps) == 0
    {
        gas.reconcile_lbm_from_species(world);
    }

    block_state.last_block_index = block_index;
    block_state.phase = (block_state.phase + 1) % 9;
    step.0 += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::gas::GasKind;
    use crate::world::grid::{linear_index, WORLD_HEIGHT, WORLD_WIDTH};

    fn total_species(gas: &GasField, kind: GasKind) -> f32 {
        gas.read.iter().map(|cell| cell[kind.index()]).sum()
    }

    #[test]
    fn with_diffusion_disabled_center_seed_spreads_to_neighbours() {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));

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
            reconcile_every_n_steps: 1,
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

        let total: f32 = gas.read.iter().map(|cell| cell[GasKind::Hydrogen.index()]).sum();
        assert!((total - 360.0).abs() < 1e-3);

        let center_idx = linear_index(cx, cy);
        let center_total_after = gas.read[center_idx][GasKind::Hydrogen.index()];
        assert!(center_total_after < 360.0);
    }

    fn run_mass_conservation_steps(steps: u32) {
        let world = WorldGrid::default();
        let mut gas = GasField::default();
        gas.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
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
            reconcile_every_n_steps: 1,
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
        gas.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
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
            gas.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
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
}
