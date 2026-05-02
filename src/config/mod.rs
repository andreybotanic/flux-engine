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
pub struct GasRegistry {
    gases: Vec<GasDefinition>,
    by_id: HashMap<String, usize>,
}

impl GasRegistry {
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

    pub fn all(&self) -> &[GasDefinition] {
        &self.gases
    }

    pub fn count(&self) -> usize {
        self.gases.len()
    }

    pub fn get(&self, index: usize) -> Option<&GasDefinition> {
        self.gases.get(index)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    pub fn molecular_masses(&self) -> Vec<f32> {
        self.gases.iter().map(|gas| gas.molecular_mass).collect()
    }
}

#[derive(Clone, Debug)]
pub struct GasDefinition {
    pub id: String,
    pub label: String,
    pub molecular_mass: f32,
    pub color: [f32; 3],
}

impl GasDefinition {
    pub fn color_as_bevy(&self) -> Color {
        Color::srgb(self.color[0], self.color[1], self.color[2])
    }
}

#[derive(Resource, Clone, Copy, Debug)]
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
    pub fn main_tint(self, material: CellMaterial) -> Color {
        let rgb = match material {
            CellMaterial::Boundary => self.boundary_main_tint,
            CellMaterial::Brick => self.brick_main_tint,
            CellMaterial::Metal => self.metal_main_tint,
        };
        Color::srgb(rgb[0], rgb[1], rgb[2])
    }

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
pub struct GameConfig {
    pub gas_registry: GasRegistry,
    pub simulation_rate: SimulationRateConfig,
    pub gas_simulation: GasSimulationConfig,
    pub gas_visual: GasVisualSettings,
    pub gas_main_visual: GasMainViewVisualConfig,
    pub cell_visuals: CellTypeVisualConfig,
}

impl GameConfig {
    pub fn load_from_default_location() -> Result<Self, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("config");
        Self::load_from_root(&root)
    }

