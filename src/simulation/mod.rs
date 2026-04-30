pub mod gas;
pub mod gpu;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

use self::{
    gas::{next_random_u32, phase_offsets, step_cpu_gas_block_sync, GasField},
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
            .init_resource::<BlockSyncState>()
            .add_plugins(GasGpuPlugin)
            .add_systems(FixedUpdate, run_simulation_tick);
    }
}

fn run_simulation_tick(
    control: Res<SimulationControl>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
) {
    if control.paused {
        return;
    }

    for _ in 0..control.speed.multiplier() {
        do_one_substep(&mut block_state, &mut gas, &world, &mut step);
    }
}

pub fn do_one_substep(
    block_state: &mut BlockSyncState,
    gas: &mut GasField,
    world: &WorldGrid,
    step: &mut SimulationStep,
) {
    let block_index = (next_random_u32(&mut block_state.rng_state) % 9) as u8;
    let (offset_x, offset_y) = phase_offsets(block_state.phase);

    step_cpu_gas_block_sync(
        gas,
        world,
        block_index,
        offset_x,
        offset_y,
        &mut block_state.rng_state,
    );

    block_state.last_block_index = block_index;
    block_state.phase = (block_state.phase + 1) % 9;
    step.0 += 1;
}
