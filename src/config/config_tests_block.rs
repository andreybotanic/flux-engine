#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_root(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        fs::create_dir_all(root.join("gases")).expect("create temp dirs");
        root
    }

    fn write_minimal_configs(root: &Path) {
        fs::create_dir_all(root.join("structures")).expect("create structures dir");
        fs::write(
            root.join("simulation.toml"),
            r#"
[rate]
target_hz = 30

[simulation]
thermal_motion_scale = 0.08

[solver_tuning]
enable_buoyancy = true
buoyancy_strength = 0.22
buoyancy_window_radius = 2
buoyancy_window_sigma = 1.2
buoyancy_gain = 3.2
buoyancy_alpha = 1.0
buoyancy_force_cap = 0.30

[visual]
gamma = 1.0
max_particles_for_max_color = 1000
f1_min_particles = 1.0
f1_max_particles_for_max_intensity = 1000.0
f1_min_intensity = 0.05
f1_alpha = 0.88
"#,
        )
        .expect("write simulation");

        fs::write(
            root.join("pipe_runtime.toml"),
            r#"
cell_volume_ratio = 25.0
cell_particle_pressure_pa = 1.0
pipe_flux_gain = 180.0
pipe_flux_damping = 0.82
max_pipe_flux_particles_per_tick = 2000.0
vent_discharge_coefficient = 110.0
max_vent_flux_particles_per_tick = 1500.0
vent_choked_pressure_ratio = 0.53
pressure_epsilon_pa = 0.01
"#,
        )
        .expect("write pipe runtime");

        fs::write(
            root.join("cell_types.toml"),
            r#"
[boundary]
main_tint = [0.90, 0.90, 0.91]
gas_tint = [0.66, 0.66, 0.67]

[brick]
main_tint = [0.99, 0.99, 0.99]
gas_tint = [0.72, 0.72, 0.74]

[metal]
main_tint = [0.99, 0.99, 0.99]
gas_tint = [0.71, 0.71, 0.73]

[world_cell_hud]
label = "Cell"
sort_order = 0

[[world_cell_hud.substance_containers]]
substance = "gas"
backing = "world_cell"
visible_on_hover = "same_cell"
"#,
        )
        .expect("write cell types");

        for (file_name, content) in [
            (
                "pipe.toml",
                "label = \"Pipe\"\ndraw_priority = 100\nsize_in_cells = [1, 1]\n\n[hud]\nsort_order = 10\n\n[[hud.substance_containers]]\nsubstance = \"gas\"\nbacking = \"pipe_node\"\nkind = \"pipe\"\nvisible_on_hover = \"same_cell\"\n",
            ),
            (
                "vent.toml",
                "label = \"Vent\"\ndraw_priority = 120\nsize_in_cells = [1, 1]\n\n[hud]\nsort_order = 30\n",
            ),
            (
                "gas_source.toml",
                "label = \"Gas Source\"\ndraw_priority = 130\nsize_in_cells = [1, 1]\n\n[hud]\nsort_order = 40\n",
            ),
            (
                "gas_sink.toml",
                "label = \"Gas Sink\"\ndraw_priority = 130\nsize_in_cells = [1, 1]\n\n[hud]\nsort_order = 50\n",
            ),
            (
                "gas_pipe_bridge.toml",
                "label = \"Bridge\"\ndraw_priority = 110\nsize_in_cells = [3, 1]\n\n[hud]\nsort_order = 20\n\n[[hud.substance_containers]]\nsubstance = \"gas\"\nbacking = \"pipe_node\"\nkind = \"bridge_pipe\"\nvisible_on_hover = \"container_cell\"\n",
            ),
            (
                "boundary.toml",
                "label = \"Boundary\"\ndraw_priority = 1000\nsize_in_cells = [1, 1]\n",
            ),
            (
                "brick.toml",
                "label = \"Brick\"\ndraw_priority = 1000\nsize_in_cells = [1, 1]\n",
            ),
            (
                "metal.toml",
                "label = \"Metal\"\ndraw_priority = 1000\nsize_in_cells = [1, 1]\n",
            ),
        ] {
            fs::write(root.join("structures").join(file_name), content)
                .expect("write structure visual config");
        }
    }

    #[test]
    fn config_loader_rejects_duplicate_gas_ids() {
        let root = make_temp_root("flux_cfg_dup");
        write_minimal_configs(&root);

        fs::write(
            root.join("gases").join("a.toml"),
            r#"id = "same"
label = "A"
molecular_mass = 1.0
color = [0.2, 0.2, 0.2]
"#,
        )
        .expect("write gas a");
        fs::write(
            root.join("gases").join("b.toml"),
            r#"id = "same"
label = "B"
molecular_mass = 2.0
color = [0.3, 0.3, 0.3]
"#,
        )
        .expect("write gas b");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("must fail on duplicate ids"),
            Err(err) => err,
        };
        assert!(err.contains("Duplicate gas id"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_uses_default_plugin_substances_when_gas_dir_is_empty() {
        let root = make_temp_root("flux_cfg_empty");
        write_minimal_configs(&root);
        let config = GameConfig::load_from_root(&root).expect("default substances should load");
        assert_eq!(config.gas_registry.count(), 3);
        assert_eq!(
            config.gas_registry.stable_ids(),
            vec![
                "flux.default.substance.h2",
                "flux.default.substance.o2",
                "flux.default.substance.co2"
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_includes_external_plugin_substances() {
        let root = make_temp_root("flux_cfg_external_gas");
        write_minimal_configs(&root);
        let plugin_id = crate::plugins::PluginId::parse("flux.sample_content").expect("plugin id");
        let substance = crate::plugins::SubstanceDefinition::gas(
            crate::plugins::SubstanceId::parse("flux.sample_content.substance.neon")
                .expect("substance id"),
            plugin_id.clone(),
            "Neon",
            20.180,
            [1.0, 0.32, 0.78],
            vec!["neon".to_string()],
        )
        .expect("substance");
        let mut content_registry = crate::plugins::default_plugin::default_content_registry();
        content_registry.register_provider_plugin(plugin_id);
        content_registry.register_substance(substance);

        let config = GameConfig::load_from_root_with_content(&root, &content_registry)
            .expect("config with external substance");
        assert_eq!(config.gas_registry.count(), 4);
        assert_eq!(config.gas_registry.index_of("neon"), Some(1));
        assert_eq!(
            config.gas_registry.stable_id_by_index(1).map(|id| id.as_str()),
            Some("flux.sample_content.substance.neon")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_rejects_invalid_gas_values() {
        let root = make_temp_root("flux_cfg_bad");
        write_minimal_configs(&root);
        fs::write(
            root.join("gases").join("bad.toml"),
            r#"id = "bad"
label = "Bad"
molecular_mass = -5.0
color = [1.2, 0.0, 0.0]
"#,
        )
        .expect("write bad gas");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("must fail on invalid values"),
            Err(err) => err,
        };
        assert!(err.contains("invalid molecular_mass") || err.contains("invalid color"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gas_registry_orders_by_molecular_mass() {
        let registry = GasRegistry::new(vec![
            GasDefinition {
                id: "co2".to_string(),
                label: "CO2".to_string(),
                molecular_mass: 44.009,
                color: [0.5, 0.5, 0.5],
            },
            GasDefinition {
                id: "h2".to_string(),
                label: "H2".to_string(),
                molecular_mass: 2.016,
                color: [0.8, 0.2, 0.9],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "O2".to_string(),
                molecular_mass: 31.998,
                color: [0.0, 0.85, 0.85],
            },
        ])
        .expect("valid registry");

        let ids = registry
            .all()
            .iter()
            .map(|g| g.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["h2", "o2", "co2"]);
        assert_eq!(
            registry.stable_ids(),
            vec![
                "flux.default.substance.h2",
                "flux.default.substance.o2",
                "flux.default.substance.co2"
            ]
        );
        assert_eq!(registry.index_of("flux.default.substance.h2"), Some(0));
    }

    #[test]
    fn config_loader_rejects_missing_structure_visual_file() {
        let root = make_temp_root("flux_cfg_missing_structure");
        write_minimal_configs(&root);
        fs::remove_file(root.join("structures").join("pipe.toml")).expect("remove pipe config");
        fs::write(
            root.join("gases").join("h2.toml"),
            r#"id = "h2"
label = "H2"
molecular_mass = 2.016
color = [0.8, 0.8, 1.0]
"#,
        )
        .expect("write gas");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("missing structure config must fail"),
            Err(err) => err,
        };
        assert!(err.contains("pipe.toml"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_rejects_invalid_bridge_size() {
        let root = make_temp_root("flux_cfg_bad_bridge_size");
        write_minimal_configs(&root);
        fs::write(
            root.join("structures").join("gas_pipe_bridge.toml"),
            "label = \"Bridge\"\ndraw_priority = 110\nsize_in_cells = [2, 1]\n",
        )
        .expect("overwrite bridge config");
        fs::write(
            root.join("gases").join("h2.toml"),
            r#"id = "h2"
label = "H2"
molecular_mass = 2.016
color = [0.8, 0.8, 1.0]
"#,
        )
        .expect("write gas");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("invalid bridge size must fail"),
            Err(err) => err,
        };
        assert!(err.contains("GasPipeBridge") || err.contains("gas_pipe_bridge"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_reads_hud_metadata_for_world_and_structures() {
        let root = make_temp_root("flux_cfg_hud_ok");
        write_minimal_configs(&root);
        fs::write(
            root.join("gases").join("h2.toml"),
            r#"id = "h2"
label = "Hydrogen"
molecular_mass = 2.016
color = [0.8, 0.8, 1.0]
"#,
        )
        .expect("write gas");

        let config = GameConfig::load_from_root(&root).expect("config should load");
        assert_eq!(config.world_cell_hud.label, "Cell");
        assert_eq!(
            config.gas_registry.stable_id_by_index(0).map(|id| id.as_str()),
            Some("flux.default.substance.h2")
        );
        assert_eq!(config.structure_visuals.get(crate::plugins::default_plugin::pipe_structure_kind()).label, "Pipe");
        assert_eq!(config.cell_visual_layouts.get(crate::plugins::default_plugin::brick_cell_material()).label, "Brick");
        assert_eq!(
            config.structure_hud.get(crate::plugins::default_plugin::gas_pipe_bridge_structure_kind()).sort_order,
            20
        );
        assert_eq!(
            config.structure_hud.get(crate::plugins::default_plugin::vent_structure_kind()).substance_containers,
            Vec::<SubstanceContainerConfig>::new()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_rejects_world_cell_backing_inside_structure_hud() {
        let root = make_temp_root("flux_cfg_bad_world_backing");
        write_minimal_configs(&root);
        fs::write(
            root.join("structures").join("vent.toml"),
            r#"label = "Vent"
draw_priority = 120
size_in_cells = [1, 1]

[hud]
sort_order = 30

[[hud.substance_containers]]
substance = "gas"
backing = "world_cell"
visible_on_hover = "same_cell"
"#,
        )
        .expect("overwrite vent config");
        fs::write(
            root.join("gases").join("h2.toml"),
            r#"id = "h2"
label = "Hydrogen"
molecular_mass = 2.016
color = [0.8, 0.8, 1.0]
"#,
        )
        .expect("write gas");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("invalid structure HUD must fail"),
            Err(err) => err,
        };
        assert!(err.contains("world_cell backing"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn config_loader_rejects_bridge_pipe_visibility_outside_center_only_mode() {
        let root = make_temp_root("flux_cfg_bad_bridge_visibility");
        write_minimal_configs(&root);
        fs::write(
            root.join("structures").join("gas_pipe_bridge.toml"),
            r#"label = "Bridge"
draw_priority = 110
size_in_cells = [3, 1]

[hud]
sort_order = 20

[[hud.substance_containers]]
substance = "gas"
backing = "pipe_node"
kind = "bridge_pipe"
visible_on_hover = "same_cell"
"#,
        )
        .expect("overwrite bridge config");
        fs::write(
            root.join("gases").join("h2.toml"),
            r#"id = "h2"
label = "Hydrogen"
molecular_mass = 2.016
color = [0.8, 0.8, 1.0]
"#,
        )
        .expect("write gas");

        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("bridge visibility mismatch must fail"),
            Err(err) => err,
        };
        assert!(err.contains("container_cell visibility"));
        let _ = fs::remove_dir_all(root);
    }
}
