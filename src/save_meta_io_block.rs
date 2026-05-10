#[derive(Serialize, Deserialize)]
struct SaveChunkMetaToml {
    id: String,
    file: String,
    format: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SaveRequiredContentItemToml {
    kind: String,
    id: String,
    plugin_id: String,
}

#[derive(Serialize, Deserialize)]
struct SaveMetaToml {
    schema_version: u32,
    save_id: String,
    display_name: String,
    created_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    world_width: u32,
    world_height: u32,
    #[serde(default)]
    required_content: Vec<SaveRequiredContentItemToml>,
    #[serde(default)]
    enabled_plugins_at_save: Vec<String>,
    chunks: Vec<SaveChunkMetaToml>,
}

struct SavedGasChunk {
    width: u32,
    height: u32,
    simulation_step: u64,
    gas_ids: Vec<String>,
    species: Vec<u32>,
    velocity: Vec<[f32; 2]>,
    total_density: Vec<f32>,
}

struct SavedPipeGasChunk {
    gas_ids: Vec<String>,
    nodes: Vec<(PipeContainerKind, UVec2, Vec<u32>)>,
}

fn validate_display_name(display_name: &str) -> Result<(), SaveError> {
    let len = display_name.chars().count();
    if len == 0 {
        return Err(SaveError::Validation(
            "Save display name must not be empty".to_string(),
        ));
    }
    if len > 64 {
        return Err(SaveError::Validation(format!(
            "Save display name is too long: {} chars (max 64)",
            len
        )));
    }
    Ok(())
}

fn generate_save_id() -> String {
    let seq = SAVE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("save_{}_{}_{}", now, std::process::id(), seq)
}

fn now_unix_ms() -> Result<i64, SaveError> {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| SaveError::Validation(format!("System clock error: {}", err)))?
        .as_millis();
    i64::try_from(ms).map_err(|_| SaveError::Validation("Unix ms timestamp overflow".to_string()))
}

fn read_meta(path: &Path) -> Result<SaveMetaToml, SaveError> {
    let content = fs::read_to_string(path).map_err(|err| {
        SaveError::Io(format!(
            "Failed to read save meta '{}': {}",
            path.display(),
            err
        ))
    })?;
    toml::from_str::<SaveMetaToml>(&content).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to parse save meta '{}': {}",
            path.display(),
            err
        ))
    })
}

fn validate_meta_dimensions(meta: &SaveMetaToml) -> Result<(), SaveError> {
    if meta.schema_version != SCHEMA_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported save schema version {}",
            meta.schema_version
        )));
    }
    if meta.world_width != WORLD_WIDTH || meta.world_height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "Save world dimensions mismatch: got {}x{}, expected {}x{}",
            meta.world_width, meta.world_height, WORLD_WIDTH, WORLD_HEIGHT
        )));
    }
    Ok(())
}

fn preview_path_from_meta(slot_dir: &Path, meta: &SaveMetaToml) -> Option<PathBuf> {
    meta.chunks
        .iter()
        .find(|chunk| chunk.id == CHUNK_PREVIEW_PNG_ID)
        .map(|chunk| slot_dir.join(&chunk.file))
        .filter(|path| path.exists())
}

