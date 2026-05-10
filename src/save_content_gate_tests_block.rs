#[test]
fn save_meta_required_content_contains_only_used_world_content() {
    let root = temp_saves_root("flux_save_required_content");
    let registry = test_registry();
    let mut world = WorldGrid::default();
    assert!(world.set_solid_with_material(
        10,
        10,
        crate::plugins::default_plugin::brick_cell_material()
    ));
    assert!(world.set_solid_with_material(
        11,
        10,
        crate::plugins::default_plugin::metal_cell_material()
    ));

    let mut gas = GasField::from_registry(&registry);
    let _ = gas.apply_species_delta_with_lbm(20, 20, 0, 1_000.0);
    let _ = gas.apply_species_delta_with_lbm(21, 20, 1, 2_000.0);
    let mut structures = PlacedStructureMap::default();
    let mut pipe_gas = PipeGasField::from_registry(&registry);
    assert!(structures.place_gas_source(14, 14, 0, 10, &world).is_some());
    assert!(structures.place_gas_sink(15, 14, 5, &world).is_some());
    assert!(structures.place_pipe(20, 20, &world));
    assert!(structures.place_vent(20, 20, &world));
    pipe_gas.sync_to_structures(&structures);
    pipe_gas.add_species_counts(0, &[50, 0, 0]);

    let descriptor = create_save(
        &root,
        "required-content",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        1,
    )
    .expect("save");
    let meta = read_meta(&root.join(&descriptor.id).join(META_FILE)).expect("read meta");

    let cells = required_ids(&meta, "cell");
    assert!(cells.contains(&CELL_BOUNDARY_ID.to_string()));
    assert!(cells.contains(&CELL_BRICK_ID.to_string()));
    assert!(cells.contains(&CELL_METAL_ID.to_string()));

    let entities = required_ids(&meta, "entity");
    assert!(entities.contains(&ENTITY_GAS_SOURCE_ID.to_string()));
    assert!(entities.contains(&ENTITY_GAS_SINK_ID.to_string()));
    assert!(entities.contains(&ENTITY_PIPE_ID.to_string()));
    assert!(entities.contains(&ENTITY_VENT_ID.to_string()));

    let substances = required_ids(&meta, "substance");
    assert!(substances.contains(&SUBSTANCE_H2_ID.to_string()));
    assert!(substances.contains(&SUBSTANCE_O2_ID.to_string()));
    assert!(!substances.iter().any(|id| id.ends_with(".co2")));

    assert!(required_ids(&meta, "pipe_container").contains(&ENTITY_PIPE_ID.to_string()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn enabled_non_content_plugin_is_diagnostic_only_for_loading() {
    let root = temp_saves_root("flux_save_non_content_diagnostic");
    let registry = test_registry();
    let content_registry = test_content_registry();
    let mut enabled_plugins = default_enabled_plugins();
    let sample_plugin = PluginId::parse("flux.sample_stage1").expect("sample plugin id");
    enabled_plugins.set_enabled(&sample_plugin, true);
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    let descriptor = super::create_save(
        &root,
        "non-content-diagnostic",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        &content_registry,
        &enabled_plugins,
        2,
    )
    .expect("save");
    let meta = read_meta(&root.join(&descriptor.id).join(META_FILE)).expect("read meta");
    assert!(meta
        .enabled_plugins_at_save
        .contains(&sample_plugin.as_str().to_string()));

    let loaded = super::load_save(&root, &descriptor.id, &registry, &content_registry)
        .expect("non-content plugin must not block load");
    assert_eq!(loaded.state.simulation_step, 2);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_gate_reports_missing_plugin_before_reading_chunks() {
    let root = temp_saves_root("flux_save_missing_plugin_gate");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    let descriptor = create_save(
        &root,
        "missing-plugin",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        3,
    )
    .expect("save");
    let meta_path = root.join(&descriptor.id).join(META_FILE);
    let mut meta = read_meta(&meta_path).expect("read meta");
    meta.required_content.push(SaveRequiredContentItemToml {
        kind: "entity".to_string(),
        id: "missing.plugin.entity.magic_pipe".to_string(),
        plugin_id: "missing.plugin".to_string(),
    });
    write_meta(&meta_path, &meta).expect("write patched meta");
    fs::write(root.join(&descriptor.id).join(WORLD_CELLS_FILE), b"bad")
        .expect("corrupt world chunk");

    let err = load_save(&root, &descriptor.id, &registry).expect_err("must block load");
    let message = err.to_string();
    assert!(message.contains("Missing plugin: missing.plugin"));
    assert!(!message.contains("Invalid world chunk magic"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_gate_reports_missing_content_when_provider_is_active() {
    let root = temp_saves_root("flux_save_missing_content_gate");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    let descriptor = create_save(
        &root,
        "missing-content",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        4,
    )
    .expect("save");
    let meta_path = root.join(&descriptor.id).join(META_FILE);
    let mut meta = read_meta(&meta_path).expect("read meta");
    meta.required_content.push(SaveRequiredContentItemToml {
        kind: "entity".to_string(),
        id: "flux.default.entity.magic_pipe".to_string(),
        plugin_id: "flux.default".to_string(),
    });
    write_meta(&meta_path, &meta).expect("write patched meta");

    let err = load_save(&root, &descriptor.id, &registry).expect_err("must block load");
    assert!(err
        .to_string()
        .contains("Missing content: flux.default.entity.magic_pipe"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn gas_source_substance_is_required_even_without_free_particles() {
    let root = temp_saves_root("flux_save_source_substance_required");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let mut structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);
    assert!(structures.place_gas_source(14, 14, 0, 10, &world).is_some());

    let descriptor = create_save(
        &root,
        "source-substance-required",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        5,
    )
    .expect("save");
    let meta = read_meta(&root.join(&descriptor.id).join(META_FILE)).expect("read meta");
    assert!(required_ids(&meta, "substance").contains(&SUBSTANCE_H2_ID.to_string()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn plugin_owned_substance_is_required_content_when_used() {
    let root = temp_saves_root("flux_save_plugin_substance_required");
    let plugin_id = PluginId::parse("flux.sample_content").expect("plugin id");
    let substance_id =
        crate::plugins::SubstanceId::parse("flux.sample_content.substance.neon")
            .expect("substance id");
    let neon = crate::plugins::SubstanceDefinition::gas(
        substance_id.clone(),
        plugin_id.clone(),
        "Neon",
        20.180,
        [1.0, 0.32, 0.78],
        vec!["neon".to_string()],
    )
    .expect("neon substance");

    let mut content_registry = test_content_registry();
    content_registry.register_provider_plugin(plugin_id.clone());
    content_registry.register_substance(neon.clone());
    let mut substances = crate::plugins::default_plugin::default_substance_definitions();
    substances.push(neon);
    let registry = GasRegistry::from_substances(substances).expect("registry with neon");
    let neon_index = registry
        .index_of("flux.sample_content.substance.neon")
        .expect("neon index");
    let mut enabled_plugins = default_enabled_plugins();
    enabled_plugins.set_enabled(&plugin_id, true);

    let world = WorldGrid::default();
    let mut gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);
    let _ = gas.apply_species_delta_with_lbm(20, 20, neon_index, 1_000.0);

    let descriptor = super::create_save(
        &root,
        "plugin-substance-required",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        &content_registry,
        &enabled_plugins,
        6,
    )
    .expect("save");
    let meta_path = root.join(&descriptor.id).join(META_FILE);
    let meta = read_meta(&meta_path).expect("read meta");
    assert!(meta.required_content.iter().any(|item| {
        item.kind == "substance"
            && item.id == substance_id.as_str()
            && item.plugin_id == "flux.sample_content"
    }));

    let missing_err = super::load_save(&root, &descriptor.id, &registry, &test_content_registry())
        .expect_err("load must require content plugin");
    assert!(missing_err
        .to_string()
        .contains("Missing plugin: flux.sample_content"));

    let loaded = super::load_save(&root, &descriptor.id, &registry, &content_registry)
        .expect("load with content plugin");
    assert_eq!(loaded.state.simulation_step, 6);

    let _ = fs::remove_dir_all(root);
}
