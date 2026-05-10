#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RequiredContentKind {
    Cell,
    Entity,
    Substance,
    PipeContainer,
}

impl RequiredContentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cell => "cell",
            Self::Entity => "entity",
            Self::Substance => "substance",
            Self::PipeContainer => "pipe_container",
        }
    }

    fn parse(raw: &str) -> Result<Self, SaveError> {
        match raw {
            "cell" => Ok(Self::Cell),
            "entity" => Ok(Self::Entity),
            "substance" => Ok(Self::Substance),
            "pipe_container" => Ok(Self::PipeContainer),
            value => Err(SaveError::Validation(format!(
                "Unknown required content kind '{}'",
                value
            ))),
        }
    }
}

fn collect_required_content(
    world_cells: &[CellKind],
    gas_snapshot: &GasFieldSnapshot,
    structures_snapshot: &PlacedStructureSnapshot,
    pipe_gas_snapshot: &PipeGasSnapshot,
    gas_registry: &GasRegistry,
    content_registry: &ContentRegistry,
) -> Result<Vec<SaveRequiredContentItemToml>, SaveError> {
    let mut items = BTreeSet::<(RequiredContentKind, String, String)>::new();

    for cell in world_cells {
        let CellKind::Solid(material) = cell else {
            continue;
        };
        let descriptor = content_registry.cell_by_material(*material).ok_or_else(|| {
            SaveError::Validation(format!(
                "Cannot collect required content for unknown cell '{}'",
                material.as_str()
            ))
        })?;
        insert_required_content(
            &mut items,
            RequiredContentKind::Cell,
            descriptor.id.as_str(),
            &descriptor.plugin_id,
        );
    }

    for entry in &structures_snapshot.entries {
        let descriptor = content_registry
            .structure_by_kind(entry.kind)
            .ok_or_else(|| {
                SaveError::Validation(format!(
                    "Cannot collect required content for unknown structure '{}'",
                    entry.kind.as_str()
                ))
            })?;
        insert_required_content(
            &mut items,
            RequiredContentKind::Entity,
            descriptor.id.as_str(),
            &descriptor.plugin_id,
        );
        if let StructureParams::GasSource { gas_index, .. } = entry.params {
            let substance_id = gas_registry.stable_id_by_index(gas_index).ok_or_else(|| {
                SaveError::Validation(format!(
                    "Gas source references unknown compact gas index {}",
                    gas_index
                ))
            })?;
            insert_required_substance(&mut items, gas_registry, substance_id)?;
        }
    }

    collect_nonzero_substances_from_world_gas(&mut items, gas_snapshot, gas_registry)?;
    collect_nonzero_substances_from_pipe_gas(&mut items, pipe_gas_snapshot, gas_registry)?;

    for node in &pipe_gas_snapshot.nodes {
        let id = pipe_container_content_id(node.key.kind);
        let content_id = ContentId::parse(id).map_err(SaveError::Validation)?;
        let descriptor = content_registry.structures().get(&content_id).ok_or_else(|| {
            SaveError::Validation(format!(
                "Cannot collect required content for unknown pipe container '{}'",
                id
            ))
        })?;
        insert_required_content(
            &mut items,
            RequiredContentKind::PipeContainer,
            descriptor.id.as_str(),
            &descriptor.plugin_id,
        );
    }

    Ok(items
        .into_iter()
        .map(|(kind, id, plugin_id)| SaveRequiredContentItemToml {
            kind: kind.as_str().to_string(),
            id,
            plugin_id,
        })
        .collect())
}

fn validate_world_content_available(
    meta: &SaveMetaToml,
    content_registry: &ContentRegistry,
) -> Result<(), SaveError> {
    let mut missing_plugins = BTreeSet::<String>::new();
    let mut missing_content = BTreeSet::<String>::new();

    for item in &meta.required_content {
        let kind = RequiredContentKind::parse(&item.kind)?;
        let plugin_id = PluginId::parse(&item.plugin_id).map_err(|err| {
            SaveError::Validation(format!(
                "Save meta contains invalid required plugin id '{}': {}",
                item.plugin_id, err
            ))
        })?;
        if !content_registry.provider_plugins().contains(&plugin_id) {
            missing_plugins.insert(item.plugin_id.clone());
            continue;
        }
        if !required_content_registered(kind, &item.id, content_registry)? {
            missing_content.insert(item.id.clone());
        }
    }

    if missing_plugins.is_empty() && missing_content.is_empty() {
        return Ok(());
    }

    let mut parts = Vec::new();
    parts.extend(
        missing_plugins
            .into_iter()
            .map(|plugin_id| format!("Missing plugin: {}", plugin_id)),
    );
    parts.extend(
        missing_content
            .into_iter()
            .map(|content_id| format!("Missing content: {}", content_id)),
    );
    Err(SaveError::Validation(parts.join("; ")))
}

