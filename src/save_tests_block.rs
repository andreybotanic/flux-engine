#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::GasDefinition,
        simulation::pipes::PipeGasField,
        world::{
            grid::{CellMaterial, WorldGrid},
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
        let accepted = pipe_gas.add_species_counts_limited(0, &[70, 30, 0]);
        assert_eq!(accepted.iter().copied().sum::<u32>(), 100);

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
        assert_eq!(restored_pipe_gas.total_amount_particles(0), 100);

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
        assert!(err.to_string().contains("unknown gas id"));

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
