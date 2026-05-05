use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    render::GasVisualSettings,
    simulation::{GasSimulationConfig, SimulationRateConfig, SolverTuning},
    world::grid::CellMaterial,
};

#[derive(Resource, Clone, Debug)]
/// Stores `GasRegistry` state.
pub struct GasRegistry {
    gases: Vec<GasDefinition>,
    by_id: HashMap<String, usize>,
}

impl GasRegistry {
/// Runs `new` logic.
    pub fn new(mut gases: Vec<GasDefinition>) -> Result<Self, String> {
        if gases.is_empty() {
            return Err("Gas registry is empty. Add at least one gas config file.".to_string());
        }

        gases.sort_by(|a, b| {
            a.molecular_mass
                .total_cmp(&b.molecular_mass)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut by_id = HashMap::new();
        for (idx, gas) in gases.iter().enumerate() {
            if by_id.insert(gas.id.clone(), idx).is_some() {
                return Err(format!("Duplicate gas id '{}'", gas.id));
            }
        }

        Ok(Self { gases, by_id })
    }

/// Runs `all` logic.
    pub fn all(&self) -> &[GasDefinition] {
        &self.gases
    }

/// Runs `count` logic.
    pub fn count(&self) -> usize {
        self.gases.len()
    }

/// Runs `get` logic.
    pub fn get(&self, index: usize) -> Option<&GasDefinition> {
        self.gases.get(index)
    }

/// Runs `index_of` logic.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

/// Runs `molecular_masses` logic.
    pub fn molecular_masses(&self) -> Vec<f32> {
        self.gases.iter().map(|gas| gas.molecular_mass).collect()
    }
}

#[derive(Clone, Debug)]
/// Stores `GasDefinition` state.
pub struct GasDefinition {
    pub id: String,
    pub label: String,
    pub molecular_mass: f32,
    pub color: [f32; 3],
}

impl GasDefinition {
/// Runs `color_as_bevy` logic.
    pub fn color_as_bevy(&self) -> Color {
        Color::srgb(self.color[0], self.color[1], self.color[2])
    }
}

#[derive(Resource, Clone, Copy, Debug)]
/// Stores `GasMainViewVisualConfig` state.
pub struct GasMainViewVisualConfig {
    pub min_particles: f32,
    pub max_particles_for_max_intensity: f32,
    pub min_intensity: f32,
    pub alpha: f32,
}

impl Default for GasMainViewVisualConfig {
    fn default() -> Self {
        Self {
            min_particles: 1.0,
            max_particles_for_max_intensity: 1000.0,
            min_intensity: 0.05,
            alpha: 0.88,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
/// Stores `CellTypeVisualConfig` state.
pub struct CellTypeVisualConfig {
    pub boundary_main_tint: [f32; 3],
    pub boundary_gas_tint: [f32; 3],
    pub brick_main_tint: [f32; 3],
    pub brick_gas_tint: [f32; 3],
    pub metal_main_tint: [f32; 3],
    pub metal_gas_tint: [f32; 3],
}

impl Default for CellTypeVisualConfig {
    fn default() -> Self {
        Self {
            boundary_main_tint: [0.90, 0.90, 0.91],
            boundary_gas_tint: [0.66, 0.66, 0.67],
            brick_main_tint: [0.99, 0.99, 0.99],
            brick_gas_tint: [0.72, 0.72, 0.74],
            metal_main_tint: [0.99, 0.99, 0.99],
            metal_gas_tint: [0.71, 0.71, 0.73],
        }
    }
}

impl CellTypeVisualConfig {
/// Runs `main_tint` logic.
    pub fn main_tint(self, material: CellMaterial) -> Color {
        let rgb = match material {
            CellMaterial::Boundary => self.boundary_main_tint,
            CellMaterial::Brick => self.brick_main_tint,
            CellMaterial::Metal => self.metal_main_tint,
        };
        Color::srgb(rgb[0], rgb[1], rgb[2])
    }

/// Runs `gas_tint` logic.
    pub fn gas_tint(self, material: CellMaterial) -> Color {
        let rgb = match material {
            CellMaterial::Boundary => self.boundary_gas_tint,
            CellMaterial::Brick => self.brick_gas_tint,
            CellMaterial::Metal => self.metal_gas_tint,
        };
        Color::srgb(rgb[0], rgb[1], rgb[2])
    }
}

#[derive(Clone)]
/// Stores `GameConfig` state.
pub struct GameConfig {
    pub gas_registry: GasRegistry,
    pub simulation_rate: SimulationRateConfig,
    pub gas_simulation: GasSimulationConfig,
    pub gas_visual: GasVisualSettings,
    pub gas_main_visual: GasMainViewVisualConfig,
    pub cell_visuals: CellTypeVisualConfig,
}

impl GameConfig {
/// Runs `load_from_default_location` logic.
    pub fn load_from_default_location() -> Result<Self, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("config");
        Self::load_from_root(&root)
    }

/// Runs `load_from_root` logic.
    pub fn load_from_root(root: &Path) -> Result<Self, String> {
        let simulation = read_toml::<SimulationToml>(&root.join("simulation.toml"))?;
        let cell_types = read_toml::<CellTypesToml>(&root.join("cell_types.toml"))?;
        let gases = load_gas_files(&root.join("gases"))?;

        let gas_registry = GasRegistry::new(gases)?;

        let simulation_rate = SimulationRateConfig {
            target_hz: simulation.rate.target_hz,
        };

        let gas_simulation = GasSimulationConfig {
            enable_lbm_velocity: true,
            enable_species_relaxation: true,
            reconcile_every_n_steps: 0,
            mass_fix_every_n_steps: 0,
            mass_fix_error_threshold: 0.0,
            mass_fix_min_residual: 0.0,
            thermal_motion_scale: simulation.simulation.thermal_motion_scale,
            solver_tuning: SolverTuning {
                tau_even: 0.85,
                tau_odd: 1.15,
                target_cfl_like_limit: 0.85,
                velocity_damping: 0.08,
                species_eq_blend: 0.28,
                enable_buoyancy: simulation.solver_tuning.enable_buoyancy,
                buoyancy_strength: simulation.solver_tuning.buoyancy_strength,
                buoyancy_window_radius: simulation.solver_tuning.buoyancy_window_radius,
                buoyancy_window_sigma: simulation.solver_tuning.buoyancy_window_sigma,
                buoyancy_gain: simulation.solver_tuning.buoyancy_gain,
                buoyancy_alpha: simulation.solver_tuning.buoyancy_alpha,
                buoyancy_force_cap: simulation.solver_tuning.buoyancy_force_cap,
            },
        };

        let gas_visual = GasVisualSettings {
            gamma: simulation.visual.gamma,
            max_particles_for_max_color: simulation.visual.max_particles_for_max_color,
        };

        let gas_main_visual = GasMainViewVisualConfig {
            min_particles: simulation.visual.f1_min_particles,
            max_particles_for_max_intensity: simulation.visual.f1_max_particles_for_max_intensity,
            min_intensity: simulation.visual.f1_min_intensity,
            alpha: simulation.visual.f1_alpha,
        };

        let cell_visuals = CellTypeVisualConfig {
            boundary_main_tint: cell_types.boundary.main_tint,
            boundary_gas_tint: cell_types.boundary.gas_tint,
            brick_main_tint: cell_types.brick.main_tint,
            brick_gas_tint: cell_types.brick.gas_tint,
            metal_main_tint: cell_types.metal.main_tint,
            metal_gas_tint: cell_types.metal.gas_tint,
        };

        Ok(Self {
            gas_registry,
            simulation_rate,
            gas_simulation,
            gas_visual,
            gas_main_visual,
            cell_visuals,
        })
    }
}


include!("config_loader_block.rs");
include!("config_tests_block.rs");
