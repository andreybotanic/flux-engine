/// Runs `saves_root_default` logic.
pub fn saves_root_default() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("saves")
}

/// Runs `list_saves` logic.
pub fn list_saves(root: &Path) -> Result<Vec<SaveDescriptor>, SaveError> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut saves = Vec::new();
    let entries = fs::read_dir(root).map_err(|err| {
        SaveError::Io(format!(
            "Failed to read saves root '{}': {}",
            root.display(),
            err
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            SaveError::Io(format!(
                "Failed to read entry from saves root '{}': {}",
                root.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let meta_path = path.join(META_FILE);
        if !meta_path.exists() {
            continue;
        }
        let meta = read_meta(&meta_path)?;
        saves.push(SaveDescriptor {
            id: meta.save_id,
            display_name: meta.display_name,
            created_at_unix_ms: meta.created_at_unix_ms,
            updated_at_unix_ms: meta.updated_at_unix_ms,
        });
    }

    saves.sort_by(|a, b| {
        b.updated_at_unix_ms
            .cmp(&a.updated_at_unix_ms)
            .then_with(|| b.created_at_unix_ms.cmp(&a.created_at_unix_ms))
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(saves)
}

/// Runs `create_save` logic.
pub fn create_save(
    root: &Path,
    display_name: &str,
    world: &WorldGrid,
    gas: &GasField,
    structures: &GasStructureGrid,
    pipe_layout: &crate::world::pipes::PipeGrid,
    pipe_gas: &crate::simulation::pipes::PipeGasField,
    gas_registry: &GasRegistry,
    simulation_step: u64,
) -> Result<SaveDescriptor, SaveError> {
    validate_display_name(display_name)?;
    fs::create_dir_all(root).map_err(|err| {
        SaveError::Io(format!(
            "Failed to create saves root '{}': {}",
            root.display(),
            err
        ))
    })?;

    let mut save_id = generate_save_id();
    while root.join(&save_id).exists() {
        save_id = generate_save_id();
    }

    let now = now_unix_ms()?;
    let descriptor = SaveDescriptor {
        id: save_id.clone(),
        display_name: display_name.to_string(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };

    write_slot(
        root,
        &descriptor,
        world,
        gas,
        structures,
        pipe_layout,
        pipe_gas,
        gas_registry,
        simulation_step,
        false,
    )?;
    Ok(descriptor)
}

/// Runs `overwrite_save` logic.
pub fn overwrite_save(
    root: &Path,
    save_id: &str,
    world: &WorldGrid,
    gas: &GasField,
    structures: &GasStructureGrid,
    pipe_layout: &crate::world::pipes::PipeGrid,
    pipe_gas: &crate::simulation::pipes::PipeGasField,
    gas_registry: &GasRegistry,
    simulation_step: u64,
) -> Result<SaveDescriptor, SaveError> {
    let slot_dir = root.join(save_id);
    let meta_path = slot_dir.join(META_FILE);
    if !meta_path.exists() {
        return Err(SaveError::Validation(format!(
            "Save slot '{}' does not exist",
            save_id
        )));
    }
    let previous = read_meta(&meta_path)?;
    let descriptor = SaveDescriptor {
        id: previous.save_id,
        display_name: previous.display_name,
        created_at_unix_ms: previous.created_at_unix_ms,
        updated_at_unix_ms: now_unix_ms()?,
    };
    write_slot(
        root,
        &descriptor,
        world,
        gas,
        structures,
        pipe_layout,
        pipe_gas,
        gas_registry,
        simulation_step,
        true,
    )?;
    Ok(descriptor)
}

/// Runs `load_save` logic.
pub fn load_save(
    root: &Path,
    save_id: &str,
    gas_registry: &GasRegistry,
) -> Result<LoadedSave, SaveError> {
    let slot_dir = root.join(save_id);
    let meta_path = slot_dir.join(META_FILE);
    let meta = read_meta(&meta_path)?;
    validate_meta_dimensions(&meta)?;

    let chunk_map = meta
        .chunks
        .iter()
        .map(|chunk| (chunk.id.clone(), chunk.file.clone()))
        .collect::<HashMap<_, _>>();

    let world_path =
        slot_dir.join(chunk_map.get(CHUNK_WORLD_CELLS_ID).ok_or_else(|| {
            SaveError::Validation("Save meta missing world_cells chunk".to_string())
        })?);
    let gas_path =
        slot_dir.join(chunk_map.get(CHUNK_GAS_STATE_ID).ok_or_else(|| {
            SaveError::Validation("Save meta missing gas_state chunk".to_string())
        })?);
    let structures_path = slot_dir.join(chunk_map.get(CHUNK_GAS_STRUCTURES_ID).ok_or_else(|| {
        SaveError::Validation("Save meta missing gas_structures chunk".to_string())
    })?);
    let pipe_layout_path = chunk_map
        .get(CHUNK_PIPE_LAYOUT_ID)
        .map(|file| slot_dir.join(file));
    let pipe_gas_path = chunk_map.get(CHUNK_PIPE_GAS_ID).map(|file| slot_dir.join(file));

    let world_codes = read_world_cells_chunk(&world_path)?;
    let gas_file = read_gas_chunk(&gas_path)?;
    let structures_snapshot = read_gas_structures_chunk(&structures_path)?;
    let pipe_layout_snapshot = if let Some(path) = pipe_layout_path {
        read_pipe_layout_chunk(&path)?
    } else {
        crate::world::pipes::PipeGrid::default().snapshot_state()
    };
    let pipe_gas_snapshot = if let Some(path) = pipe_gas_path {
        read_pipe_gas_chunk(&path, gas_registry)?
    } else {
        crate::simulation::pipes::PipeGasField::from_registry(gas_registry).snapshot_state()
    };

    if gas_file.width != WORLD_WIDTH || gas_file.height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "Gas chunk dimensions mismatch: got {}x{}, expected {}x{}",
            gas_file.width, gas_file.height, WORLD_WIDTH, WORLD_HEIGHT
        )));
    }

    let mapped_snapshot = map_saved_gas_snapshot_to_registry(&gas_file, gas_registry)?;

    Ok(LoadedSave {
        descriptor: SaveDescriptor {
            id: meta.save_id,
            display_name: meta.display_name,
            created_at_unix_ms: meta.created_at_unix_ms,
            updated_at_unix_ms: meta.updated_at_unix_ms,
        },
        state: RuntimeWorldState {
            world_cell_codes: world_codes,
            gas_snapshot: mapped_snapshot,
            gas_structures_snapshot: structures_snapshot,
            pipe_layout_snapshot,
            pipe_gas_snapshot,
            simulation_step: gas_file.simulation_step,
        },
    })
}

/// Runs `new_game_snapshot` logic.
pub fn new_game_snapshot(gas_registry: &GasRegistry) -> RuntimeWorldState {
    let world = WorldGrid::default();
    let gas = GasField::from_registry(gas_registry);
    let structures = GasStructureGrid::default();
    let pipe_layout = crate::world::pipes::PipeGrid::default();
    let pipe_gas = crate::simulation::pipes::PipeGasField::from_registry(gas_registry);
    RuntimeWorldState {
        world_cell_codes: world.snapshot_cell_codes(),
        gas_snapshot: gas.snapshot_state(),
        gas_structures_snapshot: structures.snapshot_state(),
        pipe_layout_snapshot: pipe_layout.snapshot_state(),
        pipe_gas_snapshot: pipe_gas.snapshot_state(),
        simulation_step: 0,
    }
}

/// Runs `delete_save` logic.
pub fn delete_save(root: &Path, save_id: &str) -> Result<(), SaveError> {
    let slot_dir = root.join(save_id);
    if !slot_dir.exists() || !slot_dir.is_dir() {
        return Err(SaveError::Validation(format!(
            "Save slot '{}' does not exist",
            save_id
        )));
    }
    let meta_path = slot_dir.join(META_FILE);
    if !meta_path.exists() {
        return Err(SaveError::Validation(format!(
            "Save slot '{}' has no meta file",
            save_id
        )));
    }
    fs::remove_dir_all(&slot_dir).map_err(|err| {
        SaveError::Io(format!(
            "Failed to delete save slot '{}': {}",
            slot_dir.display(),
            err
        ))
    })
}

