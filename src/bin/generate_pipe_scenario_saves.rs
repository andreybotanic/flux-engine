use flux_engine::{
    config::GameConfig,
    plugins::{
        default_plugin::{
            default_content_registry, pipe_runtime::scenarios::build_reference_pipe_scenarios,
        },
        EnabledPluginSet,
    },
    save::{create_save, delete_save, list_saves, saves_root_default},
};

const SCENARIO_PREFIX: &str = "Pipe Scenario ";

fn main() -> Result<(), String> {
    let config = GameConfig::load_from_default_location()?;
    let content_registry = default_content_registry();
    let mut enabled_plugins = EnabledPluginSet::default();
    enabled_plugins.enforce_default_plugin();
    let saves_root = saves_root_default();

    if saves_root.exists() {
        for save in list_saves(&saves_root).map_err(|err| err.to_string())? {
            if save.display_name.starts_with(SCENARIO_PREFIX) {
                delete_save(&saves_root, &save.id).map_err(|err| err.to_string())?;
            }
        }
    }

    let scenarios = build_reference_pipe_scenarios(&config.gas_registry, &config.pipe_simulation)?;
    for scenario in scenarios {
        let descriptor = create_save(
            &saves_root,
            scenario.display_name,
            &scenario.world,
            &scenario.gas,
            &scenario.structures,
            &scenario.pipe_gas,
            &config.gas_registry,
            &content_registry,
            &enabled_plugins,
            0,
        )
        .map_err(|err| err.to_string())?;
        println!("Created '{}' as {}", scenario.display_name, descriptor.id);
    }

    Ok(())
}
