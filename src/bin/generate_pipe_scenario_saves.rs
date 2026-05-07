use flux_engine::{
    config::GameConfig,
    save::{create_save, delete_save, list_saves, saves_root_default},
    simulation::{gas::GasField, pipes::PipeGasField},
    world::{
        grid::{CellMaterial, WorldGrid},
        structures::PlacedStructureMap,
    },
};

const SCENARIO_PREFIX: &str = "Pipe Scenario ";

struct ScenarioSlot {
    display_name: &'static str,
    world: WorldGrid,
    gas: GasField,
    structures: PlacedStructureMap,
    pipe_gas: PipeGasField,
}

fn main() -> Result<(), String> {
    let config = GameConfig::load_from_default_location()?;
    let registry = &config.gas_registry;
    let pipe_config = &config.gas_simulation.pipe;
    let hydrogen = registry
        .index_of("h2")
        .ok_or_else(|| "Gas registry does not contain 'h2'".to_string())?;
    let saves_root = saves_root_default();

    if saves_root.exists() {
        for save in list_saves(&saves_root).map_err(|err| err.to_string())? {
            if save.display_name.starts_with(SCENARIO_PREFIX) {
                delete_save(&saves_root, &save.id).map_err(|err| err.to_string())?;
            }
        }
    }

    let scenarios = vec![
        scenario_one_long_pipe_to_vacuum(
            registry,
            pipe_config.segment_capacity_particles,
            hydrogen,
        ),
        scenario_two_two_rooms_single_path(
            registry,
            pipe_config.segment_capacity_particles,
            hydrogen,
        ),
        scenario_three_two_rooms_with_p_branch(
            registry,
            pipe_config.segment_capacity_particles,
            hydrogen,
        ),
        scenario_four_dead_end_fill(registry, pipe_config.segment_capacity_particles, hydrogen),
        scenario_five_star_three_rooms(registry, pipe_config.segment_capacity_particles, hydrogen),
    ];

    for scenario in scenarios {
        let descriptor = create_save(
            &saves_root,
            scenario.display_name,
            &scenario.world,
            &scenario.gas,
            &scenario.structures,
            &scenario.pipe_gas,
            registry,
            0,
        )
        .map_err(|err| err.to_string())?;
        println!("Created '{}' as {}", scenario.display_name, descriptor.id);
    }

    Ok(())
}

fn blank_slot(
    registry: &flux_engine::config::GasRegistry,
    pipe_capacity: u32,
    display_name: &'static str,
) -> ScenarioSlot {
    ScenarioSlot {
        display_name,
        world: WorldGrid::default(),
        gas: GasField::from_registry(registry),
        structures: PlacedStructureMap::default(),
        pipe_gas: PipeGasField::from_registry_with_capacity(registry, pipe_capacity),
    }
}

fn scenario_one_long_pipe_to_vacuum(
    registry: &flux_engine::config::GasRegistry,
    pipe_capacity: u32,
    gas_index: usize,
) -> ScenarioSlot {
    let mut slot = blank_slot(
        registry,
        pipe_capacity,
        "Pipe Scenario 1 - 100k Source to Vacuum",
    );
    build_room(&mut slot.world, 6, 43, 24, 59);
    build_room(&mut slot.world, 78, 43, 96, 59);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 23, 79, 51);
    ensure_vent(&mut slot.structures, &slot.world, 23, 51);
    ensure_vent(&mut slot.structures, &slot.world, 79, 51);
    slot.structures
        .place_gas_source(10, 51, gas_index, 20_000, &slot.world)
        .expect("left source");
    slot.structures
        .place_gas_sink(92, 51, 20_000, &slot.world)
        .expect("right sink");
    fill_room_gas(&mut slot.gas, gas_index, 7, 44, 23, 58, 100_000.0);
    finalize_slot(&mut slot);
    slot
}

fn scenario_two_two_rooms_single_path(
    registry: &flux_engine::config::GasRegistry,
    pipe_capacity: u32,
    gas_index: usize,
) -> ScenarioSlot {
    let mut slot = blank_slot(
        registry,
        pipe_capacity,
        "Pipe Scenario 2 - Two Rooms Single Pipe",
    );
    build_room(&mut slot.world, 6, 43, 24, 59);
    build_room(&mut slot.world, 78, 43, 96, 59);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 23, 79, 51);
    ensure_vent(&mut slot.structures, &slot.world, 23, 51);
    ensure_vent(&mut slot.structures, &slot.world, 79, 51);
    fill_room_gas(&mut slot.gas, gas_index, 7, 44, 23, 58, 50_000.0);
    fill_room_gas(&mut slot.gas, gas_index, 79, 44, 95, 58, 10_000.0);
    finalize_slot(&mut slot);
    slot
}

