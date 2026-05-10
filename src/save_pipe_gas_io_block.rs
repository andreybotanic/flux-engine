fn write_pipe_gas_chunk(
    path: &Path,
    gas_ids: &[String],
    snapshot: &crate::plugins::default_plugin::pipe_runtime::PipeGasSnapshot,
    content_registry: &ContentRegistry,
) -> Result<(), SaveError> {
    if snapshot.gas_count != gas_ids.len() {
        return Err(SaveError::Validation(format!(
            "Pipe gas id count mismatch while writing chunk: ids={}, snapshot={}",
            gas_ids.len(),
            snapshot.gas_count
        )));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(PIPE_GAS_MAGIC);
    bytes.extend_from_slice(&PIPE_GAS_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(gas_ids.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(snapshot.nodes.len() as u32).to_le_bytes());
    for gas_id in gas_ids {
        write_string_to_bytes(&mut bytes, gas_id)?;
    }
    for node in &snapshot.nodes {
        if node.species.len() != snapshot.gas_count {
            return Err(SaveError::Validation(format!(
                "Pipe node species count mismatch while writing chunk: got {}, expected {}",
                node.species.len(),
                snapshot.gas_count
            )));
        }
        let container_id = pipe_container_content_id(node.key.kind);
        let content_id = ContentId::parse(container_id).map_err(SaveError::Validation)?;
        if !content_registry.structures().contains_key(&content_id) {
            return Err(SaveError::Validation(format!(
                "Cannot write unknown pipe container content id '{}'",
                container_id
            )));
        }
        write_string_to_bytes(&mut bytes, container_id)?;
        bytes.extend_from_slice(&node.key.anchor.x.to_le_bytes());
        bytes.extend_from_slice(&node.key.anchor.y.to_le_bytes());
        for value in &node.species {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_pipe_gas_chunk(
    path: &Path,
    gas_registry: &GasRegistry,
    content_registry: &ContentRegistry,
) -> Result<crate::plugins::default_plugin::pipe_runtime::PipeGasSnapshot, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());
    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != PIPE_GAS_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid pipe gas chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != PIPE_GAS_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported pipe gas chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let gas_count = read_u32(&mut cursor)? as usize;
    let node_count = read_u32(&mut cursor)? as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "Pipe gas chunk dimensions mismatch in '{}': got {}x{}, expected {}x{}",
            path.display(),
            width,
            height,
            WORLD_WIDTH,
            WORLD_HEIGHT,
        )));
    }

    let mut gas_ids = Vec::with_capacity(gas_count);
    for _ in 0..gas_count {
        gas_ids.push(read_string_from_cursor(&mut cursor, path, "pipe gas id")?);
    }

    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        let raw_container =
            read_string_from_cursor(&mut cursor, path, "pipe container content id")?;
        let kind = pipe_container_kind_from_content_id(&raw_container, content_registry)?;
        let anchor = UVec2::new(read_u32(&mut cursor)?, read_u32(&mut cursor)?);
        let mut species = Vec::with_capacity(gas_count);
        for _ in 0..gas_count {
            species.push(read_u32(&mut cursor)?);
        }
        nodes.push((kind, anchor, species));
    }

    let mapped = map_saved_pipe_gas_snapshot_to_registry(
        &SavedPipeGasChunk { gas_ids, nodes },
        gas_registry,
    )?;
    Ok(mapped)
}

fn map_saved_pipe_gas_snapshot_to_registry(
    saved: &SavedPipeGasChunk,
    gas_registry: &GasRegistry,
) -> Result<crate::plugins::default_plugin::pipe_runtime::PipeGasSnapshot, SaveError> {
    let mut seen = HashSet::new();
    for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
        if !seen.insert(gas_id.clone()) {
            return Err(SaveError::Validation(format!(
                "Pipe gas chunk contains duplicate gas id '{}'",
                gas_id
            )));
        }
        if gas_registry.index_of(gas_id).is_none() {
            let has_particles = saved
                .nodes
                .iter()
                .any(|(_, _, species)| species.get(saved_idx).copied().unwrap_or(0) > 0);
            if has_particles {
                return Err(SaveError::Validation(format!(
                    "Pipe gas chunk references unknown gas id '{}'",
                    gas_id
                )));
            }
        }
    }

    let current_gas_count = gas_registry.count();
    let mut mapped_nodes = Vec::with_capacity(saved.nodes.len());
    for (kind, anchor, species) in &saved.nodes {
        let mut mapped_species = vec![0u32; current_gas_count];
        for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
            let Some(current_idx) = gas_registry.index_of(gas_id) else {
                continue;
            };
            mapped_species[current_idx] = species.get(saved_idx).copied().unwrap_or(0);
        }
        mapped_nodes.push(PipeNodeGasSnapshotEntry {
            key: PipeNodeKey {
                kind: *kind,
                anchor: *anchor,
            },
            species: mapped_species,
        });
    }

    Ok(crate::plugins::default_plugin::pipe_runtime::PipeGasSnapshot {
        gas_count: current_gas_count,
        nodes: mapped_nodes,
    })
}

fn pipe_container_kind_from_content_id(
    raw_id: &str,
    content_registry: &ContentRegistry,
) -> Result<PipeContainerKind, SaveError> {
    let id = ContentId::parse(raw_id).map_err(|err| {
        SaveError::Validation(format!(
            "Pipe gas chunk contains invalid pipe container id '{}': {}",
            raw_id, err
        ))
    })?;
    if !content_registry.structures().contains_key(&id) {
        return Err(SaveError::Validation(format!(
            "Missing content: {}",
            id.as_str()
        )));
    }
    match id.as_str() {
        crate::plugins::default_plugin::ENTITY_PIPE_ID => Ok(PipeContainerKind::Pipe),
        crate::plugins::default_plugin::ENTITY_GAS_PIPE_BRIDGE_ID => {
            Ok(PipeContainerKind::BridgePipe)
        }
        _ => Err(SaveError::Validation(format!(
            "Unsupported pipe container content id '{}'",
            id.as_str()
        ))),
    }
}
