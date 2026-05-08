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
        gpu_solver::GpuGasSolver,
        GasSimulationPlugin,
    },
    ui::UiPlugin,
    world::WorldPlugin,
};

/// Runs `run` logic.
pub fn run() {
    let game_config = GameConfig::load_from_default_location().unwrap_or_else(|err| {
        panic!("Failed to load game config files from ./config: {err}");
    });

    let asset_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .to_string_lossy()
        .to_string();

    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Time::<Fixed>::from_hz(30.0));

    let args: Vec<String> = std::env::args().collect();
    let env_backend = std::env::var("FLUX_SIM_BACKEND").ok();
    let mut world_size = WorldSizeConfig::default();
    let mut requested_backend = parse_requested_backend(&args, env_backend.as_deref());

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
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
    if matches!(requested_backend, SimulationBackend::Gpu)
        && !gpu_backend_available(world_size.width, world_size.height)
    {
        eprintln!(
            "GPU backend is unavailable during startup for {}x{}; falling back to CPU.",
            world_size.width, world_size.height
        );
        requested_backend = SimulationBackend::Cpu;
    }

    app.insert_resource(game_config.gas_registry.clone())
        .insert_resource(game_config.simulation_rate)
        .insert_resource(game_config.gas_simulation)
        .insert_resource(game_config.gas_visual)
        .insert_resource(game_config.gas_main_visual)
        .insert_resource(game_config.cell_visuals)
        .insert_resource(game_config.world_cell_hud)
        .insert_resource(game_config.structure_hud)
        .insert_resource(game_config.structure_visuals)
        .insert_resource(game_config.cell_visual_layouts)
        .insert_resource(SimulationBackendConfig {
            backend: requested_backend,
        })
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

fn gpu_backend_available(width: u32, height: u32) -> bool {
    GpuGasSolver::new(width, height).is_ok()
}

fn parse_requested_backend(args: &[String], env_backend: Option<&str>) -> SimulationBackend {
    let mut backend = env_backend
        .map(|value| value.to_lowercase())
        .and_then(|value| match value.as_str() {
            "cpu" => Some(SimulationBackend::Cpu),
            "gpu" => Some(SimulationBackend::Gpu),
            _ => None,
        })
        .unwrap_or(SimulationBackend::Gpu);
    let mut i = 1usize;
    while i < args.len() {
        if args[i] == "--sim-backend" && i + 1 < args.len() {
            backend = match args[i + 1].to_lowercase().as_str() {
                "gpu" => SimulationBackend::Gpu,
                _ => SimulationBackend::Cpu,
            };
            i += 1;
        }
        i += 1;
    }
    backend
}

#[cfg(test)]
mod tests {
    use super::parse_requested_backend;
    use crate::simulation::backend::SimulationBackend;

    #[test]
    fn default_backend_is_gpu() {
        let args = vec!["flux_engine.exe".to_string()];
        assert_eq!(parse_requested_backend(&args, None), SimulationBackend::Gpu);
    }

    #[test]
    fn env_can_switch_backend() {
        let args = vec!["flux_engine.exe".to_string()];
        assert_eq!(
            parse_requested_backend(&args, Some("cpu")),
            SimulationBackend::Cpu
        );
        assert_eq!(
            parse_requested_backend(&args, Some("gpu")),
            SimulationBackend::Gpu
        );
    }

    #[test]
    fn cli_overrides_env_backend() {
        let args = vec![
            "flux_engine.exe".to_string(),
            "--sim-backend".to_string(),
            "cpu".to_string(),
        ];
        assert_eq!(
            parse_requested_backend(&args, Some("gpu")),
            SimulationBackend::Cpu
        );
    }
}
