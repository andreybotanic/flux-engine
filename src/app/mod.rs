use std::path::Path;

use bevy::{
    asset::AssetPlugin,
    prelude::*,
    window::{MonitorSelection, PresentMode, WindowMode},
};

use crate::{
    config::GameConfig,
    debug::DebugPlugin,
    editor::EditorPlugin,
    input::InputPlugin,
    render::RenderPlugin,
    simulation::{
        backend::{SimulationBackend, SimulationBackendConfig, WorldSizeConfig},
        GasSimulationPlugin,
    },
    ui::UiPlugin,
    world::WorldPlugin,
};

pub fn run() {
    let game_config = GameConfig::load_from_default_location().unwrap_or_else(|err| {
        panic!("Failed to load game config files from ./config: {err}");
    });

    let asset_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .to_string_lossy()
        .to_string();

    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.90, 0.91, 0.92)))
        .insert_resource(Time::<Fixed>::from_hz(30.0));

    let args: Vec<String> = std::env::args().collect();
    let mut backend = std::env::var("FLUX_SIM_BACKEND")
        .ok()
        .map(|value| value.to_lowercase())
        .and_then(|value| match value.as_str() {
            "cpu" => Some(SimulationBackend::Cpu),
            "gpu" => Some(SimulationBackend::Gpu),
            _ => None,
        })
        .unwrap_or(SimulationBackend::Gpu);
    let mut world_size = WorldSizeConfig::default();

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--sim-backend" if i + 1 < args.len() => {
                backend = match args[i + 1].to_lowercase().as_str() {
                    "gpu" => SimulationBackend::Gpu,
                    _ => SimulationBackend::Cpu,
                };
                i += 1;
            }
            "--world-size" if i + 1 < args.len() => {
                let raw = &args[i + 1];
                if let Some((w, h)) = raw.split_once('x') {
                    if let (Ok(width), Ok(height)) = (w.parse::<u32>(), h.parse::<u32>()) {
                        if width >= 3 && height >= 3 {
                            world_size.width = width;
                            world_size.height = height;
                        }
                    }
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    app.insert_resource(game_config.gas_registry.clone())
        .insert_resource(game_config.simulation_rate)
        .insert_resource(game_config.gas_simulation)
        .insert_resource(game_config.gas_visual)
        .insert_resource(game_config.gas_main_visual)
        .insert_resource(game_config.cell_visuals)
        .insert_resource(SimulationBackendConfig { backend })
        .insert_resource(world_size)
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_path,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "FluxEngine".into(),
                        present_mode: PresentMode::AutoVsync,
                        resolution: (1600.0, 900.0).into(),
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            WorldPlugin,
            GasSimulationPlugin,
            RenderPlugin,
            InputPlugin,
            UiPlugin,
            EditorPlugin,
            DebugPlugin,
        ));

    app.run();
}