fn collect_nonzero_substances_from_world_gas(
    items: &mut BTreeSet<(RequiredContentKind, String, String)>,
    snapshot: &GasFieldSnapshot,
    gas_registry: &GasRegistry,
) -> Result<(), SaveError> {
    for gas_index in 0..snapshot.gas_count {
        if !snapshot
            .species
            .iter()
            .skip(gas_index)
            .step_by(snapshot.gas_count)
            .any(|value| *value > 0)
        {
            continue;
        }
        let substance_id = gas_registry.stable_id_by_index(gas_index).ok_or_else(|| {
            SaveError::Validation(format!(
                "World gas snapshot references unknown compact gas index {}",
                gas_index
            ))
        })?;
        insert_required_substance(items, gas_registry, substance_id)?;
    }
    Ok(())
}

fn collect_nonzero_substances_from_pipe_gas(
    items: &mut BTreeSet<(RequiredContentKind, String, String)>,
    snapshot: &PipeGasSnapshot,
    gas_registry: &GasRegistry,
) -> Result<(), SaveError> {
    for gas_index in 0..snapshot.gas_count {
        if !snapshot
            .nodes
            .iter()
            .any(|node| node.species.get(gas_index).copied().unwrap_or(0) > 0)
        {
            continue;
        }
        let substance_id = gas_registry.stable_id_by_index(gas_index).ok_or_else(|| {
            SaveError::Validation(format!(
                "Pipe gas snapshot references unknown compact gas index {}",
                gas_index
            ))
        })?;
        insert_required_substance(items, gas_registry, substance_id)?;
    }
    Ok(())
}

fn insert_required_substance(
    items: &mut BTreeSet<(RequiredContentKind, String, String)>,
    gas_registry: &GasRegistry,
    substance_id: &SubstanceId,
) -> Result<(), SaveError> {
    let definition = gas_registry
        .substances()
        .get_by_id(substance_id)
        .ok_or_else(|| {
            SaveError::Validation(format!(
                "Gas registry is missing substance definition '{}'",
                substance_id
            ))
        })?;
    insert_required_content(
        items,
        RequiredContentKind::Substance,
        definition.id.as_str(),
        &definition.plugin_id,
    );
    Ok(())
}

fn insert_required_content(
    items: &mut BTreeSet<(RequiredContentKind, String, String)>,
    kind: RequiredContentKind,
    id: &str,
    plugin_id: &PluginId,
) {
    items.insert((
        kind,
        id.to_string(),
        plugin_id.as_str().to_string(),
    ));
}

fn required_content_registered(
    kind: RequiredContentKind,
    raw_id: &str,
    content_registry: &ContentRegistry,
) -> Result<bool, SaveError> {
    match kind {
        RequiredContentKind::Cell => {
            let id = ContentId::parse(raw_id).map_err(SaveError::Validation)?;
            Ok(content_registry.cells().contains_key(&id))
        }
        RequiredContentKind::Entity | RequiredContentKind::PipeContainer => {
            let id = ContentId::parse(raw_id).map_err(SaveError::Validation)?;
            Ok(content_registry.structures().contains_key(&id))
        }
        RequiredContentKind::Substance => {
            let id = SubstanceId::parse(raw_id).map_err(SaveError::Validation)?;
            Ok(content_registry.substances().contains_key(&id))
        }
    }
}

fn pipe_container_content_id(kind: PipeContainerKind) -> &'static str {
    match kind {
        PipeContainerKind::Pipe => crate::plugins::default_plugin::ENTITY_PIPE_ID,
        PipeContainerKind::BridgePipe => crate::plugins::default_plugin::ENTITY_GAS_PIPE_BRIDGE_ID,
    }
}