fn scenario_three_two_rooms_with_p_branch(
    registry: &flux_engine::config::GasRegistry,
    pipe_capacity: u32,
    gas_index: usize,
) -> ScenarioSlot {
    let mut slot = blank_slot(
        registry,
        pipe_capacity,
        "Pipe Scenario 3 - Two Rooms With P-Branch",
    );
    build_room(&mut slot.world, 6, 43, 24, 63);
    build_room(&mut slot.world, 78, 43, 96, 63);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 23, 79, 53);
    place_vertical_pipe(&mut slot.structures, &slot.world, 44, 49, 52);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 44, 58, 49);
    place_vertical_pipe(&mut slot.structures, &slot.world, 58, 49, 52);
    ensure_vent(&mut slot.structures, &slot.world, 23, 53);
    ensure_vent(&mut slot.structures, &slot.world, 79, 53);
    fill_room_gas(&mut slot.gas, gas_index, 7, 44, 23, 62, 20_000.0);
    fill_room_gas(&mut slot.gas, gas_index, 79, 44, 95, 62, 5_000.0);
    finalize_slot(&mut slot);
    slot
}

fn scenario_four_dead_end_fill(
    registry: &flux_engine::config::GasRegistry,
    pipe_capacity: u32,
    gas_index: usize,
) -> ScenarioSlot {
    let mut slot = blank_slot(
        registry,
        pipe_capacity,
        "Pipe Scenario 4 - Dead-End High Pressure",
    );
    build_room(&mut slot.world, 10, 43, 28, 59);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 27, 85, 51);
    ensure_vent(&mut slot.structures, &slot.world, 27, 51);
    slot.structures
        .place_gas_source(14, 51, gas_index, 20_000, &slot.world)
        .expect("dead-end source");
    fill_room_gas(&mut slot.gas, gas_index, 11, 44, 27, 58, 100_000.0);
    finalize_slot(&mut slot);
    slot
}

fn scenario_five_star_three_rooms(
    registry: &flux_engine::config::GasRegistry,
    pipe_capacity: u32,
    gas_index: usize,
) -> ScenarioSlot {
    let mut slot = blank_slot(registry, pipe_capacity, "Pipe Scenario 5 - Three-Room Star");
    build_room(&mut slot.world, 43, 6, 59, 24);
    build_room(&mut slot.world, 6, 43, 24, 59);
    build_room(&mut slot.world, 78, 43, 96, 59);
    place_vertical_pipe(&mut slot.structures, &slot.world, 51, 23, 51);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 23, 51, 51);
    place_horizontal_pipe(&mut slot.structures, &slot.world, 51, 79, 51);
    ensure_vent(&mut slot.structures, &slot.world, 51, 23);
    ensure_vent(&mut slot.structures, &slot.world, 23, 51);
    ensure_vent(&mut slot.structures, &slot.world, 79, 51);
    fill_room_gas(&mut slot.gas, gas_index, 7, 44, 23, 58, 10_000.0);
    fill_room_gas(&mut slot.gas, gas_index, 79, 44, 95, 58, 50_000.0);
    finalize_slot(&mut slot);
    slot
}

fn build_room(world: &mut WorldGrid, left: u32, top: u32, right: u32, bottom: u32) {
    for x in left..=right {
        let _ = world.set_solid_with_material(x, top, CellMaterial::Brick);
        let _ = world.set_solid_with_material(x, bottom, CellMaterial::Brick);
    }
    for y in top..=bottom {
        let _ = world.set_solid_with_material(left, y, CellMaterial::Brick);
        let _ = world.set_solid_with_material(right, y, CellMaterial::Brick);
    }
}

fn fill_room_gas(
    gas: &mut GasField,
    gas_index: usize,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    amount_per_cell: f32,
) {
    for y in top..=bottom {
        for x in left..=right {
            gas.set_amount(x, y, gas_index, amount_per_cell);
        }
    }
}

fn place_horizontal_pipe(
    structures: &mut PlacedStructureMap,
    world: &WorldGrid,
    start_x: u32,
    end_x: u32,
    y: u32,
) {
    for x in start_x.min(end_x)..=start_x.max(end_x) {
        ensure_pipe(structures, world, x, y);
    }
}

fn place_vertical_pipe(
    structures: &mut PlacedStructureMap,
    world: &WorldGrid,
    x: u32,
    start_y: u32,
    end_y: u32,
) {
    for y in start_y.min(end_y)..=start_y.max(end_y) {
        ensure_pipe(structures, world, x, y);
    }
}

fn ensure_pipe(structures: &mut PlacedStructureMap, world: &WorldGrid, x: u32, y: u32) {
    if !structures.has_pipe_at(x, y) {
        assert!(structures.place_pipe(x, y, world), "pipe at ({x}, {y})");
    }
}

fn ensure_vent(structures: &mut PlacedStructureMap, world: &WorldGrid, x: u32, y: u32) {
    if !structures.has_pipe_at(x, y) {
        ensure_pipe(structures, world, x, y);
    }
    if !structures.has_vent_at(x, y) {
        assert!(structures.place_vent(x, y, world), "vent at ({x}, {y})");
    }
}

fn finalize_slot(slot: &mut ScenarioSlot) {
    slot.pipe_gas.sync_to_structures(&slot.structures);
    slot.gas.recompute_total_density_buffer(&slot.world);
}
