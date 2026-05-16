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
struct PipeToml {
    cell_volume_ratio: f32,
    cell_particle_pressure_pa: f32,
    pipe_step_interval_ticks: u32,
    max_pipe_hop_particles_per_step: u32,
    min_pipe_branch_residual_particles: u32,
    vent_discharge_coefficient: f32,
    max_vent_flux_particles_per_tick: f32,
    vent_choked_pressure_ratio: f32,
    pressure_epsilon_pa: f32,
    pump_input_pressure_pa: f32,
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
    world_cell_hud: WorldCellHudToml,
}

#[derive(Deserialize)]
struct GasToml {
    id: String,
    label: String,
    molecular_mass: f32,
    color: [f32; 3],
}

#[derive(Deserialize)]
struct CellVisualPlacementToml {
    label: String,
    draw_priority: i32,
    size_in_cells: [u32; 2],
}

#[derive(Deserialize)]
struct StructureVisualToml {
    label: String,
    draw_priority: i32,
    size_in_cells: [u32; 2],
    hud: HudBlockToml,
}

#[derive(Deserialize)]
struct HudBlockToml {
    sort_order: i32,
    #[serde(default)]
    substance_containers: Vec<SubstanceContainerToml>,
}

#[derive(Deserialize)]
struct WorldCellHudToml {
    label: String,
    sort_order: i32,
    #[serde(default)]
    substance_containers: Vec<SubstanceContainerToml>,
}

#[derive(Deserialize)]
struct SubstanceContainerToml {
    substance: String,
    backing: String,
    kind: Option<String>,
    visible_on_hover: String,
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read config '{}': {}", path.display(), err))?;
    toml::from_str::<T>(&content)
        .map_err(|err| format!("Failed to parse config '{}': {}", path.display(), err))
}

fn load_default_plugin_substances(gases_root: &Path) -> Result<Vec<SubstanceDefinition>, String> {
    let mut by_id = crate::plugins::default_plugin::default_substance_definitions()
        .into_iter()
        .map(|definition| (definition.id.clone(), definition))
        .collect::<std::collections::BTreeMap<_, _>>();

    if !gases_root.exists() {
        return Ok(by_id.into_values().collect());
    }

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

    let mut seen_config_ids = std::collections::BTreeSet::new();
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

        let substance_id = crate::plugins::default_plugin::default_substance_id_for_alias(&file.id)
            .map_err(|err| {
                format!(
                    "Gas config '{}' has invalid default plugin substance id '{}': {}",
                    path.display(),
                    file.id,
                    err
                )
            })?;
        if !substance_id
            .as_str()
            .starts_with(crate::plugins::DEFAULT_PLUGIN_ID_VALUE)
        {
            return Err(format!(
                "Gas config '{}' must describe a flux.default substance, got '{}'",
                path.display(),
                substance_id
            ));
        }
        if !seen_config_ids.insert(substance_id.clone()) {
            return Err(format!("Duplicate gas id '{}'", file.id));
        }

        let alias = file
            .id
            .rsplit('.')
            .next()
            .unwrap_or(file.id.as_str())
            .to_string();
        let definition = SubstanceDefinition::gas(
            substance_id.clone(),
            crate::plugins::PluginId::default_plugin(),
            file.label,
            file.molecular_mass,
            file.color,
            vec![alias],
        )?;
        by_id.insert(substance_id, definition);
    }

    Ok(by_id.into_values().collect())
}

fn load_plugin_substances(
    gases_root: &Path,
    content_registry: &ContentRegistry,
) -> Result<Vec<SubstanceDefinition>, String> {
    let mut substances = load_default_plugin_substances(gases_root)?;
    let default_plugin = PluginId::default_plugin();
    substances.extend(
        content_registry
            .substances()
            .values()
            .filter(|definition| definition.plugin_id != default_plugin)
            .cloned(),
    );
    Ok(substances)
}

fn load_visual_placement_configs(
    structures_root: &Path,
) -> Result<
    (
        StructureVisualConfigMap,
        StructureHudConfigMap,
        CellVisualPlacementConfigMap,
    ),
    String,
