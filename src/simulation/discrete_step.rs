use bevy::prelude::*;
use std::sync::OnceLock;

use super::SolverTuning;

const EPSILON: f32 = 1e-6;
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;
const DIRS_VON_NEUMANN: [IVec2; 4] = [
    IVec2::new(0, 1),
    IVec2::new(0, -1),
    IVec2::new(-1, 0),
    IVec2::new(1, 0),
];

#[derive(Clone, Copy)]
struct KernelOffset {
    dx: i32,
    dy: i32,
    dist2: f32,
}

#[derive(Clone, Copy)]
struct Rng64 {
    state: u32,
}

impl Rng64 {
    fn seeded(seed: u32) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9,
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / 4_294_967_296.0
    }
}

pub trait SolidQuery {
    fn is_solid(&self, x: u32, y: u32) -> bool;
}

impl<F> SolidQuery for F
where
    F: Fn(u32, u32) -> bool,
{
    fn is_solid(&self, x: u32, y: u32) -> bool {
        self(x, y)
    }
}

/// Stores `DiscreteStepParams` state.
pub struct DiscreteStepParams<'a, S, T>
where
    S: SolidQuery,
    T: Fn(u32, u32) -> f32,
{
    pub width: u32,
    pub height: u32,
    pub gas_count: usize,
    pub molecular_masses: &'a [f32],
    pub read: &'a mut Vec<u32>,
    pub write: &'a mut Vec<u32>,
    pub total_density: &'a mut Vec<f32>,
    pub velocity: &'a mut Vec<Vec2>,
    pub solid_query: &'a S,
    pub temperature_multiplier: T,
    pub tuning: &'a SolverTuning,
    pub thermal_motion_scale: f32,
    pub simulation_step: u64,
}

include!("discrete_step_step_block.rs");
include!("discrete_step_helpers_block.rs");
