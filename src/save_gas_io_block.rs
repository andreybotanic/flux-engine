fn write_gas_chunk(
    path: &Path,
    simulation_step: u64,
    gas_ids: &[String],
    snapshot: &GasFieldSnapshot,
) -> Result<(), SaveError> {
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if snapshot.gas_count != gas_ids.len() {
        return Err(SaveError::Validation(format!(
            "Gas id count mismatch while writing gas chunk: ids={}, snapshot={}",
            gas_ids.len(),
            snapshot.gas_count
        )));
    }
    if snapshot.species.len() != cells * snapshot.gas_count {
        return Err(SaveError::Validation(format!(
            "Gas species length mismatch while writing gas chunk: got {}, expected {}",
            snapshot.species.len(),
            cells * snapshot.gas_count
        )));
    }
    if snapshot.velocity.len() != cells || snapshot.total_density.len() != cells {
        return Err(SaveError::Validation(
            "Gas snapshot buffer length mismatch while writing gas chunk".to_string(),
        ));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(GAS_STATE_MAGIC);
    bytes.extend_from_slice(&GAS_STATE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&simulation_step.to_le_bytes());
    bytes.extend_from_slice(&(gas_ids.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(cells as u32).to_le_bytes());

    for gas_id in gas_ids {
        let id_bytes = gas_id.as_bytes();
        let id_len = u16::try_from(id_bytes.len()).map_err(|_| {
            SaveError::Validation(format!(
                "Gas id '{}' is too long for gas chunk format",
                gas_id
            ))
        })?;
        bytes.extend_from_slice(&id_len.to_le_bytes());
        bytes.extend_from_slice(id_bytes);
    }

    for value in &snapshot.species {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for [vx, vy] in &snapshot.velocity {
        bytes.extend_from_slice(&vx.to_le_bytes());
        bytes.extend_from_slice(&vy.to_le_bytes());
    }
    for value in &snapshot.total_density {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_gas_chunk(path: &Path) -> Result<SavedGasChunk, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());

    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != GAS_STATE_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid gas chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != GAS_STATE_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported gas chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let simulation_step = read_u64(&mut cursor)?;
    let gas_count = read_u32(&mut cursor)? as usize;
    let cells = read_u32(&mut cursor)? as usize;
    let expected_cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if cells != expected_cells {
        return Err(SaveError::Validation(format!(
            "Gas chunk cell count mismatch in '{}': got {}, expected {}",
            path.display(),
            cells,
            expected_cells
        )));
    }
    if gas_count == 0 {
        return Err(SaveError::Validation(format!(
            "Gas chunk '{}' contains zero gases",
            path.display()
        )));
    }

    let mut gas_ids = Vec::with_capacity(gas_count);
    for _ in 0..gas_count {
        let id_len = read_u16(&mut cursor)? as usize;
        let mut id_bytes = vec![0u8; id_len];
        cursor.read_exact(&mut id_bytes).map_err(|err| {
            SaveError::Parse(format!(
                "Failed to read gas id from '{}' : {}",
                path.display(),
                err
            ))
        })?;
        let id = String::from_utf8(id_bytes).map_err(|err| {
            SaveError::Parse(format!(
                "Gas chunk '{}' contains invalid UTF-8 in gas id: {}",
                path.display(),
                err
            ))
        })?;
        gas_ids.push(id);
    }

    let species_len = cells * gas_count;
    let mut species = Vec::with_capacity(species_len);
    for _ in 0..species_len {
        species.push(read_u32(&mut cursor)?);
    }

    let mut velocity = Vec::with_capacity(cells);
    for _ in 0..cells {
        let vx = read_f32(&mut cursor)?;
        let vy = read_f32(&mut cursor)?;
        velocity.push([vx, vy]);
    }

    let mut total_density = Vec::with_capacity(cells);
    for _ in 0..cells {
        total_density.push(read_f32(&mut cursor)?);
    }

    Ok(SavedGasChunk {
        width,
        height,
        simulation_step,
        gas_ids,
        species,
        velocity,
        total_density,
    })
}

fn write_gas_structures_chunk(
    path: &Path,
    snapshot: &GasStructureSnapshot,
) -> Result<(), SaveError> {
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if snapshot.kinds.len() != cells
        || snapshot.gas_indices.len() != cells
        || snapshot.amounts.len() != cells
    {
        return Err(SaveError::Validation(format!(
            "Gas structures snapshot length mismatch while writing chunk: kinds={}, gas_indices={}, amounts={}, expected={}",
            snapshot.kinds.len(),
            snapshot.gas_indices.len(),
            snapshot.amounts.len(),
            cells
        )));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(GAS_STRUCTURES_MAGIC);
    bytes.extend_from_slice(&GAS_STRUCTURES_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(cells as u32).to_le_bytes());
    for idx in 0..cells {
        bytes.push(snapshot.kinds[idx]);
        bytes.extend_from_slice(&snapshot.gas_indices[idx].to_le_bytes());
        bytes.extend_from_slice(&snapshot.amounts[idx].to_le_bytes());
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_gas_structures_chunk(path: &Path) -> Result<GasStructureSnapshot, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());

    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != GAS_STRUCTURES_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid gas structures chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != GAS_STRUCTURES_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported gas structures chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let cells = read_u32(&mut cursor)? as usize;
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT || cells != expected {
        return Err(SaveError::Validation(format!(
            "Gas structures chunk dimensions mismatch in '{}': got {}x{} cells={}, expected {}x{} cells={}",
            path.display(),
            width,
            height,
            cells,
            WORLD_WIDTH,
            WORLD_HEIGHT,
            expected
        )));
    }

    let mut kinds = Vec::with_capacity(cells);
    let mut gas_indices = Vec::with_capacity(cells);
    let mut amounts = Vec::with_capacity(cells);
    for _ in 0..cells {
        let kind = read_exact_array::<1>(&mut cursor)?[0];
        let gas_index = read_u32(&mut cursor)?;
        let amount = read_u32(&mut cursor)?;
        kinds.push(kind);
        gas_indices.push(gas_index);
        amounts.push(amount);
    }

    Ok(GasStructureSnapshot {
        kinds,
        gas_indices,
        amounts,
    })
}

fn map_saved_gas_snapshot_to_registry(
    saved: &SavedGasChunk,
    gas_registry: &GasRegistry,
) -> Result<GasFieldSnapshot, SaveError> {
    let mut seen = HashSet::new();
    for gas_id in &saved.gas_ids {
        if !seen.insert(gas_id.clone()) {
            return Err(SaveError::Validation(format!(
                "Gas chunk contains duplicate gas id '{}'",
                gas_id
            )));
        }
        if gas_registry.index_of(gas_id).is_none() {
            return Err(SaveError::Validation(format!(
                "Gas chunk references unknown gas id '{}'",
                gas_id
            )));
        }
    }

    let current_gas_count = gas_registry.count();
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    let mut mapped_species = vec![0u32; cells * current_gas_count];
    for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
        let current_idx = gas_registry.index_of(gas_id).ok_or_else(|| {
            SaveError::Validation(format!("Gas chunk references unknown gas id '{}'", gas_id))
        })?;
        for cell in 0..cells {
            let saved_value = saved.species[cell * saved.gas_ids.len() + saved_idx];
            let mapped_index = cell * current_gas_count + current_idx;
            mapped_species[mapped_index] = saved_value;
        }
    }

    Ok(GasFieldSnapshot {
        gas_count: current_gas_count,
        species: mapped_species,
        total_density: saved
            .total_density
            .iter()
            .copied()
            .map(|value| value.max(0.0))
            .collect(),
        velocity: saved.velocity.clone(),
    })
}

fn write_pipe_layout_chunk(
    path: &Path,
    snapshot: &crate::world::pipes::PipeLayoutSnapshot,
) -> Result<(), SaveError> {
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if snapshot.cells.len() != expected {
        return Err(SaveError::Validation(format!(
            "Pipe layout snapshot length mismatch while writing chunk: got {}, expected {}",
            snapshot.cells.len(),
            expected
        )));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(PIPE_LAYOUT_MAGIC);
    bytes.extend_from_slice(&PIPE_LAYOUT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(expected as u32).to_le_bytes());
    bytes.extend_from_slice(&snapshot.cells);

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_pipe_layout_chunk(path: &Path) -> Result<crate::world::pipes::PipeLayoutSnapshot, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());
    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != PIPE_LAYOUT_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid pipe layout chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != PIPE_LAYOUT_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported pipe layout chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let count = read_u32(&mut cursor)? as usize;
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT || count != expected {
        return Err(SaveError::Validation(format!(
            "Pipe layout chunk dimensions mismatch in '{}': got {}x{} cells={}, expected {}x{} cells={}",
            path.display(),
            width,
            height,
            count,
            WORLD_WIDTH,
            WORLD_HEIGHT,
            expected
        )));
    }
    let mut cells = vec![0u8; count];
    cursor.read_exact(&mut cells).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to read pipe layout payload '{}': {}",
            path.display(),
            err
        ))
    })?;
    Ok(crate::world::pipes::PipeLayoutSnapshot { cells })
}

