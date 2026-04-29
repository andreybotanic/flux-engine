use bevy::{prelude::*, window::PresentMode};

use crate::{debug::DebugPlugin, input::InputPlugin, render::RenderPlugin, simulation::GasSimulationPlugin, ui::UiPlugin, world::WorldPlugin};

pub fn run() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "FluxEngine".into(),
                        present_mode: PresentMode::AutoVsync,
                        resolution: (1600.0, 900.0).into(),
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
            DebugPlugin,
        ))
        .run();
}
