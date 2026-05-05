fn handle_editor_mouse_input(
    input_state: (
        Res<ButtonInput<MouseButton>>,
        Single<&Window, With<PrimaryWindow>>,
        Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ),
    tool_state: (
        Res<ActiveEditorTool>,
        Res<CellToolSettings>,
        ResMut<SourceStructureToolSettings>,
        ResMut<SinkStructureToolSettings>,
        Res<MainMenuState>,
        Res<WorldLoadState>,
        Res<DebugMode>,
    ),
    ui_tool_state: (
        Res<GasToolSettings>,
        Res<GasRegistry>,
        Res<PanelManager>,
        ResMut<SelectFieldState>,
    ),
    mut field_state: ParamSet<(
        Query<&TextInputField, With<GasAmountInputField>>,
        Query<&mut TextInputField, With<SourceAmountInputField>>,
        Query<&mut TextInputField, With<SinkAmountInputField>>,
    )>,
    data_state: (
        ResMut<WorldGrid>,
        ResMut<GasStructureGrid>,
        ResMut<GasField>,
        ResMut<StructureEditState>,
        ResMut<SelectionDragState>,
        ResMut<BrushDragState>,
        EventWriter<WorldCellChanged>,
    ),
) {
    let (mouse_buttons, window, camera_query) = input_state;
    let (
        active_tool,
        cell_settings,
        mut source_settings,
        mut sink_settings,
        main_menu,
        world_load_state,
        debug_mode,
    ) = tool_state;
    let (gas_settings, gas_registry, panel_manager, mut select_fields) = ui_tool_state;
    let (
        mut world,
        mut structures,
        mut gas,
        mut structure_edit,
        mut selection_drag,
        mut brush_drag,
        mut world_changed,
    ) = data_state;

    if main_menu.open || !world_load_state.has_world {
        structure_edit.selected_cell = None;
        clear_active_tool_state(&mut selection_drag, &mut brush_drag);
        return;
    }

    let cursor_position = window.cursor_position();
    let blocked_by_ui = cursor_position
        .map(|cursor| {
            is_cursor_over_ui(
                cursor,
                &window,
                debug_mode.active,
                active_tool.selected,
                main_menu.open,
                Some(&panel_manager),
            )
        })
        .unwrap_or(false);

    let hovered_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    match active_tool.selected {
        Some(EditorTool::BuildSolid) => {
            structure_edit.selected_cell = None;
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    if structures.blocks_solid_placement(cell.x, cell.y) {
                        return;
                    }
                    if world.set_solid_with_material(cell.x, cell.y, cell_settings.material) {
                        gas.clear_cell(cell.x, cell.y);
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        Some(EditorTool::EraseSolid) => {
            structure_edit.selected_cell = None;
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    let _ = structures.clear(cell.x, cell.y);
                    if world.set_empty(cell.x, cell.y) {
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        Some(EditorTool::AddGas) | Some(EditorTool::ClearGas) => {
            structure_edit.selected_cell = None;
            if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                if let Some(cell) = hovered_cell {
                    selection_drag.active = true;
                    selection_drag.start = Some(cell);
                    selection_drag.current = Some(cell);
                }
            }

            if selection_drag.active && mouse_buttons.pressed(MouseButton::Left) {
                if let Some(cell) = hovered_cell {
                    selection_drag.current = Some(cell);
                }
            }

            if selection_drag.active && mouse_buttons.just_released(MouseButton::Left) {
                if let (Some(start), Some(end)) = (selection_drag.start, selection_drag.current) {
                    let (min, max) = normalized_rect(start, end);
                    let amount = field_state
                        .p0()
                        .single()
                        .ok()
                        .and_then(|f| f.parsed_u32())
                        .unwrap_or(gas_settings.amount);
                    match active_tool.selected {
                        Some(EditorTool::AddGas) => {
                            if gas_registry.count() == 0 {
                                selection_drag.active = false;
                                selection_drag.start = None;
                                selection_drag.current = None;
                                return;
                            }
                            gas.apply_rect(
                                min,
                                max,
                                gas_settings.gas_index.min(gas_registry.count() - 1),
                                amount,
                                gas_settings.replace,
                                &world,
                            );
                        }
                        Some(EditorTool::ClearGas) => {
                            gas.clear_rect(min, max);
                        }
                        _ => {}
                    }
                }

                selection_drag.active = false;
                selection_drag.start = None;
                selection_drag.current = None;
            }
        }
        Some(EditorTool::CreateGasSource) => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                if let Some(cell) = hovered_cell {
                    if gas_registry.count() > 0 {
                        let gas_index = source_settings.gas_index.min(gas_registry.count() - 1);
                        if structures.set_source(
                            cell.x,
                            cell.y,
                            gas_index,
                            source_settings.amount.max(1),
                            &world,
                        ) {
                            structure_edit.selected_cell = Some(cell);
                        }
                    }
                }
            }
        }
        Some(EditorTool::CreateGasSink) => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                if let Some(cell) = hovered_cell {
                    if structures.set_sink(cell.x, cell.y, sink_settings.amount.max(1), &world) {
                        structure_edit.selected_cell = Some(cell);
                    }
                }
            }
        }
        None => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            if debug_mode.active
                && mouse_buttons.just_pressed(MouseButton::Left)
                && !blocked_by_ui
            {
                structure_edit.selected_cell =
                    hovered_cell.filter(|cell| structures.cell(cell.x, cell.y).is_some());
                if let Some(cell) = structure_edit.selected_cell {
                    match structures.cell(cell.x, cell.y) {
                        Some(GasStructureCell::Source { gas_index, amount }) => {
                            source_settings.gas_index = gas_index;
                            select_fields.set_selected(GAS_SELECT_SOURCE_ID, gas_index);
                            source_settings.amount = amount.max(1);
                            if let Ok(mut source_amount_input) = field_state.p1().single_mut() {
                                source_amount_input.text = source_settings.amount.to_string();
                                source_amount_input.cursor = source_amount_input.text.chars().count();
                                source_amount_input.value =
                                    crate::ui::input_field::ParsedInputValue::U32(
                                        source_settings.amount,
                                    );
                            }
                        }
                        Some(GasStructureCell::Sink { amount }) => {
                            sink_settings.amount = amount.max(1);
                            if let Ok(mut sink_amount_input) = field_state.p2().single_mut() {
                                sink_amount_input.text = sink_settings.amount.to_string();
                                sink_amount_input.cursor = sink_amount_input.text.chars().count();
                                sink_amount_input.value =
                                    crate::ui::input_field::ParsedInputValue::U32(sink_settings.amount);
                            }
                        }
                        None => {}
                    }
                }
            }
        }
    }
}

