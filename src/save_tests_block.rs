#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::GasDefinition,
        plugins::{
            default_plugin::{
                default_content_registry,
                pipe_runtime::PipeGasField,
                CELL_BRICK_ID, CELL_BOUNDARY_ID, CELL_METAL_ID, ENTITY_GAS_SOURCE_ID,
                ENTITY_GAS_SINK_ID, ENTITY_PIPE_ID, ENTITY_VENT_ID, SUBSTANCE_H2_ID,
                SUBSTANCE_O2_ID,
            },
            ContentId, EnabledPluginSet, PluginId, SaveChunkStore,
        },
        world::{
            grid::WorldGrid,
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

    fn test_content_registry() -> ContentRegistry {
        default_content_registry()
    }

    fn default_enabled_plugins() -> EnabledPluginSet {
        let mut enabled = EnabledPluginSet::default();
        enabled.enforce_default_plugin();
        enabled
    }

    fn create_save(
        root: &Path,
        display_name: &str,
        world: &WorldGrid,
        gas: &GasField,
        structures: &PlacedStructureMap,
        pipe_gas: &PipeGasField,
        gas_registry: &GasRegistry,
        simulation_step: u64,
    ) -> Result<SaveDescriptor, SaveError> {
        let content_registry = test_content_registry();
        let enabled_plugins = default_enabled_plugins();
        super::create_save(
            root,
            display_name,
            world,
            gas,
            structures,
            pipe_gas,
            gas_registry,
            &content_registry,
            &enabled_plugins,
            simulation_step,
        )
    }

    fn overwrite_save(
        root: &Path,
        save_id: &str,
        world: &WorldGrid,
        gas: &GasField,
        structures: &PlacedStructureMap,
        pipe_gas: &PipeGasField,
        gas_registry: &GasRegistry,
        simulation_step: u64,
    ) -> Result<SaveDescriptor, SaveError> {
        let content_registry = test_content_registry();
        let enabled_plugins = default_enabled_plugins();
        super::overwrite_save(
            root,
            save_id,
            world,
            gas,
            structures,
            pipe_gas,
            gas_registry,
            &content_registry,
            &enabled_plugins,
            simulation_step,
        )
    }

    fn load_save(
        root: &Path,
        save_id: &str,
        gas_registry: &GasRegistry,
    ) -> Result<LoadedSave, SaveError> {
        let content_registry = test_content_registry();
        super::load_save(root, save_id, gas_registry, &content_registry)
    }

    fn required_ids(meta: &SaveMetaToml, kind: &str) -> Vec<String> {
        meta.required_content
            .iter()
            .filter(|item| item.kind == kind)
            .map(|item| item.id.clone())
            .collect()
    }

    fn patch_first_gas_id(path: &Path, replacement_prefix: &[u8]) {
        let mut bytes = fs::read(path).expect("read gas chunk");
        let id_len = u16::from_le_bytes([bytes[30], bytes[31]]) as usize;
        assert!(id_len >= replacement_prefix.len());
        for (offset, byte) in replacement_prefix.iter().copied().enumerate() {
            bytes[32 + offset] = byte;
        }
        fs::write(path, bytes).expect("write patched gas chunk");
    }

    #[test]
    fn save_roundtrip_preserves_world_and_gas_state() {
        let root = temp_saves_root("flux_save_roundtrip");
        let registry = test_registry();
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(10, 10, crate::plugins::default_plugin::brick_cell_material()));
        assert!(world.set_solid_with_material(11, 10, crate::plugins::default_plugin::metal_cell_material()));

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
        let gas_path = root.join(&descriptor.id).join(GAS_STATE_FILE);
        let bytes = fs::read(&gas_path).expect("read gas chunk");
        let first_id_len = u16::from_le_bytes([bytes[30], bytes[31]]) as usize;
        let first_id =
            std::str::from_utf8(&bytes[32..32 + first_id_len]).expect("first gas id utf8");
        assert_eq!(first_id, "flux.default.substance.h2");

        let loaded = load_save(&root, &descriptor.id, &registry).expect("load save");
        assert_eq!(loaded.state.simulation_step, 123);
        assert_eq!(
            loaded.state.world_cells.len(),
            (WORLD_WIDTH * WORLD_HEIGHT) as usize
        );

        let mut restored_world = WorldGrid::default();
        restored_world
            .restore_cells(&loaded.state.world_cells)
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
        let source = restored_structures
            .editable_structure_at(14, 14)
            .expect("restored source");
        assert_eq!(
            source.params,
            StructureParams::GasSource {
                gas_index: 0,
                amount: 10
            }
        );
        assert!(restored_structures.has_pipe_at(20, 20));
        assert!(restored_structures.has_vent_at(20, 20));
        assert_eq!(restored_pipe_gas.total_amount_particles(0), 100_000);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_roundtrip_preserves_plugin_chunks() {
        let root = temp_saves_root("flux_save_plugin_chunks");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let content_registry = test_content_registry();
        let enabled_plugins = default_enabled_plugins();
        let plugin_id = PluginId::parse("flux.api_ui_save_demo").expect("plugin id");
        let chunk_id =
            ContentId::parse("flux.api_ui_save_demo.save.counter").expect("chunk id");
        let mut chunks = SaveChunkStore::default();
        chunks.write_plugin_chunk(plugin_id.clone(), chunk_id.clone(), 1, 42u32.to_le_bytes().to_vec());

        let descriptor = create_save_with_plugin_chunks(
            &root,
            "Plugin Chunk Save",
            &world,
            &gas,
            &structures,
            &pipe_gas,
            &registry,
            &content_registry,
            &enabled_plugins,
            7,
            &chunks,
        )
        .expect("create save");
        let loaded = load_save(&root, &descriptor.id, &registry).expect("load save");
        let restored = loaded
            .state
            .plugin_save_chunks
            .read_plugin_chunk(&plugin_id, &chunk_id)
            .expect("plugin chunk");

        assert_eq!(restored.version, 1);
        assert_eq!(restored.bytes, 42u32.to_le_bytes().to_vec());

        let _ = fs::remove_dir_all(root);
    }

    include!("save_content_gate_tests_block.rs");

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
    fn list_saves_skips_unsupported_schema_versions() {
        let root = temp_saves_root("flux_save_skip_old_schema");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);

        let descriptor = create_save(
            &root,
            "old-schema-hidden",
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
        meta.schema_version = SCHEMA_VERSION - 1;
        write_meta(&meta_path, &meta).expect("write old-schema meta");

        let saves = list_saves(&root).expect("list saves");
        assert!(saves.is_empty(), "unsupported schema should be hidden from the list");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_meta_includes_preview_chunk_and_list_preview_path() {
        let root = temp_saves_root("flux_save_preview_meta");
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
        let preview_path = save_preview_target_path(&root, &descriptor.id);
        write_preview_png(&preview_path);

        let meta = read_meta(&root.join(&descriptor.id).join(META_FILE)).expect("read meta");
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert!(meta.chunks.iter().any(|chunk| chunk.id == CHUNK_PREVIEW_PNG_ID));
        assert_eq!(descriptor.preview_path.as_deref(), Some(preview_path.as_path()));

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
        let chunk_ids = meta
            .chunks
            .iter()
            .map(|chunk| chunk.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert!(chunk_ids.contains(&CHUNK_WORLD_CELLS_ID));
        assert!(chunk_ids.contains(&CHUNK_GAS_STATE_ID));
        assert!(chunk_ids.contains(&CHUNK_PLACED_STRUCTURES_ID));
        assert!(chunk_ids.contains(&CHUNK_PIPE_GAS_ID));
        assert!(chunk_ids.contains(&CHUNK_PREVIEW_PNG_ID));

        let _ = fs::remove_dir_all(root);
    }

    include!("save_format_tests_block.rs");

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