> {
    let structure_entries = crate::plugins::default_plugin::default_structure_descriptors()
        .into_iter()
        .map(|descriptor| {
            load_structure_visual_config_entry(structures_root, descriptor.config_file_name)
                .map(|config| (descriptor.kind, config))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cell_entries = crate::plugins::default_plugin::default_cell_descriptors()
        .into_iter()
        .map(|descriptor| {
            load_cell_visual_placement_entry(structures_root, descriptor.config_file_name)
                .map(|mut config| {
                    // One shared visual TOML (e.g. brick/metal family) can back multiple
                    // material entities, so the UI label must come from the descriptor itself.
                    config.label = descriptor.visual.label.clone();
                    (descriptor.material, config)
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    for (kind, (visual_config, hud_config)) in &structure_entries {
        validate_structure_visual_placement(*kind, visual_config)?;
        validate_structure_hud_config(*kind, hud_config)?;
    }
    for (material, config) in &cell_entries {
        validate_cell_visual_placement(*material, config)?;
    }

    Ok((
        StructureVisualConfigMap::from_entries(
            structure_entries
                .iter()
                .map(|(kind, (visual, _))| (*kind, visual.clone()))
                .collect(),
        ),
        StructureHudConfigMap::from_entries(
            structure_entries
                .into_iter()
                .map(|(kind, (_, hud))| (kind, hud))
                .collect(),
        ),
        CellVisualPlacementConfigMap::from_entries(cell_entries),
    ))
}

fn load_structure_visual_config_entry(
    structures_root: &Path,
    file_name: &str,
) -> Result<(VisualPlacementConfig, HudBlockConfig), String> {
    let path = structures_root.join(file_name);
    let file = read_toml::<StructureVisualToml>(&path)?;
    let size_in_cells = UVec2::new(file.size_in_cells[0], file.size_in_cells[1]);
    if size_in_cells.x == 0 || size_in_cells.y == 0 {
        return Err(format!(
            "Visual config '{}' has invalid size_in_cells [{}, {}]",
            path.display(),
            size_in_cells.x,
            size_in_cells.y
        ));
    }
    Ok((
        VisualPlacementConfig {
            label: file.label,
            draw_priority: file.draw_priority,
            size_in_cells,
        },
        parse_hud_block_config(&path, file.hud)?,
    ))
}

fn load_cell_visual_placement_entry(
    structures_root: &Path,
    file_name: &str,
) -> Result<VisualPlacementConfig, String> {
    let path = structures_root.join(file_name);
    let file = read_toml::<CellVisualPlacementToml>(&path)?;
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
        label: file.label,
        draw_priority: file.draw_priority,
        size_in_cells,
    })
}

fn load_world_cell_hud_config(
    path: &Path,
    hud: WorldCellHudToml,
) -> Result<WorldCellHudConfig, String> {
    if hud.label.trim().is_empty() {
        return Err(format!("HUD config '{}' has empty label", path.display()));
    }
    let block = parse_hud_block_config(
        path,
        HudBlockToml {
            sort_order: hud.sort_order,
            substance_containers: hud.substance_containers,
        },
    )?;
    let config = WorldCellHudConfig {
        label: hud.label,
        block,
    };
    validate_world_cell_hud_config(&config, path)?;
    Ok(config)
}

fn validate_structure_visual_placement(
    kind: StructureKind,
    config: &VisualPlacementConfig,
) -> Result<(), String> {
    if config.label.trim().is_empty() {
        return Err(format!(
            "Structure visual config for {:?} must use non-empty label",
            kind
        ));
    }
    let expected = crate::plugins::default_plugin::structure_content_descriptor(kind)
        .visual
        .size_in_cells;
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
    config: &VisualPlacementConfig,
) -> Result<(), String> {
    if config.label.trim().is_empty() {
        return Err(format!(
            "Cell visual config for {:?} must use non-empty label",
            material
        ));
    }
    let expected = crate::plugins::default_plugin::cell_content_descriptor(material)
        .visual
        .size_in_cells;
    if config.size_in_cells != expected {
        return Err(format!(
            "Cell visual config for {:?} must use size_in_cells [{}, {}], got [{}, {}]",
            material,
            expected.x,
            expected.y,
            config.size_in_cells.x,
            config.size_in_cells.y
        ));
    }
    Ok(())
}

fn parse_hud_block_config(path: &Path, file: HudBlockToml) -> Result<HudBlockConfig, String> {
    let mut substance_containers = Vec::with_capacity(file.substance_containers.len());
    for (index, container) in file.substance_containers.into_iter().enumerate() {
        substance_containers.push(parse_substance_container_config(path, index, container)?);
    }

    Ok(HudBlockConfig {
        sort_order: file.sort_order,
        substance_containers,
    })
}

fn parse_substance_container_config(
    path: &Path,
    index: usize,
    file: SubstanceContainerToml,
) -> Result<SubstanceContainerConfig, String> {
    let substance = match file.substance.trim() {
        "gas" => SubstanceKind::Gas,
        other => {
            return Err(format!(
                "HUD config '{}' has unsupported substance '{}' at container {}",
                path.display(),
                other,
                index
            ));
        }
    };

    let backing = match file.backing.trim() {
        "world_cell" => {
            if file.kind.is_some() {
                return Err(format!(
                    "HUD config '{}' uses unexpected kind for world_cell container {}",
                    path.display(),
                    index
                ));
            }
            ContainerBacking::WorldCell
        }
        "pipe_node" => {
            let Some(kind) = file.kind.as_deref() else {
                return Err(format!(
                    "HUD config '{}' is missing pipe node kind for container {}",
                    path.display(),
                    index
                ));
            };
            let kind = match kind {
                "pipe" => ConfiguredPipeNodeKind::Pipe,
                "bridge_pipe" => ConfiguredPipeNodeKind::BridgePipe,
                other => {
                    return Err(format!(
                        "HUD config '{}' has unsupported pipe node kind '{}' at container {}",
                        path.display(),
                        other,
                        index
                    ));
                }
            };
            ContainerBacking::PipeNode { kind }
        }
        other => {
            return Err(format!(
                "HUD config '{}' has unsupported backing '{}' at container {}",
                path.display(),
                other,
                index
            ));
        }
    };

    let visible_on_hover = match file.visible_on_hover.trim() {
        "same_cell" => HoverVisibility::SameCell,
        "container_cell" => HoverVisibility::ContainerCell,
        other => {
            return Err(format!(
                "HUD config '{}' has unsupported visible_on_hover '{}' at container {}",
                path.display(),
                other,
                index
            ));
        }
    };

    Ok(SubstanceContainerConfig {
        substance,
        backing,
        visible_on_hover,
    })
}

fn validate_world_cell_hud_config(config: &WorldCellHudConfig, path: &Path) -> Result<(), String> {
    for (index, container) in config.block.substance_containers.iter().enumerate() {
        if container.backing != ContainerBacking::WorldCell {
            return Err(format!(
                "HUD config '{}' uses non-world backing in world_cell_hud container {}",
                path.display(),
                index
            ));
        }
    }
    Ok(())
}

fn validate_structure_hud_config(
    kind: StructureKind,
    block: &HudBlockConfig,
) -> Result<(), String> {
    for (index, container) in block.substance_containers.iter().enumerate() {
        match container.backing {
            ContainerBacking::WorldCell => {
                return Err(format!(
                    "HUD config for {:?} cannot use world_cell backing in container {}",
                    kind, index
                ));
            }
            ContainerBacking::PipeNode {
                kind: ConfiguredPipeNodeKind::Pipe,
            } => {
                if !crate::plugins::default_plugin::is_pipe_structure(kind) {
                    return Err(format!(
                        "HUD config for {:?} cannot use pipe-node backing 'pipe' in container {}",
                        kind, index
                    ));
                }
                if container.visible_on_hover != HoverVisibility::SameCell {
                    return Err(format!(
                        "HUD config for {:?} must use same_cell visibility for pipe container {}",
                        kind, index
                    ));
                }
            }
            ContainerBacking::PipeNode {
                kind: ConfiguredPipeNodeKind::BridgePipe,
            } => {
                if !crate::plugins::default_plugin::is_gas_pipe_bridge_structure(kind) {
                    return Err(format!(
                        "HUD config for {:?} cannot use pipe-node backing 'bridge_pipe' in container {}",
                        kind, index
                    ));
                }
                if container.visible_on_hover != HoverVisibility::ContainerCell {
                    return Err(format!(
                        "HUD config for {:?} must use container_cell visibility for bridge pipe container {}",
                        kind, index
                    ));
                }
            }
        }
    }
    Ok(())
}

