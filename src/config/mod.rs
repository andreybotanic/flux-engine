mod hud;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    plugins::{
        default_plugin::{default_substance_id_for_alias, pipe_runtime::PipeSimulationConfig},
        ContentRegistry, PluginId, SubstanceDefinition, SubstanceId, SubstanceRegistry,
    },
    render::GasVisualSettings,
    simulation::{GasSimulationConfig, SimulationRateConfig, SolverTuning},
    world::{grid::CellMaterial, structures::StructureKind},
};

pub use self::hud::{
    ConfiguredPipeNodeKind, ContainerBacking, HoverVisibility, HudBlockConfig,
    StructureHudConfigMap, SubstanceContainerConfig, SubstanceKind, WorldCellHudConfig,
};

#[derive(Resource, Clone, Debug)]
/// Stores `GasRegistry` state.
pub struct GasRegistry {
    substances: SubstanceRegistry,
    gases: Vec<GasDefinition>,
    stable_ids: Vec<SubstanceId>,
    by_id: HashMap<String, usize>,
}

impl GasRegistry {
    /// Builds a compatibility registry from legacy gas definitions.
    pub fn new(gases: Vec<GasDefinition>) -> Result<Self, String> {
        let mut substances = Vec::with_capacity(gases.len());
        for gas in gases {
            let substance_id = default_substance_id_for_alias(&gas.id)?;
            substances.push(SubstanceDefinition::gas(
                substance_id,
                crate::plugins::PluginId::default_plugin(),
                gas.label,
                gas.molecular_mass,
                gas.color,
                vec![gas.id],
            )?);
        }
        Self::from_substances(substances)
    }

    /// Builds a gas registry from plugin-owned substance definitions.
    pub fn from_substances(substances: Vec<SubstanceDefinition>) -> Result<Self, String> {
        let substances = SubstanceRegistry::new(substances)?;
        Self::from_substance_registry(substances)
    }

    /// Builds a gas registry from an already validated substance registry.
    pub fn from_substance_registry(substances: SubstanceRegistry) -> Result<Self, String> {
        let gas_substances = substances
            .all()
            .iter()
            .filter(|definition| definition.flags.gas)
            .collect::<Vec<_>>();
        let gases = gas_substances
            .iter()
            .map(|definition| GasDefinition {
                id: definition
                    .aliases
                    .first()
                    .cloned()
                    .unwrap_or_else(|| definition.id.leaf().to_string()),
                label: definition.label.clone(),
                molecular_mass: definition.molecular_mass,
                color: definition.color,
            })
            .collect::<Vec<_>>();

        if gases.is_empty() {
            return Err("Gas registry is empty. Register at least one gas substance.".to_string());
        }

        let stable_ids = gas_substances
            .iter()
            .map(|definition| definition.id.clone())
            .collect::<Vec<_>>();
        let mut by_id = HashMap::new();
        for (idx, definition) in gas_substances.into_iter().enumerate() {
            by_id.insert(definition.id.as_str().to_string(), idx);
            for alias in &definition.aliases {
                by_id.insert(alias.clone(), idx);
            }
        }

        Ok(Self {
            substances,
            gases,
            stable_ids,
            by_id,
        })
    }

    /// Returns all gas definitions in compact runtime order.
    pub fn all(&self) -> &[GasDefinition] {
        &self.gases
    }

    /// Returns the number of gas-capable substances.
    pub fn count(&self) -> usize {
        self.gases.len()
    }

    /// Returns one gas definition by compact runtime index.
    pub fn get(&self, index: usize) -> Option<&GasDefinition> {
        self.gases.get(index)
    }

    /// Returns the compact runtime index for a stable substance id or legacy alias.
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// Returns the stable substance id for one compact runtime index.
    pub fn stable_id_by_index(&self, index: usize) -> Option<&SubstanceId> {
        self.stable_ids.get(index)
    }

    /// Returns stable substance ids in compact runtime order.
    pub fn stable_ids(&self) -> Vec<String> {
        (0..self.count())
            .filter_map(|index| self.stable_id_by_index(index))
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Returns the underlying plugin-owned substance registry.
    pub fn substances(&self) -> &SubstanceRegistry {
        &self.substances
    }

    /// Returns molecular masses in compact runtime order.
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
        let rgb = if material == crate::plugins::default_plugin::boundary_cell_material() {
            self.boundary_main_tint
        } else if material == crate::plugins::default_plugin::metal_cell_material() {
            self.metal_main_tint
        } else {
            self.brick_main_tint
        };
        Color::srgb(rgb[0], rgb[1], rgb[2])
    }

