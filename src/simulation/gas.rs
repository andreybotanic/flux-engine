use super::discrete_step::{step_discrete_in_place, DiscreteStepParams};
use super::gpu_solver::GpuSolverHostState;
use super::SolverTuning;
use crate::config::GasRegistry;
use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};
use bevy::prelude::*;
#[cfg(test)]
use std::sync::OnceLock;

pub const HYDROGEN_GPU_STORAGE_MAX_PARTICLES: u32 = 20_000;

const EPSILON: f32 = 1e-6;
#[cfg(test)]
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;

pub type GasScalar = f32;
pub type GasCell = Vec<u32>;

#[derive(Clone, Debug)]
/// Stores `GasFieldSnapshot` state.
pub struct GasFieldSnapshot {
    pub gas_count: usize,
    pub species: Vec<u32>,
    pub total_density: Vec<f32>,
    pub velocity: Vec<[f32; 2]>,
}

#[derive(Clone, Copy)]
#[cfg(test)]
struct KernelOffset {
    dx: i32,
    dy: i32,
    dist2: f32,
}

#[derive(Resource, Clone)]
/// Stores `GasField` state.
pub struct GasField {
    pub read: Vec<GasCell>,
    write: Vec<GasCell>,
    total_density: Vec<f32>,
    velocity: Vec<Vec2>,
    gas_count: usize,
    molecular_masses: Vec<f32>,
}

impl Default for GasField {
    fn default() -> Self {
        let registry = GasRegistry::from_substances(
            crate::plugins::default_plugin::default_substance_definitions(),
        )
        .expect("default plugin substances must stay valid");
        Self::from_registry(&registry)
    }
}

include!("gas_core_block.rs");
include!("gas_test_support_block.rs");
include!("gas_tests_block.rs");