fn write_slot(
    root: &Path,
    descriptor: &SaveDescriptor,
    world: &WorldGrid,
    gas: &GasField,
    structures: &PlacedStructureMap,
    pipe_gas: &crate::plugins::default_plugin::pipe_runtime::PipeGasField,
    gas_registry: &GasRegistry,
    content_registry: &ContentRegistry,
    enabled_plugins: &EnabledPluginSet,
    simulation_step: u64,
    allow_overwrite: bool,
) -> Result<(), SaveError> {
    let slot_dir = root.join(&descriptor.id);
    if slot_dir.exists() && !allow_overwrite {
        return Err(SaveError::Validation(format!(
            "Save slot '{}' already exists",
            descriptor.id
        )));
    }

    let chunk_meta = vec![
        SaveChunkMetaToml {
            id: CHUNK_WORLD_CELLS_ID.to_string(),
            file: WORLD_CELLS_FILE.to_string(),
            format: "binary_v2".to_string(),
        },
        SaveChunkMetaToml {
            id: CHUNK_GAS_STATE_ID.to_string(),
            file: GAS_STATE_FILE.to_string(),
            format: "binary_v2".to_string(),
        },
        SaveChunkMetaToml {
            id: CHUNK_PLACED_STRUCTURES_ID.to_string(),
            file: PLACED_STRUCTURES_FILE.to_string(),
            format: "binary_v2".to_string(),
        },
        SaveChunkMetaToml {
            id: CHUNK_PIPE_GAS_ID.to_string(),
            file: PIPE_GAS_FILE.to_string(),
            format: "binary_v2".to_string(),
        },
        SaveChunkMetaToml {
            id: CHUNK_PREVIEW_PNG_ID.to_string(),
            file: PREVIEW_PNG_FILE.to_string(),
            format: "png_v1".to_string(),
        },
    ];

    let world_cells = world.snapshot_cells();
    let gas_snapshot = gas.snapshot_state();
    let structures_snapshot = structures.snapshot_state();
    let pipe_gas_snapshot = pipe_gas.snapshot_state();
    let gas_ids = gas_registry.stable_ids();
    if gas_ids.len() != gas_snapshot.gas_count {
        return Err(SaveError::Validation(format!(
            "Gas registry count ({}) does not match gas snapshot count ({})",
            gas_ids.len(),
            gas_snapshot.gas_count
        )));
    }
    let required_content = collect_required_content(
        &world_cells,
        &gas_snapshot,
        &structures_snapshot,
        &pipe_gas_snapshot,
        gas_registry,
        content_registry,
    )?;
    let enabled_plugins_at_save = enabled_plugins
        .iter()
        .map(|plugin_id| plugin_id.as_str().to_string())
        .collect::<Vec<_>>();

    let meta = SaveMetaToml {
        schema_version: SCHEMA_VERSION,
        save_id: descriptor.id.clone(),
        display_name: descriptor.display_name.clone(),
        created_at_unix_ms: descriptor.created_at_unix_ms,
        updated_at_unix_ms: descriptor.updated_at_unix_ms,
        world_width: WORLD_WIDTH,
        world_height: WORLD_HEIGHT,
        required_content,
        enabled_plugins_at_save,
        chunks: chunk_meta,
    };

    let tmp_dir = root.join(format!(
        ".tmp_{}_{}_{}",
        descriptor.id,
        std::process::id(),
        SAVE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).map_err(|err| {
            SaveError::Io(format!(
                "Failed to remove stale temp save directory '{}': {}",
                tmp_dir.display(),
                err
            ))
        })?;
    }
    fs::create_dir_all(&tmp_dir).map_err(|err| {
        SaveError::Io(format!(
            "Failed to create temp save directory '{}': {}",
            tmp_dir.display(),
            err
        ))
    })?;

    let result = (|| -> Result<(), SaveError> {
        write_meta(&tmp_dir.join(META_FILE), &meta)?;
        write_world_cells_chunk(&tmp_dir.join(WORLD_CELLS_FILE), &world_cells, content_registry)?;
        write_gas_chunk(
            &tmp_dir.join(GAS_STATE_FILE),
            simulation_step,
            &gas_ids,
            &gas_snapshot,
        )?;
        write_placed_structures_chunk(
            &tmp_dir.join(PLACED_STRUCTURES_FILE),
            &structures_snapshot,
            gas_registry,
            content_registry,
        )?;
        write_pipe_gas_chunk(
            &tmp_dir.join(PIPE_GAS_FILE),
            &gas_ids,
            &pipe_gas_snapshot,
            content_registry,
        )?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return result;
    }

    if !slot_dir.exists() {
        fs::rename(&tmp_dir, &slot_dir).map_err(|err| {
            SaveError::Io(format!(
                "Failed to commit save '{}' from '{}' to '{}': {}",
                descriptor.id,
                tmp_dir.display(),
                slot_dir.display(),
                err
            ))
        })?;
        return Ok(());
    }

    let backup_dir = root.join(format!(
        ".bak_{}_{}_{}",
        descriptor.id,
        std::process::id(),
        SAVE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).map_err(|err| {
            SaveError::Io(format!(
                "Failed to remove stale backup save directory '{}': {}",
                backup_dir.display(),
                err
            ))
        })?;
    }

    fs::rename(&slot_dir, &backup_dir).map_err(|err| {
        SaveError::Io(format!(
            "Failed to move existing save '{}' to backup '{}' before overwrite: {}",
            slot_dir.display(),
            backup_dir.display(),
            err
        ))
    })?;

    match fs::rename(&tmp_dir, &slot_dir) {
        Ok(_) => {
            let _ = fs::remove_dir_all(&backup_dir);
        }
        Err(commit_err) => {
            let restore_result = fs::rename(&backup_dir, &slot_dir);
            let _ = fs::remove_dir_all(&tmp_dir);
            return match restore_result {
                Ok(_) => Err(SaveError::Io(format!(
                    "Failed to commit overwrite for save '{}' from '{}' to '{}': {}",
                    descriptor.id,
                    tmp_dir.display(),
                    slot_dir.display(),
                    commit_err
                ))),
                Err(restore_err) => Err(SaveError::Io(format!(
                    "Failed to commit overwrite for save '{}' ({}) and failed to restore backup '{}': {}",
                    descriptor.id,
                    commit_err,
                    backup_dir.display(),
                    restore_err
                ))),
            };
        }
    }

    Ok(())
}

fn write_meta(path: &Path, meta: &SaveMetaToml) -> Result<(), SaveError> {
    let text = toml::to_string_pretty(meta).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to encode save meta for '{}': {}",
            path.display(),
            err
        ))
    })?;
    fs::write(path, text).map_err(|err| {
        SaveError::Io(format!(
            "Failed to write save meta '{}': {}",
            path.display(),
            err
        ))
    })
}

