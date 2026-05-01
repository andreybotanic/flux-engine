use std::path::Path;

use bevy::{
    asset::AssetPlugin,
    prelude::*,
    window::{MonitorSelection, PresentMode, WindowMode},
};

use crate::{
    debug::DebugPlugin, editor::EditorPlugin, input::InputPlugin, render::RenderPlugin,
    simulation::GasSimulationPlugin, ui::UiPlugin, world::WorldPlugin,
};

pub fn run() {
    let asset_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .to_string_lossy()
        .to_string();

    App::new()
        .insert_resource(ClearColor(Color::srgb(0.90, 0.91, 0.92)))
        .insert_resource(Time::<Fixed>::from_hz(30.0))
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
        ))
        .run();
}
