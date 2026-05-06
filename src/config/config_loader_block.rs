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

#[derive(Deserialize)]
struct VisualPlacementToml {
    draw_priority: i32,
    size_in_cells: [u32; 2],
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

fn load_visual_placement_configs(
    structures_root: &Path,
) -> Result<(StructureVisualConfigMap, CellVisualPlacementConfigMap), String> {
    let structure_entries = vec![
        (
            StructureKind::Pipe,
            load_visual_placement_entry(structures_root, "pipe.toml")?,
        ),
        (
            StructureKind::Vent,
            load_visual_placement_entry(structures_root, "vent.toml")?,
        ),
        (
            StructureKind::GasSource,
            load_visual_placement_entry(structures_root, "gas_source.toml")?,
        ),
        (
            StructureKind::GasSink,
            load_visual_placement_entry(structures_root, "gas_sink.toml")?,
        ),
        (
            StructureKind::GasPipeBridge,
            load_visual_placement_entry(structures_root, "gas_pipe_bridge.toml")?,
        ),
    ];
    let cell_entries = vec![
        (
            CellMaterial::Boundary,
            load_visual_placement_entry(structures_root, "boundary.toml")?,
        ),
        (
            CellMaterial::Brick,
            load_visual_placement_entry(structures_root, "brick.toml")?,
        ),
        (
            CellMaterial::Metal,
            load_visual_placement_entry(structures_root, "metal.toml")?,
        ),
    ];

    for (kind, config) in &structure_entries {
        validate_structure_visual_placement(*kind, *config)?;
    }
    for (material, config) in &cell_entries {
        validate_cell_visual_placement(*material, *config)?;
    }

    Ok((
        StructureVisualConfigMap::from_entries(structure_entries),
        CellVisualPlacementConfigMap::from_entries(cell_entries),
    ))
}

fn load_visual_placement_entry(
    structures_root: &Path,
    file_name: &str,
) -> Result<VisualPlacementConfig, String> {
    let path = structures_root.join(file_name);
    let file = read_toml::<VisualPlacementToml>(&path)?;
    let size_in_cells = UVec2::new(file.size_in_cells[0], file.size_in_cells[1]);
    if size_in_cells.x == 0 || size_in_cells.y == 0 {
        return Err(format!(
            "Visual config '{}' has invalid size_in_cells [{}, {}]",
            path.display(),
            size_in_cells.x,
            size_in_cells.y
        ));
    }
    Ok(VisualPlacementConfig {
        draw_priority: file.draw_priority,
        size_in_cells,
    })
}

fn validate_structure_visual_placement(
    kind: StructureKind,
    config: VisualPlacementConfig,
) -> Result<(), String> {
    let expected = match kind {
        StructureKind::Pipe
        | StructureKind::Vent
        | StructureKind::GasSource
        | StructureKind::GasSink => UVec2::ONE,
        StructureKind::GasPipeBridge => UVec2::new(3, 1),
    };
    if config.size_in_cells != expected {
        return Err(format!(
            "Structure visual config for {:?} must use size_in_cells [{}, {}], got [{}, {}]",
            kind,
            expected.x,
            expected.y,
            config.size_in_cells.x,
            config.size_in_cells.y
        ));
    }
    Ok(())
}

fn validate_cell_visual_placement(
    material: CellMaterial,
    config: VisualPlacementConfig,
) -> Result<(), String> {
    if config.size_in_cells != UVec2::ONE {
        return Err(format!(
            "Cell visual config for {:?} must use size_in_cells [1, 1], got [{}, {}]",
            material,
            config.size_in_cells.x,
            config.size_in_cells.y
        ));
    }
    Ok(())
}

