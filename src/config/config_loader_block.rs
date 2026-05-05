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
    thermal_motion_scale: f32,
}

#[derive(Deserialize)]
struct SolverTuningToml {
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

