use bevy::prelude::*;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SimulationBackend {
    Cpu,
    #[default]
    Gpu,
}

#[derive(Resource, Clone, Copy, Debug)]
/// Stores `SimulationBackendConfig` state.
pub struct SimulationBackendConfig {
    pub backend: SimulationBackend,
}

impl Default for SimulationBackendConfig {
    fn default() -> Self {
        Self {
            backend: SimulationBackend::Gpu,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
/// Stores `WorldSizeConfig` state.
pub struct WorldSizeConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for WorldSizeConfig {
    fn default() -> Self {
        Self {
            width: crate::world::grid::WORLD_WIDTH,
            height: crate::world::grid::WORLD_HEIGHT,
        }
    }
}
