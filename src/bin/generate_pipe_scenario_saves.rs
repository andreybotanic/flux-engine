use flux_engine::{
    config::GameConfig,
    save::{create_save, delete_save, list_saves, saves_root_default},
    simulation::pipes::scenarios::build_reference_pipe_scenarios,
};

const SCENARIO_PREFIX: &str = "Pipe Scenario ";

fn main() -> Result<(), String> {
    let config = GameConfig::load_from_default_location()?;
    let saves_root = saves_root_default();

    if saves_root.exists() {
        for save in list_saves(&saves_root).map_err(|err| err.to_string())? {
            if save.display_name.starts_with(SCENARIO_PREFIX) {
                delete_save(&saves_root, &save.id).map_err(|err| err.to_string())?;
            }
        }
    }

    let scenarios = build_reference_pipe_scenarios(&config.gas_registry, &config.gas_simulation.pipe)?;
    for scenario in scenarios {
        let descriptor = create_save(
            &saves_root,
            scenario.display_name,
            &scenario.world,
            &scenario.gas,
            &scenario.structures,
            &scenario.pipe_gas,
            &config.gas_registry,
            0,
        )
        .map_err(|err| err.to_string())?;
        println!("Created '{}' as {}", scenario.display_name, descriptor.id);
    }

    Ok(())
}