fn write_pipe_gas_chunk(
    path: &Path,
    gas_ids: &[String],
    snapshot: &crate::simulation::pipes::PipeGasSnapshot,
) -> Result<(), SaveError> {
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if snapshot.gas_count != gas_ids.len() {
        return Err(SaveError::Validation(format!(
            "Pipe gas id count mismatch while writing chunk: ids={}, snapshot={}",
            gas_ids.len(),
            snapshot.gas_count
        )));
    }
    if snapshot.species.len() != cells * snapshot.gas_count {
        return Err(SaveError::Validation(format!(
            "Pipe gas species length mismatch while writing chunk: got {}, expected {}",
            snapshot.species.len(),
            cells * snapshot.gas_count
        )));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(PIPE_GAS_MAGIC);
    bytes.extend_from_slice(&PIPE_GAS_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(gas_ids.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(cells as u32).to_le_bytes());
    for gas_id in gas_ids {
        let id_bytes = gas_id.as_bytes();
        let id_len = u16::try_from(id_bytes.len()).map_err(|_| {
            SaveError::Validation(format!(
                "Gas id '{}' is too long for pipe gas chunk format",
                gas_id
            ))
        })?;
        bytes.extend_from_slice(&id_len.to_le_bytes());
        bytes.extend_from_slice(id_bytes);
    }
    for value in &snapshot.species {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_pipe_gas_chunk(path: &Path, gas_registry: &GasRegistry) -> Result<crate::simulation::pipes::PipeGasSnapshot, SaveError> {
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
    let cells = read_u32(&mut cursor)? as usize;
    let expected_cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT || cells != expected_cells {
        return Err(SaveError::Validation(format!(
            "Pipe gas chunk dimensions mismatch in '{}': got {}x{} cells={}, expected {}x{} cells={}",
            path.display(),
            width,
            height,
            cells,
            WORLD_WIDTH,
            WORLD_HEIGHT,
            expected_cells
        )));
    }

    let mut gas_ids = Vec::with_capacity(gas_count);
    for _ in 0..gas_count {
        let id_len = read_u16(&mut cursor)? as usize;
        let mut id_bytes = vec![0u8; id_len];
        cursor.read_exact(&mut id_bytes).map_err(|err| {
            SaveError::Parse(format!(
                "Failed to read pipe gas id from '{}' : {}",
                path.display(),
                err
            ))
        })?;
        let id = String::from_utf8(id_bytes).map_err(|err| {
            SaveError::Parse(format!(
                "Pipe gas chunk '{}' contains invalid UTF-8 in gas id: {}",
                path.display(),
                err
            ))
        })?;
        gas_ids.push(id);
    }

    let species_len = cells * gas_count;
    let mut species = Vec::with_capacity(species_len);
    for _ in 0..species_len {
        species.push(read_u32(&mut cursor)?);
    }

    let mapped = map_saved_pipe_gas_snapshot_to_registry(
        &SavedPipeGasChunk { gas_ids, species },
        gas_registry,
    )?;
    Ok(mapped)
}

fn map_saved_pipe_gas_snapshot_to_registry(
    saved: &SavedPipeGasChunk,
    gas_registry: &GasRegistry,
) -> Result<crate::simulation::pipes::PipeGasSnapshot, SaveError> {
    let mut seen = HashSet::new();
    for gas_id in &saved.gas_ids {
        if !seen.insert(gas_id.clone()) {
            return Err(SaveError::Validation(format!(
                "Pipe gas chunk contains duplicate gas id '{}'",
                gas_id
            )));
        }
        if gas_registry.index_of(gas_id).is_none() {
            return Err(SaveError::Validation(format!(
                "Pipe gas chunk references unknown gas id '{}'",
                gas_id
            )));
        }
    }

    let current_gas_count = gas_registry.count();
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    let mut mapped_species = vec![0u32; cells * current_gas_count];
    for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
        let current_idx = gas_registry.index_of(gas_id).ok_or_else(|| {
            SaveError::Validation(format!("Pipe gas chunk references unknown gas id '{}'", gas_id))
        })?;
        for cell in 0..cells {
            let saved_value = saved.species[cell * saved.gas_ids.len() + saved_idx];
            let mapped_index = cell * current_gas_count + current_idx;
            mapped_species[mapped_index] = saved_value;
        }
    }

    Ok(crate::simulation::pipes::PipeGasSnapshot {
        gas_count: current_gas_count,
        species: mapped_species,
    })
}

fn read_exact_array<const N: usize>(cursor: &mut Cursor<&[u8]>) -> Result<[u8; N], SaveError> {
    let mut buf = [0u8; N];
    cursor
        .read_exact(&mut buf)
        .map_err(|err| SaveError::Parse(format!("Failed to read binary payload: {}", err)))?;
    Ok(buf)
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, SaveError> {
    Ok(u16::from_le_bytes(read_exact_array::<2>(cursor)?))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, SaveError> {
    Ok(u32::from_le_bytes(read_exact_array::<4>(cursor)?))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, SaveError> {
    Ok(u64::from_le_bytes(read_exact_array::<8>(cursor)?))
}

fn read_f32(cursor: &mut Cursor<&[u8]>) -> Result<f32, SaveError> {
    Ok(f32::from_le_bytes(read_exact_array::<4>(cursor)?))
}

