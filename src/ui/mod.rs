pub mod cell_inspector;
pub mod input_field;
pub mod palette;
pub mod panels;
pub mod scroll_area;
pub mod select_field;
pub mod sim_controls;

use bevy::prelude::*;

use self::cell_inspector::{setup_cell_inspector, update_cell_inspector};
use self::input_field::TextInputPlugin;
use self::panels::PanelPlugin;
use self::scroll_area::ScrollAreaPlugin;
use self::select_field::SelectFieldPlugin;
use self::sim_controls::{
    handle_sim_control_buttons, handle_sim_control_keyboard, refresh_sim_control_ui,
    refresh_sim_control_visibility, setup_sim_control_ui,
};

#[derive(Resource, Clone)]
/// Stores `UiFont` state.
pub struct UiFont {
    pub handle: Handle<Font>,
}

fn setup_ui_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(UiFont {
        handle: asset_server.load("fonts/ui_main.ttf"),
    });
}

fn apply_ui_font_to_added_text(ui_font: Res<UiFont>, mut query: Query<&mut TextFont, Added<Text>>) {
    for mut text_font in &mut query {
        text_font.font = ui_font.handle.clone();
    }
}

fn apply_ui_font_to_existing_text(
    ui_font: Res<UiFont>,
    mut query: Query<&mut TextFont>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }
    for mut text_font in &mut query {
        text_font.font = ui_font.handle.clone();
    }
    *initialized = true;
}

/// Stores `UiPlugin` state.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TextInputPlugin,
            PanelPlugin,
            ScrollAreaPlugin,
            SelectFieldPlugin,
        ))
        .add_systems(
            Startup,
            (setup_ui_font, setup_cell_inspector, setup_sim_control_ui),
        )
        .add_systems(
            Update,
            (apply_ui_font_to_existing_text, apply_ui_font_to_added_text),
        )
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