    pub fn load_from_root(root: &Path) -> Result<Self, String> {
        let simulation = read_toml::<SimulationToml>(&root.join("simulation.toml"))?;
        let cell_types = read_toml::<CellTypesToml>(&root.join("cell_types.toml"))?;
        let gases = load_gas_files(&root.join("gases"))?;

        let gas_registry = GasRegistry::new(gases)?;

        let simulation_rate = SimulationRateConfig {
            target_hz: simulation.rate.target_hz,
        };

        let gas_simulation = GasSimulationConfig {
            enable_lbm_velocity: simulation.simulation.enable_lbm_velocity,
            enable_species_relaxation: simulation.simulation.enable_species_relaxation,
            reconcile_every_n_steps: simulation.simulation.reconcile_every_n_steps,
            mass_fix_every_n_steps: simulation.simulation.mass_fix_every_n_steps,
            mass_fix_error_threshold: simulation.simulation.mass_fix_error_threshold,
            mass_fix_min_residual: simulation.simulation.mass_fix_min_residual,
            solver_tuning: SolverTuning {
                tau_even: simulation.solver_tuning.tau_even,
                tau_odd: simulation.solver_tuning.tau_odd,
                target_cfl_like_limit: simulation.solver_tuning.target_cfl_like_limit,
                velocity_damping: simulation.solver_tuning.velocity_damping,
                species_eq_blend: simulation.solver_tuning.species_eq_blend,
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

#[derive(Deserialize)]
struct SimulationToml {
    rate: SimulationRateToml,
    simulation: GasSimulationToml,
    solver_tuning: SolverTuningToml,
    visual: VisualToml,
}

#[derive(Deserialize)]
struct SimulationRateToml {
    target_hz: u32,
}

#[derive(Deserialize)]
struct GasSimulationToml {
    enable_lbm_velocity: bool,
    enable_species_relaxation: bool,
    reconcile_every_n_steps: u32,
    mass_fix_every_n_steps: u32,
    mass_fix_error_threshold: f32,
    mass_fix_min_residual: f32,
}

#[derive(Deserialize)]
struct SolverTuningToml {
    tau_even: f32,
    tau_odd: f32,
    target_cfl_like_limit: f32,
    #[serde(default = "default_velocity_damping")]
    velocity_damping: f32,
    #[serde(default = "default_species_eq_blend")]
    species_eq_blend: f32,
    enable_buoyancy: bool,
    buoyancy_strength: f32,
    buoyancy_window_radius: u8,
    buoyancy_window_sigma: f32,
    buoyancy_gain: f32,
    buoyancy_alpha: f32,
    buoyancy_force_cap: f32,
}

#[derive(Deserialize)]
struct VisualToml {
    gamma: f32,
    max_particles_for_max_color: u32,
    f1_min_particles: f32,
    f1_max_particles_for_max_intensity: f32,
    f1_min_intensity: f32,
    f1_alpha: f32,
}

fn default_velocity_damping() -> f32 {
    SolverTuning::default().velocity_damping
}

fn default_species_eq_blend() -> f32 {
    SolverTuning::default().species_eq_blend
}

#[derive(Deserialize)]
struct CellTintToml {
    main_tint: [f32; 3],
    gas_tint: [f32; 3],
}

#[derive(Deserialize)]
struct CellTypesToml {
    boundary: CellTintToml,
    brick: CellTintToml,
    metal: CellTintToml,
}

#[derive(Deserialize)]
struct GasToml {
    id: String,
    label: String,
    molecular_mass: f32,
    color: [f32; 3],
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read config '{}': {}", path.display(), err))?;
    toml::from_str::<T>(&content)
        .map_err(|err| format!("Failed to parse config '{}': {}", path.display(), err))
}

fn load_gas_files(gases_root: &Path) -> Result<Vec<GasDefinition>, String> {
    let mut files: Vec<PathBuf> = fs::read_dir(gases_root)
        .map_err(|err| {
            format!(
                "Failed to read gas config directory '{}': {}",
                gases_root.display(),
                err
            )
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
        })
        .collect();

    files.sort();

    if files.is_empty() {
        return Err(format!(
            "Gas config directory '{}' contains no .toml files",
            gases_root.display()
        ));
    }

    let mut gases = Vec::with_capacity(files.len());
    for path in files {
        let file = read_toml::<GasToml>(&path)?;

        if file.id.trim().is_empty() {
            return Err(format!("Gas config '{}' has empty id", path.display()));
        }
        if file.label.trim().is_empty() {
            return Err(format!("Gas config '{}' has empty label", path.display()));
        }
        if !file.molecular_mass.is_finite() || file.molecular_mass <= 0.0 {
            return Err(format!(
                "Gas config '{}' has invalid molecular_mass {}",
                path.display(),
                file.molecular_mass
            ));
        }
        for component in file.color {
            if !(0.0..=1.0).contains(&component) || !component.is_finite() {
                return Err(format!(
                    "Gas config '{}' has invalid color component {}",
                    path.display(),
                    component
                ));
            }
        }

        gases.push(GasDefinition {
            id: file.id,
            label: file.label,
            molecular_mass: file.molecular_mass,
            color: file.color,
        });
    }

    Ok(gases)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_root(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        fs::create_dir_all(root.join("gases")).expect("create temp dirs");
        root
    }

    fn write_minimal_configs(root: &Path) {
        fs::write(
            root.join("simulation.toml"),
            r#"
[rate]
target_hz = 30

[simulation]
enable_lbm_velocity = true
enable_species_relaxation = true
reconcile_every_n_steps = 4
mass_fix_every_n_steps = 4
mass_fix_error_threshold = 0.0001
mass_fix_min_residual = 0.00001

[solver_tuning]
tau_even = 0.85
tau_odd = 1.15
target_cfl_like_limit = 0.85
velocity_damping = 0.08
species_eq_blend = 0.28
enable_buoyancy = true
buoyancy_strength = 0.12
buoyancy_window_radius = 2
buoyancy_window_sigma = 1.2
buoyancy_gain = 2.2
buoyancy_alpha = 0.9
buoyancy_force_cap = 0.20

[visual]
gamma = 1.0
max_particles_for_max_color = 1000
f1_min_particles = 1.0
f1_max_particles_for_max_intensity = 1000.0
f1_min_intensity = 0.05
f1_alpha = 0.88
"#,
        )
        .expect("write simulation");

        fs::write(
            root.join("cell_types.toml"),
            r#"
[boundary]
main_tint = [0.90, 0.90, 0.91]
gas_tint = [0.66, 0.66, 0.67]

[brick]
main_tint = [0.99, 0.99, 0.99]
gas_tint = [0.72, 0.72, 0.74]

[metal]
main_tint = [0.99, 0.99, 0.99]
gas_tint = [0.71, 0.71, 0.73]
"#,
        )
        .expect("write cell types");
    }

    #[test]
    fn config_loader_rejects_duplicate_gas_ids() {
        let root = make_temp_root("flux_cfg_dup");
        write_minimal_configs(&root);

        fs::write(
            root.join("gases").join("a.toml"),
            r#"id = "same"
label = "A"
molecular_mass = 1.0
color = [0.2, 0.2, 0.2]
"#,
        )
        .expect("write gas a");
        fs::write(
            root.join("gases").join("b.toml"),
            r#"id = "same"
label = "B"
molecular_mass = 2.0
color = [0.3, 0.3, 0.3]
"#,
        )
        .expect("write gas b");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("must fail on duplicate ids"),
            Err(err) => err,
        };
        assert!(err.contains("Duplicate gas id"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_rejects_empty_gas_list() {
        let root = make_temp_root("flux_cfg_empty");
        write_minimal_configs(&root);
        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("must fail on empty gases"),
            Err(err) => err,
        };
        assert!(err.contains("contains no .toml files"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_rejects_invalid_gas_values() {
        let root = make_temp_root("flux_cfg_bad");
        write_minimal_configs(&root);
        fs::write(
            root.join("gases").join("bad.toml"),
            r#"id = "bad"
label = "Bad"
molecular_mass = -5.0
color = [1.2, 0.0, 0.0]
"#,
        )
        .expect("write bad gas");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("must fail on invalid values"),
            Err(err) => err,
        };
        assert!(err.contains("invalid molecular_mass") || err.contains("invalid color"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gas_registry_orders_by_molecular_mass() {
        let registry = GasRegistry::new(vec![
            GasDefinition {
                id: "co2".to_string(),
                label: "CO2".to_string(),
                molecular_mass: 44.009,
                color: [0.5, 0.5, 0.5],
            },
            GasDefinition {
                id: "h2".to_string(),
                label: "H2".to_string(),
                molecular_mass: 2.016,
                color: [0.8, 0.2, 0.9],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "O2".to_string(),
                molecular_mass: 31.998,
                color: [0.0, 0.85, 0.85],
            },
        ])
        .expect("valid registry");

        let ids = registry
            .all()
            .iter()
            .map(|g| g.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["h2", "o2", "co2"]);
    }
}
