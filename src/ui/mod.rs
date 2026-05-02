pub mod cell_inspector;
pub mod input_field;
pub mod sim_controls;

use bevy::prelude::*;

use self::cell_inspector::{setup_cell_inspector, update_cell_inspector};
use self::input_field::TextInputPlugin;
use self::sim_controls::{
    handle_sim_control_buttons, handle_sim_control_keyboard, refresh_sim_control_ui,
    refresh_sim_control_visibility, setup_sim_control_ui,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin)
            .add_systems(Startup, (setup_cell_inspector, setup_sim_control_ui))
            .add_systems(
                Update,
                (
                    handle_sim_control_keyboard,
                    handle_sim_control_buttons,
                    refresh_sim_control_visibility,
                    refresh_sim_control_ui,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                update_cell_inspector.after(TransformSystem::TransformPropagate),
            );
    }
}