fn apply_brush_tool(
    mouse_buttons: &ButtonInput<MouseButton>,
    blocked_by_ui: bool,
    hovered_cell: Option<UVec2>,
    brush_drag: &mut BrushDragState,
    mut apply_cell: impl FnMut(UVec2),
) {
    if mouse_buttons.just_released(MouseButton::Left) {
        brush_drag.active = false;
        brush_drag.last_cell = None;
        return;
    }

    if blocked_by_ui {
        return;
    }

    if mouse_buttons.just_pressed(MouseButton::Left) {
        brush_drag.active = true;
        brush_drag.last_cell = None;
    }

    if !brush_drag.active || !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    let Some(cell) = hovered_cell else {
        return;
    };

    if brush_drag.last_cell == Some(cell) {
        return;
    }

    apply_cell(cell);
    brush_drag.last_cell = Some(cell);
}

fn update_editor_cursor_overlays(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    active_tool: Res<ActiveEditorTool>,
    cell_settings: Res<CellToolSettings>,
    main_menu: Res<MainMenuState>,
    world_load_state: Res<WorldLoadState>,
    overlay_ui_state: (Res<EditorIconSet>, Res<DebugMode>, Res<PanelManager>),
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut overlay_set: ParamSet<(
        Single<(&mut Transform, &mut Visibility, &mut Sprite), With<BlueprintGhost>>,
        Single<(&mut Transform, &mut Visibility), With<EraseCellHighlight>>,
        Single<(&mut Node, &mut Visibility), With<EraseCursorOverlay>>,
    )>,
) {
    let (icon_set, debug_mode, panel_manager) = overlay_ui_state;

    if !world_load_state.has_world {
        {
            let mut blueprint = overlay_set.p0();
            let (_, ghost_visibility, _) = &mut *blueprint;
            **ghost_visibility = Visibility::Hidden;
        }
        {
            let mut erase_highlight = overlay_set.p1();
            let (_, highlight_visibility) = &mut *erase_highlight;
            **highlight_visibility = Visibility::Hidden;
        }
        {
            let mut erase_overlay = overlay_set.p2();
            let (_, erase_visibility) = &mut *erase_overlay;
            **erase_visibility = Visibility::Hidden;
        }
        return;
    }

    let cursor_position = window.cursor_position();
    let is_on_ui = cursor_position
        .map(|cursor| {
            is_cursor_over_ui(
                cursor,
                &window,
                debug_mode.active,
                active_tool.selected,
                main_menu.open,
                Some(&panel_manager),
            )
        })
        .unwrap_or(false);

    let world_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    {
        let mut blueprint = overlay_set.p0();
        let (ghost_transform, ghost_visibility, ghost_sprite) = &mut *blueprint;
        let ghost_image = match active_tool.selected {
            Some(EditorTool::BuildSolid) => Some(match cell_settings.material {
                CellMaterial::Brick => icon_set.brick_silhouette.clone(),
                CellMaterial::Metal => icon_set.metal_silhouette.clone(),
                CellMaterial::Boundary => icon_set.brick_silhouette.clone(),
            }),
            Some(EditorTool::CreateGasSource) => Some(icon_set.source_silhouette.clone()),
            Some(EditorTool::CreateGasSink) => Some(icon_set.sink_silhouette.clone()),
            _ => None,
        };
        if !is_on_ui && !main_menu.open && !mouse_buttons.pressed(MouseButton::Left) {
            if let (Some(image), Some(cell)) = (ghost_image, world_cell) {
                ghost_sprite.image = image;
                **ghost_transform =
                    Transform::from_translation(cell_center(cell.x, cell.y).extend(1.8));
                **ghost_visibility = Visibility::Visible;
            } else {
                **ghost_visibility = Visibility::Hidden;
            }
        } else {
            **ghost_visibility = Visibility::Hidden;
        }
    }

    {
        let mut erase_highlight = overlay_set.p1();
        let (highlight_transform, highlight_visibility) = &mut *erase_highlight;
        if active_tool.selected == Some(EditorTool::EraseSolid) && !is_on_ui && !main_menu.open {
            if let Some(cell) = world_cell {
                **highlight_transform =
                    Transform::from_translation(cell_center(cell.x, cell.y).extend(1.79));
                **highlight_visibility = Visibility::Visible;
            } else {
                **highlight_visibility = Visibility::Hidden;
            }
        } else {
            **highlight_visibility = Visibility::Hidden;
        }
    }

    {
        let mut erase_overlay = overlay_set.p2();
        let (erase_node, erase_visibility) = &mut *erase_overlay;
        if active_tool.selected == Some(EditorTool::EraseSolid) && !main_menu.open {
            if let Some(cursor) = cursor_position {
                erase_node.left = Val::Px(cursor.x + 10.0);
                erase_node.top = Val::Px(cursor.y + 8.0);
                **erase_visibility = Visibility::Visible;
            } else {
                **erase_visibility = Visibility::Hidden;
            }
        } else {
            **erase_visibility = Visibility::Hidden;
        }
    }
}

