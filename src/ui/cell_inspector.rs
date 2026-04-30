use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    input::camera::MainCamera,
    render::OverlayMode,
    simulation::gas::{GasField, GasKind},
    world::grid::{world_to_cell, WorldGrid},
};

#[derive(Component)]
pub(crate) struct CellInspectorText;

#[derive(Component)]
pub(crate) struct CellInspectorPanel;

pub fn setup_cell_inspector(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                width: Val::Px(310.0),
                min_height: Val::Px(108.0),
                padding: UiRect::all(Val::Px(7.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.09, 0.12, 0.72)),
            CellInspectorPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FluxEngine loading..."),
                TextFont::from_font_size(13.0),
                TextColor(Color::WHITE),
                CellInspectorText,
            ));
        });
}

pub fn update_cell_inspector(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    gas: Res<GasField>,
    world: Res<WorldGrid>,
    overlay_mode: Res<OverlayMode>,
    mut text_query: Single<&mut Text, With<CellInspectorText>>,
    mut panel_query: Single<&mut Node, With<CellInspectorPanel>>,
) {
    let (camera, camera_transform) = *camera_query;
    let text = &mut *text_query;
    let node = &mut *panel_query;

    let overlay_name = match *overlay_mode {
        OverlayMode::Main => "F1 Main",
        OverlayMode::Gas => "F2 Gas",
    };

    let message = match window.cursor_position() {
        Some(cursor_position) => match camera.viewport_to_world_2d(camera_transform, cursor_position) {
            Ok(cursor_world_pos) => match world_to_cell(cursor_world_pos) {
                Some(cell) => {
                    let cell_kind = if world.is_solid(cell.x, cell.y) { "solid" } else { "empty" };
                    let h2 = gas.amount(cell.x, cell.y, GasKind::Hydrogen);
                    let o2 = gas.amount(cell.x, cell.y, GasKind::Oxygen);
                    let total = gas.total_amount(cell.x, cell.y);
                    format!(
                        "{overlay_name}\n({}, {}) {}\nH2: {} particles\nO2: {} particles\nTotal: {} particles",
                        cell.x, cell.y, cell_kind, h2, o2, total
                    )
                }
                None => format!("{overlay_name}\noutside grid"),
            },
            Err(_) => format!("{overlay_name}\nunavailable"),
        },
        None => format!("{overlay_name}\nunavailable"),
    };

    if let Some(cursor_position) = window.cursor_position() {
        let panel_offset = Vec2::new(12.0, 12.0);
        let panel_size = Vec2::new(310.0, 108.0);

        let max_left = (window.width() - panel_size.x).max(0.0);
        let max_top = (window.height() - panel_size.y).max(0.0);

        node.left = Val::Px((cursor_position.x + panel_offset.x).clamp(0.0, max_left));
        node.top = Val::Px((cursor_position.y + panel_offset.y).clamp(0.0, max_top));
    }

    text.0 = message;
}