    /// Runs `gas_tint` logic.
    pub fn gas_tint(self, material: CellMaterial) -> Color {
        let rgb = if material == crate::plugins::default_plugin::boundary_cell_material() {
            self.boundary_gas_tint
        } else if material == crate::plugins::default_plugin::metal_cell_material() {
            self.metal_gas_tint
        } else {
            self.brick_gas_tint
        };
        Color::srgb(rgb[0], rgb[1], rgb[2])
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
/// Stores one base config entry loaded from TOML for a built-in material or structure.
///
/// # Fields
/// - `label`: Human-readable label shown in UI for this material or structure.
/// - `draw_priority`: Relative ordering value used when drawing overlapping visuals.
/// - `size_in_cells`: Sprite footprint expressed in world-cell dimensions.
pub struct VisualPlacementConfig {
    pub label: String,
    pub draw_priority: i32,
    pub size_in_cells: UVec2,
}

#[derive(Resource, Clone, Debug)]
/// Stores appearance-layout config for every built-in structure kind.
pub struct StructureVisualConfigMap {
    configs: HashMap<StructureKind, VisualPlacementConfig>,
}

impl StructureVisualConfigMap {
    /// Returns the appearance-layout config for the requested structure kind.
    pub fn get(&self, kind: StructureKind) -> &VisualPlacementConfig {
        self.configs
            .get(&kind)
            .unwrap_or_else(|| panic!("missing structure visual config for {:?}", kind))
    }

    pub(crate) fn from_entries(entries: Vec<(StructureKind, VisualPlacementConfig)>) -> Self {
        Self {
            configs: entries.into_iter().collect(),
        }
    }
}

#[derive(Resource, Clone, Debug)]
/// Stores appearance-layout config for every built-in wall material.
pub struct CellVisualPlacementConfigMap {
    configs: HashMap<CellMaterial, VisualPlacementConfig>,
}

impl CellVisualPlacementConfigMap {
    /// Returns the appearance-layout config for the requested wall material.
    pub fn get(&self, material: CellMaterial) -> &VisualPlacementConfig {
        self.configs
            .get(&material)
            .unwrap_or_else(|| panic!("missing cell visual config for {:?}", material))
    }

    pub(crate) fn from_entries(entries: Vec<(CellMaterial, VisualPlacementConfig)>) -> Self {
        Self {
            configs: entries.into_iter().collect(),
        }
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
    pub pipe_simulation: PipeSimulationConfig,
    pub cell_visuals: CellTypeVisualConfig,
    pub world_cell_hud: WorldCellHudConfig,
    pub structure_hud: StructureHudConfigMap,
    pub structure_visuals: StructureVisualConfigMap,
    pub cell_visual_layouts: CellVisualPlacementConfigMap,
}

impl GameConfig {
    /// Runs `load_from_default_location` logic.
    pub fn load_from_default_location() -> Result<Self, String> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let core_root = repo_root.join("config");
        let default_plugin_root = crate::plugins::default_plugin::config_root(repo_root);
        Self::load_from_roots(&core_root, &default_plugin_root)
    }

    /// Loads config from the default location and includes enabled plugin content.
    pub fn load_from_default_location_with_content(
        content_registry: &ContentRegistry,
    ) -> Result<Self, String> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let core_root = repo_root.join("config");
        let default_plugin_root = crate::plugins::default_plugin::config_root(repo_root);
        Self::load_from_roots_with_content(&core_root, &default_plugin_root, content_registry)
    }

    /// Runs `load_from_root` logic.
    pub fn load_from_root(root: &Path) -> Result<Self, String> {
        let content_registry = crate::plugins::default_plugin::default_content_registry();
        Self::load_from_root_with_content(root, &content_registry)
    }

    /// Loads config from one root and appends external plugin substances.
    pub fn load_from_root_with_content(
        root: &Path,
        content_registry: &ContentRegistry,
    ) -> Result<Self, String> {
        Self::load_from_roots_with_content(root, root, content_registry)
    }

    /// Loads config from separate core and default plugin roots.
    pub fn load_from_roots(core_root: &Path, default_plugin_root: &Path) -> Result<Self, String> {
        let content_registry = crate::plugins::default_plugin::default_content_registry();
        Self::load_from_roots_with_content(core_root, default_plugin_root, &content_registry)
    }

    /// Loads config from separate roots and appends external plugin substances.
    pub fn load_from_roots_with_content(
        core_root: &Path,
        default_plugin_root: &Path,
        content_registry: &ContentRegistry,
    ) -> Result<Self, String> {
        let simulation = read_toml::<SimulationToml>(&core_root.join("simulation.toml"))?;
        let pipe = read_toml::<PipeToml>(&default_plugin_root.join("pipe_runtime.toml"))?;
        let cell_types = read_toml::<CellTypesToml>(&default_plugin_root.join("cell_types.toml"))?;
        let substances =
            load_plugin_substances(&default_plugin_root.join("gases"), content_registry)?;
        let world_cell_hud = load_world_cell_hud_config(
            &default_plugin_root.join("cell_types.toml"),
            cell_types.world_cell_hud,
        )?;
        let (structure_visuals, structure_hud, cell_visual_layouts) =
            load_visual_placement_configs(&default_plugin_root.join("structures"))?;

        let gas_registry = GasRegistry::from_substances(substances)?;

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

        let pipe_simulation = PipeSimulationConfig {
            cell_volume_ratio: pipe.cell_volume_ratio,
            cell_particle_pressure_pa: pipe.cell_particle_pressure_pa,
            pipe_flux_gain: pipe.pipe_flux_gain,
            pipe_flux_damping: pipe.pipe_flux_damping,
            max_pipe_flux_particles_per_tick: pipe.max_pipe_flux_particles_per_tick,
            vent_discharge_coefficient: pipe.vent_discharge_coefficient,
            max_vent_flux_particles_per_tick: pipe.max_vent_flux_particles_per_tick,
            vent_choked_pressure_ratio: pipe.vent_choked_pressure_ratio,
            pressure_epsilon_pa: pipe.pressure_epsilon_pa,
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
            pipe_simulation,
            cell_visuals,
            world_cell_hud,
            structure_hud,
            structure_visuals,
            cell_visual_layouts,
        })
    }

    /// Builds the current gas registry from default config and enabled plugin content.
    pub fn load_gas_registry_from_default_location(
        content_registry: &ContentRegistry,
    ) -> Result<GasRegistry, String> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = crate::plugins::default_plugin::config_root(repo_root);
        let substances = load_plugin_substances(&root.join("gases"), content_registry)?;
        GasRegistry::from_substances(substances)
    }
}

include!("config_loader_block.rs");
include!("audio_settings_block.rs");
include!("config_tests_block.rs");
