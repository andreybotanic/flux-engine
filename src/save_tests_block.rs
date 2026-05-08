#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::GasDefinition,
        simulation::pipes::PipeGasField,
        world::{
            gas_structures::GasStructureGrid,
            grid::{CellMaterial, WorldGrid},
            pipes::PipeGrid,
            structures::PlacedStructureMap,
        },
    };
    fn test_registry() -> GasRegistry {
        GasRegistry::new(vec![
            GasDefinition {
                id: "h2".to_string(),
                label: "Hydrogen".to_string(),
                molecular_mass: 2.016,
                color: [0.8, 0.2, 0.9],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "Oxygen".to_string(),
                molecular_mass: 31.998,
                color: [0.0, 0.85, 0.85],
            },
            GasDefinition {
                id: "co2".to_string(),
                label: "Carbon dioxide".to_string(),
                molecular_mass: 44.009,
                color: [0.5, 0.5, 0.5],
            },
        ])
        .expect("valid registry")
    }

    fn temp_saves_root(prefix: &str) -> PathBuf {
        let name = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(name);
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_preview_png(path: &Path) {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255]));
        image.save(path).expect("write preview png");
    }

    fn write_legacy_schema_slot(root: &Path, schema_version: u32, registry: &GasRegistry) -> String {
        let world = WorldGrid::default();
        let mut gas = GasField::from_registry(registry);
        let _ = gas.apply_species_delta_with_lbm(40, 40, 0, 3_000.0);
        let _ = gas.apply_species_delta_with_lbm(41, 40, 1, 2_000.0);

        let mut legacy_structures = GasStructureGrid::default();
        assert!(legacy_structures.set_source(14, 14, 0, 10, &world));
        assert!(legacy_structures.set_sink(15, 14, 5, &world));

        let mut legacy_pipes = PipeGrid::default();
        if schema_version >= 3 {
            assert!(legacy_pipes.set_pipe(20, 20, &world, &legacy_structures));
            assert!(legacy_pipes.set_pipe(21, 20, &world, &legacy_structures));
            assert!(legacy_pipes.set_vent(20, 20, &world, &legacy_structures));
            assert!(legacy_pipes.add_connection(UVec2::new(20, 20), UVec2::new(21, 20)));
        }

        let mut placed_structures = PlacedStructureMap::default();
        let mut pipe_gas = PipeGasField::from_registry(registry);
        if schema_version >= 4 {
            assert!(placed_structures
                .place_gas_source(14, 14, 0, 10, &world)
                .is_some());
            assert!(placed_structures.place_gas_sink(15, 14, 5, &world).is_some());
            assert!(placed_structures.place_pipe(20, 20, &world));
            assert!(placed_structures.place_pipe(21, 20, &world));
            assert!(placed_structures.place_vent(20, 20, &world));
            pipe_gas.sync_to_structures(&placed_structures);
            pipe_gas.add_species_counts(0, &[70_000, 30_000, 0]);
        }

        let gas_ids = registry
            .all()
            .iter()
            .map(|definition| definition.id.clone())
            .collect::<Vec<_>>();
        let slot_id = format!("legacy_schema_{schema_version}");
        let slot_dir = root.join(&slot_id);
        fs::create_dir_all(&slot_dir).expect("create legacy slot dir");

        let mut chunks = vec![
            SaveChunkMetaToml {
                id: CHUNK_WORLD_CELLS_ID.to_string(),
                file: WORLD_CELLS_FILE.to_string(),
                format: "binary_v1".to_string(),
            },
            SaveChunkMetaToml {
                id: CHUNK_GAS_STATE_ID.to_string(),
                file: GAS_STATE_FILE.to_string(),
                format: "binary_v2".to_string(),
            },
        ];
        match schema_version {
            2 => {
                chunks.push(SaveChunkMetaToml {
                    id: CHUNK_GAS_STRUCTURES_ID.to_string(),
                    file: "gas_structures.bin".to_string(),
                    format: "binary_v1".to_string(),
                });
            }
            3 => {
                chunks.push(SaveChunkMetaToml {
                    id: CHUNK_GAS_STRUCTURES_ID.to_string(),
                    file: "gas_structures.bin".to_string(),
                    format: "binary_v1".to_string(),
                });
                chunks.push(SaveChunkMetaToml {
                    id: CHUNK_PIPE_LAYOUT_ID.to_string(),
                    file: "pipe_layout.bin".to_string(),
                    format: "binary_v1".to_string(),
                });
            }
            4 => {
                chunks.push(SaveChunkMetaToml {
                    id: CHUNK_PLACED_STRUCTURES_ID.to_string(),
                    file: PLACED_STRUCTURES_FILE.to_string(),
                    format: "binary_v1".to_string(),
                });
                chunks.push(SaveChunkMetaToml {
                    id: CHUNK_PIPE_GAS_ID.to_string(),
                    file: PIPE_GAS_FILE.to_string(),
                    format: "binary_v1".to_string(),
                });
            }
            _ => panic!("unsupported legacy schema {schema_version}"),
        }

        write_meta(
            &slot_dir.join(META_FILE),
            &SaveMetaToml {
                schema_version,
                save_id: slot_id.clone(),
                display_name: format!("Legacy schema {schema_version}"),
                created_at_unix_ms: 123,
                updated_at_unix_ms: 456,
                world_width: WORLD_WIDTH,
                world_height: WORLD_HEIGHT,
                chunks,
            },
        )
        .expect("write legacy meta");
        write_world_cells_chunk(&slot_dir.join(WORLD_CELLS_FILE), &world.snapshot_cell_codes())
            .expect("write legacy world");
        write_gas_chunk(
            &slot_dir.join(GAS_STATE_FILE),
            77,
            &gas_ids,
            &gas.snapshot_state(),
        )
        .expect("write legacy gas");
        match schema_version {
            2 => {
                write_gas_structures_chunk(
                    &slot_dir.join("gas_structures.bin"),
                    &legacy_structures.snapshot_state(),
                )
                .expect("write schema2 structures");
            }
            3 => {
                write_gas_structures_chunk(
                    &slot_dir.join("gas_structures.bin"),
                    &legacy_structures.snapshot_state(),
                )
                .expect("write schema3 structures");
                write_pipe_layout_chunk(
                    &slot_dir.join("pipe_layout.bin"),
                    &legacy_pipes.snapshot_state(),
                )
                .expect("write schema3 pipes");
            }
            4 => {
                write_placed_structures_chunk(
                    &slot_dir.join(PLACED_STRUCTURES_FILE),
                    &placed_structures.snapshot_state(),
                )
                .expect("write schema4 structures");
                write_pipe_gas_chunk(
                    &slot_dir.join(PIPE_GAS_FILE),
                    &gas_ids,
                    &pipe_gas.snapshot_state(),
                )
                .expect("write schema4 pipe gas");
            }
            _ => unreachable!(),
        }

        slot_id
    }

    fn write_legacy_dense_pipe_gas_chunk(
        path: &Path,
        gas_ids: &[String],
        pipe_layout: &crate::world::pipes::PipeLayoutSnapshot,
    ) {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(PIPE_GAS_MAGIC);
        bytes.extend_from_slice(&PIPE_GAS_VERSION.to_le_bytes());
        bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
        bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
        bytes.extend_from_slice(&(gas_ids.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(WORLD_WIDTH * WORLD_HEIGHT).to_le_bytes());
        for gas_id in gas_ids {
            let raw = gas_id.as_bytes();
            bytes.extend_from_slice(&(raw.len() as u16).to_le_bytes());
            bytes.extend_from_slice(raw);
        }
        for encoded in &pipe_layout.cells {
            let amount = if (encoded & 0b0001_0000) != 0 { 250_u32 } else { 0_u32 };
            for gas_index in 0..gas_ids.len() {
                let value = if gas_index == 0 { amount } else { 0 };
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        fs::write(path, bytes).expect("write dense legacy pipe gas");
    }

    #[test]
    fn save_roundtrip_preserves_world_and_gas_state() {
        let root = temp_saves_root("flux_save_roundtrip");
        let registry = test_registry();
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(10, 10, CellMaterial::Brick));
        assert!(world.set_solid_with_material(11, 10, CellMaterial::Metal));

        let mut gas = GasField::from_registry(&registry);
        let mut structures = PlacedStructureMap::default();
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let _ = gas.apply_species_delta_with_lbm(50, 50, 0, 7_000.0);
        let _ = gas.apply_species_delta_with_lbm(51, 50, 1, 4_000.0);
        let _ = gas.apply_species_delta_with_lbm(52, 50, 2, 2_000.0);
        assert!(structures.place_gas_source(14, 14, 0, 10, &world).is_some());
        assert!(structures.place_gas_sink(15, 14, 5, &world).is_some());
        assert!(structures.place_pipe(20, 20, &world));
        assert!(structures.place_pipe(21, 20, &world));
        assert!(structures.place_vent(20, 20, &world));
        assert!(structures.place_vent(21, 20, &world));
        pipe_gas.sync_to_structures(&structures);
        pipe_gas.add_species_counts(0, &[70_000, 30_000, 0]);

        let descriptor = create_save(
            &root,
            "Test Save",
            &world,
            &gas,
            &structures,
            &pipe_gas,
            &registry,
            123,
        )
        .expect("create save");
        let loaded = load_save(&root, &descriptor.id, &registry).expect("load save");
        assert_eq!(loaded.state.simulation_step, 123);
        assert_eq!(
            loaded.state.world_cell_codes.len(),
            (WORLD_WIDTH * WORLD_HEIGHT) as usize
        );

        let mut restored_world = WorldGrid::default();
        restored_world
            .restore_from_cell_codes(&loaded.state.world_cell_codes)
            .expect("restore world");
        assert_eq!(restored_world.cell(10, 10), world.cell(10, 10));
        assert_eq!(restored_world.cell(11, 10), world.cell(11, 10));

        let mut restored_gas = GasField::from_registry(&registry);
        restored_gas
            .restore_state(&loaded.state.gas_snapshot)
            .expect("restore gas");
        let mut restored_structures = PlacedStructureMap::default();
        restored_structures
            .restore_state(&loaded.state.placed_structures_snapshot, &restored_world)
            .expect("restore structures");
        let mut restored_pipe_gas = PipeGasField::from_registry(&registry);
        restored_pipe_gas
            .restore_state(&loaded.state.pipe_gas_snapshot, &restored_structures)
            .expect("restore pipe gas");
        let original = gas.snapshot_state();
        let restored = restored_gas.snapshot_state();
        assert_eq!(original.gas_count, restored.gas_count);
        assert_eq!(original.species.len(), restored.species.len());
        for i in 0..original.species.len() {
            assert_eq!(original.species[i], restored.species[i]);
        }
        assert!(restored_structures.editable_structure_at(14, 14).is_some());
        assert!(restored_structures.editable_structure_at(15, 14).is_some());
        assert!(restored_structures.has_pipe_at(20, 20));
        assert!(restored_structures.has_vent_at(20, 20));
        assert_eq!(restored_pipe_gas.total_amount_particles(0), 100_000);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn list_saves_sorted_from_new_to_old() {
        let root = temp_saves_root("flux_save_sort");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);

        let first = create_save(
            &root, "first", &world, &gas, &structures, &pipe_gas, &registry, 1,
        )
        .expect("first save");
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = create_save(
            &root, "second", &world, &gas, &structures, &pipe_gas, &registry, 2,
        )
            .expect("second save");

        let saves = list_saves(&root).expect("list saves");
        assert_eq!(saves.len(), 2);
        assert_eq!(saves[0].id, second.id);
        assert_eq!(saves[1].id, first.id);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preview_patch_adds_schema_v5_chunk_and_list_preview_path() {
        let root = temp_saves_root("flux_save_preview_patch");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);

        let descriptor = create_save(
            &root,
            "preview-test",
            &world,
            &gas,
            &structures,
            &pipe_gas,
            &registry,
            10,
        )
        .expect("save");
        let preview_path = patch_save_preview_meta(&root, &descriptor.id).expect("patch meta");
        write_preview_png(&preview_path);

        let meta = read_meta(&root.join(&descriptor.id).join(META_FILE)).expect("read meta");
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert!(meta.chunks.iter().any(|chunk| chunk.id == CHUNK_PREVIEW_PNG_ID));

        let listed = list_saves(&root).expect("list saves");
        let saved = listed
            .into_iter()
            .find(|item| item.id == descriptor.id)
            .expect("saved descriptor");
        assert_eq!(saved.preview_path.as_deref(), Some(preview_path.as_path()));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn overwrite_save_keeps_preview_chunk_metadata() {
        let root = temp_saves_root("flux_save_overwrite_preview_chunk");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);

        let descriptor = create_save(
            &root,
            "overwrite-preview",
            &world,
            &gas,
            &structures,
            &pipe_gas,
            &registry,
            10,
        )
        .expect("save");
        overwrite_save(
            &root,
            &descriptor.id,
            &world,
            &gas,
            &structures,
            &pipe_gas,
            &registry,
            11,
        )
        .expect("overwrite");

        let meta = read_meta(&root.join(&descriptor.id).join(META_FILE)).expect("read meta");
        let chunk_ids = meta.chunks.iter().map(|chunk| chunk.id.as_str()).collect::<Vec<_>>();
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert!(chunk_ids.contains(&CHUNK_WORLD_CELLS_ID));
        assert!(chunk_ids.contains(&CHUNK_GAS_STATE_ID));
        assert!(chunk_ids.contains(&CHUNK_PLACED_STRUCTURES_ID));
        assert!(chunk_ids.contains(&CHUNK_PIPE_GAS_ID));
        assert!(chunk_ids.contains(&CHUNK_PREVIEW_PNG_ID));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preview_meta_patch_upgrades_schema_2_3_4_slots_without_breaking_load() {
        let root = temp_saves_root("flux_save_legacy_preview_patch");
        let registry = test_registry();

        for schema_version in [2_u32, 3, 4] {
            let slot_id = write_legacy_schema_slot(&root, schema_version, &registry);
            let preview_path =
                patch_save_preview_meta(&root, &slot_id).expect("patch legacy preview meta");
            write_preview_png(&preview_path);

            let meta = read_meta(&root.join(&slot_id).join(META_FILE)).expect("read legacy meta");
            assert_eq!(meta.schema_version, SCHEMA_VERSION);
            assert!(meta.chunks.iter().any(|chunk| chunk.id == CHUNK_PREVIEW_PNG_ID));

            let loaded = load_save(&root, &slot_id, &registry).expect("load patched legacy slot");
            assert_eq!(loaded.descriptor.id, slot_id);
            assert_eq!(loaded.descriptor.preview_path.as_deref(), Some(preview_path.as_path()));
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_supports_legacy_dense_pipe_gas_chunk_format() {
        let root = temp_saves_root("flux_save_legacy_dense_pipe_gas");
        let registry = test_registry();
        let slot_id = write_legacy_schema_slot(&root, 3, &registry);
        let slot_dir = root.join(&slot_id);
        let pipe_layout = read_pipe_layout_chunk(&slot_dir.join("pipe_layout.bin"))
            .expect("read legacy pipe layout");
        let gas_ids = registry
            .all()
            .iter()
            .map(|definition| definition.id.clone())
            .collect::<Vec<_>>();
        let meta_path = slot_dir.join(META_FILE);
        let mut meta = read_meta(&meta_path).expect("read legacy meta");
        meta.chunks.push(SaveChunkMetaToml {
            id: CHUNK_PIPE_GAS_ID.to_string(),
            file: PIPE_GAS_FILE.to_string(),
            format: "binary_v1".to_string(),
        });
        write_meta(&meta_path, &meta).expect("rewrite legacy meta");
        write_legacy_dense_pipe_gas_chunk(&slot_dir.join(PIPE_GAS_FILE), &gas_ids, &pipe_layout);

        let loaded = load_save(&root, &slot_id, &registry).expect("load dense legacy pipe gas");
        assert!(loaded.state.pipe_gas_snapshot.nodes.iter().any(|node| {
            matches!(node.key.kind, PipeContainerKind::Pipe)
                && node.species.iter().copied().sum::<u32>() > 0
        }));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_rejects_unknown_saved_gas_id() {
        let root = temp_saves_root("flux_save_unknown_gas");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
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
        let mut bytes = fs::read(&gas_path).expect("read gas chunk");
        // Header: magic(4)+version(2)+width(4)+height(4)+step(8)+gas_count(4)+cells(4) = 30
        // First gas id len u16 starts at 30.
        let id_len = u16::from_le_bytes([bytes[30], bytes[31]]) as usize;
        assert!(id_len >= 2);
        bytes[32] = b'z';
        bytes[33] = b'z';
        fs::write(&gas_path, bytes).expect("write patched gas chunk");

        let err =
            load_save(&root, &descriptor.id, &registry).expect_err("must fail on unknown gas");
        let message = err.to_string();
        assert!(message.contains("unknown gas id") || message.contains("Gas chunk references"));

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

    #[test]
    fn save_session_state_uses_step_only_for_dirty_check() {
        let mut session = SaveSessionState::default();
        session.mark_persisted(100, Some("slot".to_string()));
        assert!(!session.has_unsaved_changes(100));
        assert!(session.has_unsaved_changes(101));
    }

    #[test]
    fn delete_save_removes_slot_and_fails_for_missing_slot() {
        let root = temp_saves_root("flux_save_delete_slot");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);

        let descriptor = create_save(
            &root, "to-delete", &world, &gas, &structures, &pipe_gas, &registry, 9,
        )
        .expect("save should be created");
        assert!(root.join(&descriptor.id).exists());

        delete_save(&root, &descriptor.id).expect("existing save should be deleted");
        assert!(!root.join(&descriptor.id).exists());

        let err =
            delete_save(&root, &descriptor.id).expect_err("missing save should report error");
        assert!(err.to_string().contains("does not exist"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn world_load_state_starts_unloaded() {
        assert!(!WorldLoadState::default().has_world);
    }
}
