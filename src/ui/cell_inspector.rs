use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    config::GasRegistry,
    editor::{is_cursor_over_ui, ActiveEditorTool, MainMenuState},
    input::camera::MainCamera,
    render::OverlayMode,
    simulation::gas::GasField,
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
            BackgroundColor(Color::srgba(0.12, 0.14, 0.16, 0.90)),
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
    active_tool: Res<ActiveEditorTool>,
    main_menu: Res<MainMenuState>,
    debug_mode: Res<crate::debug::DebugMode>,
    gas: Res<GasField>,
    gas_registry: Res<GasRegistry>,
    world: Res<WorldGrid>,
    overlay_mode: Res<OverlayMode>,
    mut text_query: Single<&mut Text, With<CellInspectorText>>,
    mut panel_query: Single<(&mut Node, &mut Visibility), With<CellInspectorPanel>>,
) {
    let (camera, camera_transform) = *camera_query;
    let text = &mut *text_query;
    let (node, visibility) = &mut *panel_query;

    let overlay_name = match *overlay_mode {
        OverlayMode::Main => "F1 Main",
        OverlayMode::Gas => "F2 Gas",
    };

    let Some(cursor_position) = window.cursor_position() else {
        **visibility = Visibility::Hidden;
        return;
    };

    if is_cursor_over_ui(
        cursor_position,
        &window,
        debug_mode.active,
        active_tool.selected,
        main_menu.open,
    ) {
        **visibility = Visibility::Hidden;
        return;
    }

    let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    else {
        **visibility = Visibility::Hidden;
        return;
    };
    let Some(cell) = world_to_cell(cursor_world_pos) else {
        **visibility = Visibility::Hidden;
        return;
    };
    **visibility = Visibility::Visible;

    let cell_kind = if world.is_solid(cell.x, cell.y) {
        "solid"
    } else {
        "empty"
    };
    let mut gas_lines = String::new();
    for (gas_index, gas_def) in gas_registry.all().iter().enumerate() {
        let amount = gas.amount_rounded(cell.x, cell.y, gas_index);
        gas_lines.push_str(&format!(
            "{}: {} particles\n",
            gas_def.id.to_uppercase(),
            amount
        ));
    }
    let total = gas.total_amount_rounded(cell.x, cell.y);
    let message = format!(
        "{overlay_name}\n({}, {}) {}\n{}Total: {} particles",
        cell.x, cell.y, cell_kind, gas_lines, total
    );

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
