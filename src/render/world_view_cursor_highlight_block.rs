const CURSOR_HIGHLIGHT_ALPHA: f32 = 0.46;
const CURSOR_HIGHLIGHT_INSET: f32 = 0.75;
const CURSOR_HIGHLIGHT_DASH_COUNT: usize = 4;
const CURSOR_HIGHLIGHT_GAP: f32 = CELL_SIZE * 0.08;

/// Runs `draw_cursor_cell_highlight` logic.
pub(crate) fn draw_cursor_cell_highlight(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    world_load_state: Res<WorldLoadState>,
    panels: Option<Res<PanelManager>>,
    main_menu: Option<Res<crate::editor::MainMenuState>>,
    mut gizmos: Gizmos,
) {
    if !world_load_state.has_world {
        return;
    }
    if main_menu.as_ref().map(|menu| menu.open).unwrap_or(false) {
        return;
    }

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    if panels
        .as_ref()
        .map(|panel_manager| panel_manager.is_cursor_over_any_panel(cursor_pos))
        .unwrap_or(false)
    {
        return;
    }

    let (camera, camera_transform) = *camera_query;
    let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };
    let Some(cursor_cell) = world_to_cell(cursor_world_pos) else {
        return;
    };

    for (start, end) in cursor_highlight_segments(cursor_cell) {
        gizmos.line_2d(
            start,
            end,
            Color::srgba(1.0, 1.0, 1.0, CURSOR_HIGHLIGHT_ALPHA),
        );
    }
}

fn cursor_highlight_segments(cell: UVec2) -> Vec<(Vec2, Vec2)> {
    let center = cell_center(cell.x, cell.y);
    let half = CELL_SIZE * 0.5 - CURSOR_HIGHLIGHT_INSET;
    let left = center.x - half;
    let right = center.x + half;
    let bottom = center.y - half;
    let top = center.y + half;
    let span = (right - left).max(0.0);
    let dash_length =
        ((span - CURSOR_HIGHLIGHT_GAP * (CURSOR_HIGHLIGHT_DASH_COUNT.saturating_sub(1) as f32))
            / CURSOR_HIGHLIGHT_DASH_COUNT as f32)
            .max(0.0);

    let mut segments = Vec::with_capacity(CURSOR_HIGHLIGHT_DASH_COUNT * 4);
    for index in 0..CURSOR_HIGHLIGHT_DASH_COUNT {
        let offset = index as f32 * (dash_length + CURSOR_HIGHLIGHT_GAP);
        let start_x = left + offset;
        let end_x = (start_x + dash_length).min(right);
        let start_y = bottom + offset;
        let end_y = (start_y + dash_length).min(top);

        segments.push((Vec2::new(start_x, top), Vec2::new(end_x, top)));
        segments.push((Vec2::new(start_x, bottom), Vec2::new(end_x, bottom)));
        segments.push((Vec2::new(left, start_y), Vec2::new(left, end_y)));
        segments.push((Vec2::new(right, start_y), Vec2::new(right, end_y)));
    }
    segments
}
