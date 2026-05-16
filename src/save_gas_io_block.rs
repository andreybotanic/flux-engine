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

fn write_placed_structures_chunk(
    path: &Path,
    snapshot: &PlacedStructureSnapshot,
    gas_registry: &GasRegistry,
    content_registry: &ContentRegistry,
) -> Result<(), SaveError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PLACED_STRUCTURES_MAGIC);
    bytes.extend_from_slice(&PLACED_STRUCTURES_VERSION.to_le_bytes());
    bytes.extend_from_slice(&(snapshot.entries.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(snapshot.pipe_cuts.len() as u32).to_le_bytes());
    for entry in &snapshot.entries {
        let descriptor = content_registry
            .structure_by_kind(entry.kind)
            .ok_or_else(|| {
                SaveError::Validation(format!(
                    "Cannot write unknown placed structure content id '{}'",
                    entry.kind.as_str()
                ))
            })?;
        write_string_to_bytes(&mut bytes, descriptor.id.as_str())?;
        bytes.extend_from_slice(&entry.origin.x.to_le_bytes());
        bytes.extend_from_slice(&entry.origin.y.to_le_bytes());
        bytes.push(match entry.rotation {
            StructureRotation::Deg0 => 0,
            StructureRotation::Deg90 => 1,
            StructureRotation::Deg180 => 2,
            StructureRotation::Deg270 => 3,
        });
        bytes.extend_from_slice(&entry.state.value().to_le_bytes());
        match entry.params {
            StructureParams::None => {
                bytes.push(0);
            }
            StructureParams::GasSource { gas_index, amount } => {
                bytes.push(1);
                let substance_id = gas_registry.stable_id_by_index(gas_index).ok_or_else(|| {
                    SaveError::Validation(format!(
                        "Cannot write gas source with unknown compact gas index {}",
                        gas_index
                    ))
                })?;
                write_string_to_bytes(&mut bytes, substance_id.as_str())?;
                bytes.extend_from_slice(&amount.to_le_bytes());
            }
            StructureParams::GasSink { amount } => {
                bytes.push(2);
                bytes.extend_from_slice(&amount.to_le_bytes());
            }
        }
    }
    for [a, b] in &snapshot.pipe_cuts {
        bytes.extend_from_slice(&a.x.to_le_bytes());
        bytes.extend_from_slice(&a.y.to_le_bytes());
        bytes.extend_from_slice(&b.x.to_le_bytes());
        bytes.extend_from_slice(&b.y.to_le_bytes());
    }
    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_placed_structures_chunk(
    path: &Path,
    content_registry: &ContentRegistry,
    gas_registry: &GasRegistry,
) -> Result<PlacedStructureSnapshot, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());

    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != PLACED_STRUCTURES_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid placed structures chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != PLACED_STRUCTURES_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported placed structures chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let entry_count = read_u32(&mut cursor)? as usize;
    let cut_count = read_u32(&mut cursor)? as usize;
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let raw_kind = read_string_from_cursor(&mut cursor, path, "placed structure content id")?;
        let content_id = ContentId::parse(&raw_kind).map_err(|err| {
            SaveError::Validation(format!(
                "Placed structures chunk '{}' contains invalid content id '{}': {}",
                path.display(),
                raw_kind,
                err
            ))
        })?;
        let kind = content_registry
            .structures()
            .get(&content_id)
            .ok_or_else(|| SaveError::Validation(format!("Missing content: {}", content_id)))?
            .kind;
        let origin = UVec2::new(read_u32(&mut cursor)?, read_u32(&mut cursor)?);
        let rotation = match read_exact_array::<1>(&mut cursor)?[0] {
            0 => StructureRotation::Deg0,
            1 => StructureRotation::Deg90,
            2 => StructureRotation::Deg180,
            3 => StructureRotation::Deg270,
            value => {
                return Err(SaveError::Validation(format!(
                    "Unknown placed structure rotation {} in '{}'",
                    value,
                    path.display()
                )))
            }
        };
        let state = PackedState(read_u16(&mut cursor)?);
        let params = match read_exact_array::<1>(&mut cursor)?[0] {
            0 => StructureParams::None,
            1 => StructureParams::GasSource {
                gas_index: {
                    let raw_substance =
                        read_string_from_cursor(&mut cursor, path, "gas source substance id")?;
                    gas_registry.index_of(&raw_substance).ok_or_else(|| {
                        SaveError::Validation(format!(
                            "Placed structures chunk references unknown gas source substance id '{}'",
                            raw_substance
                        ))
                    })?
                },
                amount: read_u32(&mut cursor)?,
            },
            2 => StructureParams::GasSink {
                amount: read_u32(&mut cursor)?,
            },
            value => {
                return Err(SaveError::Validation(format!(
                    "Unknown placed structure params {} in '{}'",
                    value,
                    path.display()
                )))
            }
        };
        entries.push(PlacedStructureSnapshotEntry {
            kind,
            origin,
            rotation,
            state,
            params,
        });
    }
    let mut pipe_cuts = Vec::with_capacity(cut_count);
    for _ in 0..cut_count {
        pipe_cuts.push([
            UVec2::new(read_u32(&mut cursor)?, read_u32(&mut cursor)?),
            UVec2::new(read_u32(&mut cursor)?, read_u32(&mut cursor)?),
        ]);
    }
    Ok(PlacedStructureSnapshot { entries, pipe_cuts })
}

fn map_saved_gas_snapshot_to_registry(
    saved: &SavedGasChunk,
    gas_registry: &GasRegistry,
) -> Result<GasFieldSnapshot, SaveError> {
    let mut seen = HashSet::new();
    for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
        if !seen.insert(gas_id.clone()) {
            return Err(SaveError::Validation(format!(
                "Gas chunk contains duplicate gas id '{}'",
                gas_id
            )));
        }
        if gas_registry.index_of(gas_id).is_none() {
            let has_particles = saved
                .species
                .iter()
                .skip(saved_idx)
                .step_by(saved.gas_ids.len())
                .any(|value| *value > 0);
            if has_particles {
                return Err(SaveError::Validation(format!(
                    "Gas chunk references unknown gas id '{}'",
                    gas_id
                )));
            }
        }
    }

    let current_gas_count = gas_registry.count();
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    let mut mapped_species = vec![0u32; cells * current_gas_count];
    for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
        let Some(current_idx) = gas_registry.index_of(gas_id) else {
            continue;
        };
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

fn write_string_to_bytes(bytes: &mut Vec<u8>, value: &str) -> Result<(), SaveError> {
    let value_bytes = value.as_bytes();
    let len = u16::try_from(value_bytes.len()).map_err(|_| {
        SaveError::Validation(format!("String '{}' is too long for save chunk format", value))
    })?;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(value_bytes);
    Ok(())
}

fn read_string_from_cursor(
    cursor: &mut Cursor<&[u8]>,
    path: &Path,
    label: &str,
) -> Result<String, SaveError> {
    let id_len = read_u16(cursor)? as usize;
    let mut id_bytes = vec![0u8; id_len];
    cursor.read_exact(&mut id_bytes).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to read {} from '{}': {}",
            label,
            path.display(),
            err
        ))
    })?;
    String::from_utf8(id_bytes).map_err(|err| {
        SaveError::Parse(format!(
            "Save chunk '{}' contains invalid UTF-8 in {}: {}",
            path.display(),
            label,
            err
        ))
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