fn draw_selection_overlay(selection_drag: Res<SelectionDragState>, mut gizmos: Gizmos) {
    if !selection_drag.active {
        return;
    }

    let (Some(start), Some(current)) = (selection_drag.start, selection_drag.current) else {
        return;
    };

    let (min, max) = normalized_rect(start, current);

    let min_center = cell_center(min.x, min.y);
    let max_center = cell_center(max.x, max.y);
    let center = (min_center + max_center) * 0.5;

    let size = Vec2::new(
        (max.x - min.x + 1) as f32 * CELL_SIZE - 1.0,
        (max.y - min.y + 1) as f32 * CELL_SIZE - 1.0,
    );

    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size,
        crate::ui::palette::WARNING_ACCENT,
    );
}

fn normalized_rect(a: UVec2, b: UVec2) -> (UVec2, UVec2) {
    let min = UVec2::new(a.x.min(b.x), a.y.min(b.y));
    let max = UVec2::new(a.x.max(b.x), a.y.max(b.y));
    (min, max)
}

fn viewport_cursor_to_cell(
    cursor: Vec2,
    camera_query: &Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<UVec2> {
    let (camera, camera_transform) = **camera_query;
    let world_pos = camera.viewport_to_world_2d(camera_transform, cursor).ok()?;
    world_to_cell(world_pos)
}

pub(crate) fn is_cursor_over_ui(
    cursor: Vec2,
    window: &Window,
    debug_mode_active: bool,
    selected_tool: Option<EditorTool>,
    main_menu_open: bool,
    panel_manager: Option<&PanelManager>,
) -> bool {
    let mut rects = vec![
        UiRectPx::top_left(
            12.0,
            12.0,
            TOP_LEFT_SIM_PANEL_WIDTH,
            TOP_LEFT_SIM_PANEL_HEIGHT,
        ),
        UiRectPx::top_left(
            MAIN_TOOLBAR_LEFT,
            window.height() - MAIN_TOOLBAR_BOTTOM - MAIN_TOOLBAR_HEIGHT,
            MAIN_TOOLBAR_WIDTH,
            MAIN_TOOLBAR_HEIGHT,
        ),
    ];

    if selected_tool == Some(EditorTool::BuildSolid) {
        rects.push(UiRectPx::top_left(
            MAIN_TOOLBAR_LEFT,
            window.height() - CELL_TYPE_PANEL_BOTTOM - CELL_TYPE_PANEL_HEIGHT,
            CELL_TYPE_PANEL_WIDTH,
            CELL_TYPE_PANEL_HEIGHT,
        ));
    }

    if debug_mode_active {
        rects.push(UiRectPx::top_left(
            DEBUG_TOOLBAR_LEFT,
            DEBUG_TOOLBAR_TOP,
            DEBUG_TOOLBAR_WIDTH,
            DEBUG_TOOLBAR_HEIGHT,
        ));
    }

    if main_menu_open {
        rects.push(UiRectPx::top_left(
            0.0,
            0.0,
            window.width(),
            window.height(),
        ));
    }

    rects.into_iter().any(|rect| rect.contains(cursor))
        || panel_manager
            .map(|panels| panels.is_cursor_over_any_panel(cursor))
            .unwrap_or(false)
}

#[derive(Clone, Copy)]
struct UiRectPx {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

