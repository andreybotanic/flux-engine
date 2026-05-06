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
"#,
        )
        .expect("write cell types");

        for (file_name, content) in [
            ("pipe.toml", "draw_priority = 100\nsize_in_cells = [1, 1]\n"),
            ("vent.toml", "draw_priority = 120\nsize_in_cells = [1, 1]\n"),
            (
                "gas_source.toml",
                "draw_priority = 130\nsize_in_cells = [1, 1]\n",
            ),
            (
                "gas_sink.toml",
                "draw_priority = 130\nsize_in_cells = [1, 1]\n",
            ),
            (
                "gas_pipe_bridge.toml",
                "draw_priority = 110\nsize_in_cells = [3, 1]\n",
            ),
            (
                "boundary.toml",
                "draw_priority = 1000\nsize_in_cells = [1, 1]\n",
            ),
            ("brick.toml", "draw_priority = 1000\nsize_in_cells = [1, 1]\n"),
            ("metal.toml", "draw_priority = 1000\nsize_in_cells = [1, 1]\n"),
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
    fn config_loader_rejects_empty_gas_list() {
        let root = make_temp_root("flux_cfg_empty");
        write_minimal_configs(&root);
        let err = match GameConfig::load_from_root(&root) {
            Ok(_) => panic!("must fail on empty gases"),
            Err(err) => err,
        };
        assert!(err.contains("contains no .toml files"));
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
            "draw_priority = 110\nsize_in_cells = [2, 1]\n",
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
}
