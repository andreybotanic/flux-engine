use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    config::GasRegistry,
    editor::{is_cursor_over_ui, ActiveEditorTool, MainMenuState},
    input::camera::MainCamera,
    render::OverlayMode,
    save::WorldLoadState,
    simulation::gas::GasField,
    ui::palette,
    ui::panels::PanelManager,
    world::grid::{world_to_cell, WorldGrid},
};

#[derive(Component)]
pub(crate) struct CellInspectorText;

#[derive(Component)]
pub(crate) struct CellInspectorPanel;

/// Runs `setup_cell_inspector` logic.
pub(crate) fn setup_cell_inspector(mut commands: Commands) {
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
            BackgroundColor(palette::HUD_BG),
            CellInspectorPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FluxEngine loading..."),
                TextFont::from_font_size(13.0),
                TextColor(palette::TEXT_ON_DARK),
                CellInspectorText,
            ));
        });
}

/// Runs `update_cell_inspector` logic.
pub(crate) fn update_cell_inspector(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    active_tool: Res<ActiveEditorTool>,
    main_menu: Res<MainMenuState>,
    world_load_state: Res<WorldLoadState>,
    debug_mode: Res<crate::debug::DebugMode>,
    panel_manager: Res<PanelManager>,
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

    if !world_load_state.has_world {
        **visibility = Visibility::Hidden;
        return;
    }

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
        Some(&panel_manager),
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
        let panel_size = Vec2::new(310.0, 108.0);
        let panel_position = compute_hud_position(
            cursor_position,
            panel_size,
            Vec2::new(window.width(), window.height()),
            Vec2::new(24.0, 18.0),
        );
        node.left = Val::Px(panel_position.x);
        node.top = Val::Px(panel_position.y);
    }

    text.0 = message;
}

fn compute_hud_position(cursor: Vec2, panel_size: Vec2, viewport_size: Vec2, offset: Vec2) -> Vec2 {
    let mut x = cursor.x + offset.x;
    let mut y = cursor.y + offset.y;
    if x + panel_size.x > viewport_size.x {
        x = cursor.x - panel_size.x - offset.x;
    }
    if y + panel_size.y > viewport_size.y {
        y = cursor.y - panel_size.y - offset.y;
    }
    Vec2::new(
        x.clamp(0.0, (viewport_size.x - panel_size.x).max(0.0)),
        y.clamp(0.0, (viewport_size.y - panel_size.y).max(0.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::compute_hud_position;
    use bevy::prelude::*;

    #[test]
    fn hud_flips_to_left_and_up_near_screen_edge() {
        let pos = compute_hud_position(
            Vec2::new(1590.0, 890.0),
            Vec2::new(310.0, 108.0),
            Vec2::new(1600.0, 900.0),
            Vec2::new(24.0, 18.0),
        );
        assert!(pos.x < 1590.0);
        assert!(pos.y < 890.0);
    }
}