fn write_world_cells_chunk(
    path: &Path,
    cells: &[CellKind],
    content_registry: &ContentRegistry,
) -> Result<(), SaveError> {
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if cells.len() != expected {
        return Err(SaveError::Validation(format!(
            "World cell snapshot length mismatch while writing: got {}, expected {}",
            cells.len(),
            expected
        )));
    }

    let mut ids = BTreeSet::<ContentId>::new();
    for cell in cells {
        let CellKind::Solid(material) = cell else {
            continue;
        };
        let descriptor = content_registry.cell_by_material(*material).ok_or_else(|| {
            SaveError::Validation(format!(
                "Cannot write unknown world cell content id '{}'",
                material.as_str()
            ))
        })?;
        ids.insert(descriptor.id.clone());
    }
    let id_to_index = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let index = u16::try_from(index + 1).map_err(|_| {
                SaveError::Validation("World cell content table is too large".to_string())
            })?;
            Ok((id.clone(), index))
        })
        .collect::<Result<BTreeMap<_, _>, SaveError>>()?;
    let mut encoded_cells = Vec::with_capacity(cells.len());
    for cell in cells {
        let CellKind::Solid(material) = cell else {
            encoded_cells.push(0u16);
            continue;
        };
        let descriptor = content_registry
            .cell_by_material(*material)
            .expect("cell descriptor was validated above");
        encoded_cells.push(
            *id_to_index
                .get(&descriptor.id)
                .expect("cell id has assigned save table index"),
        );
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(WORLD_CELLS_MAGIC);
    bytes.extend_from_slice(&WORLD_CELLS_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(ids.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(encoded_cells.len() as u32).to_le_bytes());
    for id in ids.iter() {
        write_string_to_bytes(&mut bytes, id.as_str())?;
    }
    for index in encoded_cells {
        bytes.extend_from_slice(&index.to_le_bytes());
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_world_cells_chunk(
    path: &Path,
    content_registry: &ContentRegistry,
) -> Result<Vec<CellKind>, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());
    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != WORLD_CELLS_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid world chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != WORLD_CELLS_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported world chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let table_count = read_u32(&mut cursor)? as usize;
    let count = read_u32(&mut cursor)? as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "World chunk dimensions mismatch in '{}': got {}x{}, expected {}x{}",
            path.display(),
            width,
            height,
            WORLD_WIDTH,
            WORLD_HEIGHT
        )));
    }
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if count != expected {
        return Err(SaveError::Validation(format!(
            "World chunk cell count mismatch in '{}': got {}, expected {}",
            path.display(),
            count,
            expected
        )));
    }

    let mut table = Vec::with_capacity(table_count);
    for _ in 0..table_count {
        let raw_id = read_string_from_cursor(&mut cursor, path, "world cell content id")?;
        let id = ContentId::parse(&raw_id).map_err(|err| {
            SaveError::Validation(format!(
                "World chunk '{}' contains invalid cell content id '{}': {}",
                path.display(),
                raw_id,
                err
            ))
        })?;
        let descriptor = content_registry.cells().get(&id).ok_or_else(|| {
            SaveError::Validation(format!("Missing content: {}", id.as_str()))
        })?;
        table.push(descriptor.material);
    }

    let mut cells = Vec::with_capacity(count);
    for index in 0..count {
        let raw = read_u16(&mut cursor)?;
        if raw == 0 {
            cells.push(CellKind::Empty);
            continue;
        }
        let material = table.get((raw - 1) as usize).copied().ok_or_else(|| {
            SaveError::Validation(format!(
                "World chunk '{}' references unknown cell table index {} at cell {}",
                path.display(),
                raw,
                index
            ))
        })?;
        cells.push(CellKind::Solid(material));
    }
    Ok(cells)
}

