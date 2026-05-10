use crate::{
    config::GasRegistry,
    plugins::default_plugin::{
        default_substance_definitions, pipe_runtime::apply_gas_structures_pre_step,
    },
    simulation::{
        gas::GasField, gpu_solver::GpuGasSolver, GasSimulationConfig, SimulationStep, SolverTuning,
    },
    world::{
        grid::{is_boundary, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::PlacedStructureMap,
    },
};
use bevy::prelude::UVec2;

#[derive(Clone, Copy, Debug)]
/// Stores `ParityThresholds` state.
pub struct ParityThresholds {
    pub radius: u32,
    pub mean_abs_error_max: f32,
    pub p95_abs_error_max: f32,
    pub max_abs_error_max: f32,
    pub mass_rel_error_max: f32,
}

#[derive(Clone, Debug)]
/// Stores `ParityMetrics` state.
pub struct ParityMetrics {
    pub compared_cells: usize,
    pub mean_abs_error: f32,
    pub p95_abs_error: f32,
    pub max_abs_error: f32,
    pub mass_rel_errors: Vec<f32>,
}

#[derive(Clone, Copy, Debug)]
/// Stores `ScenarioSpec` state.
pub struct ScenarioSpec {
    pub name: &'static str,
    pub with_internal_walls: bool,
    pub with_structures: bool,
}

pub const PARITY_STEPS: u32 = 5_000;
pub const PARITY_THRESHOLDS: ParityThresholds = ParityThresholds {
    radius: 3,
    mean_abs_error_max: 150.0,
    p95_abs_error_max: 700.0,
    max_abs_error_max: 2_500.0,
    mass_rel_error_max: 0.50,
};

pub const PARITY_SCENARIOS: [ScenarioSpec; 3] = [
    ScenarioSpec {
        name: "open-field",
        with_internal_walls: false,
        with_structures: false,
    },
    ScenarioSpec {
        name: "inner-walls",
        with_internal_walls: true,
        with_structures: false,
    },
    ScenarioSpec {
        name: "inner-walls-with-structures",
        with_internal_walls: true,
        with_structures: true,
    },
];

fn test_registry_three_gases() -> GasRegistry {
    GasRegistry::from_substances(default_substance_definitions()).expect("test gas registry")
}

fn tuned_config() -> GasSimulationConfig {
    GasSimulationConfig {
        thermal_motion_scale: 0.08,
        solver_tuning: SolverTuning::default(),
        ..GasSimulationConfig::default()
    }
}

fn populate_scenario(
    world: &mut WorldGrid,
    structures: &mut PlacedStructureMap,
    gas: &mut GasField,
    with_internal_walls: bool,
    with_structures: bool,
) {
    gas.clear_rect(
        UVec2::new(1, 1),
        UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
    );
    *structures = PlacedStructureMap::default();
    if with_internal_walls {
        for x in 20..=80 {
            let _ = world.set_solid_with_material(x, 20, CellMaterial::Brick);
            let _ = world.set_solid_with_material(x, 80, CellMaterial::Brick);
        }
        for y in 20..=80 {
            let _ = world.set_solid_with_material(20, y, CellMaterial::Brick);
            let _ = world.set_solid_with_material(80, y, CellMaterial::Brick);
        }
        for x in 48..=53 {
            let _ = world.set_empty(x, 20);
        }
        for y in 34..=71 {
            let _ = world.set_solid_with_material(50, y, CellMaterial::Metal);
        }
        let _ = world.set_empty(50, 52);
        let _ = world.set_empty(50, 53);
    }

    if with_structures {
        let _ = structures.place_gas_source(28, 28, 0, 8, world);
        let _ = structures.place_gas_source(74, 74, 2, 11, world);
        let _ = structures.place_gas_sink(28, 74, 7, world);
        let _ = structures.place_gas_sink(74, 28, 9, world);
    }

    for y in 1..WORLD_HEIGHT - 1 {
        for x in 1..WORLD_WIDTH - 1 {
            if is_boundary(x, y) || world.is_solid(x, y) {
                continue;
            }
            let a0 = ((x * 31 + y * 17) % 43) as f32;
            let a1 = ((x * 7 + y * 19) % 29) as f32;
            let a2 = ((x * 23 + y * 13) % 37) as f32;
            if (x + y) % 2 == 0 {
                gas.set_amount(x, y, 0, a0 + 10.0);
            }
            if (x * 2 + y) % 3 == 0 {
                gas.set_amount(x, y, 1, a1 + 6.0);
            }
            if (x + y * 2) % 5 == 0 {
                gas.set_amount(x, y, 2, a2 + 4.0);
            }
        }
    }
    gas.recompute_total_density_buffer(world);
}

fn percentile(values: &[f32], pct: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() - 1) as f32 * pct).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn local_average(
    field: &GasField,
    world: &WorldGrid,
    x: u32,
    y: u32,
    gas_index: usize,
    radius: u32,
) -> Option<f32> {
    let mut sum = 0.0f32;
    let mut n = 0u32;
    let r2 = i32::try_from(radius.saturating_mul(radius)).unwrap_or(i32::MAX);
    for dy in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
            if dx * dx + dy * dy > r2 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx <= 0
                || ny <= 0
                || nx >= (WORLD_WIDTH - 1) as i32
                || ny >= (WORLD_HEIGHT - 1) as i32
            {
                continue;
            }
            let nx = nx as u32;
            let ny = ny as u32;
            if world.is_solid(nx, ny) {
                continue;
            }
            sum += field.amount(nx, ny, gas_index);
            n += 1;
        }
    }
    if n == 0 {
        None
    } else {
        Some(sum / n as f32)
    }
}

include!("parity_runtime_block.rs");
include!("parity_tests_block.rs");
