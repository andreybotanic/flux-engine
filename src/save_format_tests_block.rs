#[test]
fn load_rejects_unsupported_schema_version() {
    let root = temp_saves_root("flux_save_old_schema_rejected");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    let descriptor = create_save(
        &root,
        "old-schema",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        6,
    )
    .expect("save");
    let meta_path = root.join(&descriptor.id).join(META_FILE);
    let mut meta = read_meta(&meta_path).expect("read meta");
    meta.schema_version = SCHEMA_VERSION - 1;
    write_meta(&meta_path, &meta).expect("write downgraded meta");

    let err = load_save(&root, &descriptor.id, &registry).expect_err("must reject old schema");
    assert!(err.to_string().contains("Unsupported save schema version"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_rejects_missing_current_schema_chunks() {
    let root = temp_saves_root("flux_save_missing_current_chunks");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    for missing_chunk in [CHUNK_PLACED_STRUCTURES_ID, CHUNK_PIPE_GAS_ID] {
        let descriptor = create_save(
            &root,
            missing_chunk,
            &world,
            &gas,
            &structures,
            &pipe_gas,
            &registry,
            8,
        )
        .expect("save");
        let meta_path = root.join(&descriptor.id).join(META_FILE);
        let mut meta = read_meta(&meta_path).expect("read meta");
        meta.chunks.retain(|chunk| chunk.id != missing_chunk);
        write_meta(&meta_path, &meta).expect("write patched meta");

        let err = load_save(&root, &descriptor.id, &registry)
            .expect_err("must reject incomplete current-schema save");
        assert!(err.to_string().contains(missing_chunk));
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_rejects_unknown_saved_gas_id() {
    let root = temp_saves_root("flux_save_unknown_gas");
    let registry = test_registry();
    let world = WorldGrid::default();
    let mut gas = GasField::from_registry(&registry);
    let _ = gas.apply_species_delta_with_lbm(50, 50, 0, 7_000.0);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    let descriptor = create_save(
        &root,
        "unknown-gas",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        5,
    )
    .expect("save");
    let gas_path = root.join(&descriptor.id).join(GAS_STATE_FILE);
    patch_first_gas_id(&gas_path, b"zz");

    let err = load_save(&root, &descriptor.id, &registry).expect_err("must fail on unknown gas");
    let message = err.to_string();
    assert!(message.contains("unknown gas id") || message.contains("Gas chunk references"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_ignores_unknown_saved_gas_id_when_that_species_is_empty() {
    let root = temp_saves_root("flux_save_unknown_empty_gas");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);

    let descriptor = create_save(
        &root,
        "unknown-empty-gas",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        5,
    )
    .expect("save");
    let gas_path = root.join(&descriptor.id).join(GAS_STATE_FILE);
    patch_first_gas_id(&gas_path, b"zz");

    let loaded =
        load_save(&root, &descriptor.id, &registry).expect("unknown empty gas id should be ignored");
    assert_eq!(loaded.state.simulation_step, 5);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_rejects_invalid_world_chunk_magic() {
    let root = temp_saves_root("flux_save_bad_world_magic");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);
    let descriptor = create_save(
        &root,
        "bad-world-magic",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        7,
    )
    .expect("save");

    let world_path = root.join(&descriptor.id).join(WORLD_CELLS_FILE);
    let mut bytes = fs::read(&world_path).expect("read world chunk");
    bytes[0] = b'B';
    fs::write(&world_path, bytes).expect("write patched world chunk");

    let err = load_save(&root, &descriptor.id, &registry).expect_err("must fail on bad magic");
    assert!(err.to_string().contains("Invalid world chunk magic"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_rejects_invalid_gas_chunk_version() {
    let root = temp_saves_root("flux_save_bad_gas_version");
    let registry = test_registry();
    let world = WorldGrid::default();
    let gas = GasField::from_registry(&registry);
    let structures = PlacedStructureMap::default();
    let pipe_gas = PipeGasField::from_registry(&registry);
    let descriptor = create_save(
        &root,
        "bad-gas-version",
        &world,
        &gas,
        &structures,
        &pipe_gas,
        &registry,
        11,
    )
    .expect("save");

    let gas_path = root.join(&descriptor.id).join(GAS_STATE_FILE);
    let mut bytes = fs::read(&gas_path).expect("read gas chunk");
    bytes[4] = 0xFF;
    bytes[5] = 0x7F;
    fs::write(&gas_path, bytes).expect("write patched gas chunk");

    let err =
        load_save(&root, &descriptor.id, &registry).expect_err("must fail on bad gas version");
    assert!(err.to_string().contains("Unsupported gas chunk version"));

    let _ = fs::remove_dir_all(root);
}
