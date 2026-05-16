fn handle_editor_mouse_input(
    input_state: (
        Res<ButtonInput<MouseButton>>,
        Res<ButtonInput<KeyCode>>,
        Single<&Window, With<PrimaryWindow>>,
        Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ),
    tool_state: (
        ResMut<ActiveEditorTool>,
        Res<ActiveEntityCategory>,
        Res<EntityCategoryUiRegistry>,
        Res<CellToolSettings>,
        Res<PipeToolSettings>,
        ResMut<BridgePlacementState>,
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
        Res<MainToolbarLayout>,
        ResMut<SelectFieldState>,
    ),
    mut field_state: ParamSet<(
        Query<&TextInputField, With<GasAmountInputField>>,
        Query<&mut TextInputField, With<SourceAmountInputField>>,
        Query<&mut TextInputField, With<SinkAmountInputField>>,
    )>,
    data_state: (
        ResMut<WorldGrid>,
        ResMut<PlacedStructureMap>,
        ResMut<GasField>,
        ResMut<crate::plugins::default_plugin::pipe_runtime::PipeGasField>,
        ResMut<crate::plugins::default_plugin::pipe_runtime::PipeFlowVisualState>,
        ResMut<StructureEditState>,
        ResMut<SelectionDragState>,
        ResMut<BrushDragState>,
        EventWriter<WorldCellChanged>,
        EventWriter<PluginRuntimeEvent>,
    ),
) {
    let (mouse_buttons, keyboard, window, camera_query) = input_state;
    let (
        mut active_tool,
        active_entity_category,
        category_ui_registry,
        cell_settings,
        pipe_settings,
        mut bridge_state,
        mut source_settings,
        mut sink_settings,
        main_menu,
        world_load_state,
        debug_mode,
    ) = tool_state;
    let (gas_settings, gas_registry, panel_manager, main_toolbar_layout, mut select_fields) =
        ui_tool_state;
    let (
        mut world,
        mut structures,
        mut gas,
        mut pipe_gas,
        mut pipe_flow_visuals,
        mut structure_edit,
        mut selection_drag,
        mut brush_drag,
        mut world_changed,
        mut plugin_events,
    ) = data_state;
    let selected_category_kind = active_entity_category
        .selected
        .as_ref()
        .and_then(|category_id| category_ui_registry.by_id(category_id))
        .map(|descriptor| descriptor.kind);

    if !main_menu.open && world_load_state.has_world {
        for key in [KeyCode::KeyX, KeyCode::KeyC] {
            if !keyboard.just_pressed(key) {
                continue;
            }
            if let Some(tool) = editor_tool_for_hotkey(key) {
                active_tool_toggle(
                    &mut selection_drag,
                    &mut brush_drag,
                    &mut active_tool,
                    &mut structure_edit,
                    tool,
                );
            }
        }
    }
    if keyboard.just_pressed(KeyCode::KeyR)
        && !main_menu.open
        && world_load_state.has_world
        && active_tool.selected == Some(EditorTool::Construct)
        && selected_category_kind == Some(EntityCategoryKind::Gases)
        && pipe_settings.selected == PipeToolKind::Bridge
    {
        bridge_state.rotation = bridge_state.rotation.next_bridge_rotation();
    }

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
                *main_toolbar_layout,
                main_menu.open,
                Some(&panel_manager),
            )
        })
        .unwrap_or(false);

    let hovered_world_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_world_cell(cursor, &camera_query));
    let hovered_cell = hovered_world_cell.map(|(_, cell)| cell);

    emit_plugin_mouse_cell_events(
        &mouse_buttons,
        &keyboard,
        cursor_position,
        hovered_world_cell,
        blocked_by_ui,
        active_tool.selected,
        active_entity_category.selected.as_ref(),
        &mut plugin_events,
    );

    match active_tool.selected {
        Some(EditorTool::Construct) if selected_category_kind == Some(EntityCategoryKind::Cells) => {
            structure_edit.selected_cell = None;
            if let Some(selected_material) = cell_settings.material {
                apply_brush_tool(
                    &mouse_buttons,
                    blocked_by_ui,
                    hovered_cell,
                    &mut brush_drag,
                    |cell| {
                        if structures.blocks_solid_placement(cell.x, cell.y) {
                            return;
                        }
                        if world.set_solid_with_material(cell.x, cell.y, selected_material) {
                            gas.clear_cell(cell.x, cell.y);
                            world_changed.write(WorldCellChanged { cell });
                        }
                    },
                );
            } else {
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
        }
        Some(EditorTool::EraseSolid) => {
            structure_edit.selected_cell = None;
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    let had_structures = !structures.structure_ids_at(cell.x, cell.y).is_empty();
                    if had_structures {
                        pipe_gas.sync_to_structures(&structures);
                        pipe_gas.clear_cell(&structures, cell.x, cell.y);
                        let _ = structures.clear_cell(cell.x, cell.y);
                        pipe_flow_visuals.reset_flow();
                    }
                    if world.set_empty(cell.x, cell.y) {
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        Some(EditorTool::Construct) if selected_category_kind == Some(EntityCategoryKind::Gases) => {
            match pipe_settings.selected {
            PipeToolKind::Pipe => {
                structure_edit.selected_cell = None;
                let mut changed_pipe_layout = false;
                apply_path_tool(
                    &mouse_buttons,
                    blocked_by_ui,
                    hovered_cell,
                    &mut brush_drag,
                    |path| {
                        changed_pipe_layout |= structures.apply_pipe_path(path, &world);
                    },
                );
                if changed_pipe_layout {
                    pipe_flow_visuals.reset_flow();
                }
            }
            PipeToolKind::Vent => {
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
                structure_edit.selected_cell = None;
                if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                    if let Some(cell) = hovered_cell {
                        if structures.place_vent(cell.x, cell.y, &world) {
                            pipe_flow_visuals.reset_flow();
                        }
                    }
                }
            }
            PipeToolKind::Bridge => {
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
                structure_edit.selected_cell = None;
                if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                    if let Some(cell) = hovered_cell {
                        if structures
                            .place_bridge(cell, bridge_state.rotation, &world)
                            .is_some()
                        {
                            pipe_flow_visuals.reset_flow();
                        }
                    }
                }
            }
            }
        }
        Some(EditorTool::Construct) => {
            structure_edit.selected_cell = None;
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
        }
        Some(EditorTool::Scissors) => {
            structure_edit.selected_cell = None;
            let mut removed_connection = false;
            apply_path_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |path| {
                    for pair in path.windows(2) {
                        removed_connection |= structures.add_pipe_cut(pair[0], pair[1]);
                    }
                },
            );
            if removed_connection {
                pipe_flow_visuals.reset_flow();
            }
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
                    if gas_registry.count() > 0 && !structures.blocks_solid_placement(cell.x, cell.y) {
                        let gas_index = source_settings.gas_index.min(gas_registry.count() - 1);
                        if structures.place_gas_source(
                            cell.x,
                            cell.y,
                            gas_index,
                            source_settings.amount.max(1),
                            &world,
                        ).is_some() {
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
                    if !structures.blocks_solid_placement(cell.x, cell.y)
                        && structures
                            .place_gas_sink(cell.x, cell.y, sink_settings.amount.max(1), &world)
                            .is_some()
                    {
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
                structure_edit.selected_cell = hovered_cell
                    .filter(|cell| structures.editable_structure_at(cell.x, cell.y).is_some());
                if let Some(cell) = structure_edit.selected_cell {
                    match structures
                        .editable_structure_at(cell.x, cell.y)
                        .map(|structure| structure.params)
                    {
                        Some(StructureParams::GasSource { gas_index, amount }) => {
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
                        Some(StructureParams::GasSink { amount }) => {
                            sink_settings.amount = amount.max(1);
                            if let Ok(mut sink_amount_input) = field_state.p2().single_mut() {
                                sink_amount_input.text = sink_settings.amount.to_string();
                                sink_amount_input.cursor = sink_amount_input.text.chars().count();
                                sink_amount_input.value =
                                    crate::ui::input_field::ParsedInputValue::U32(sink_settings.amount);
                            }
                        }
                        _ => {}
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

fn apply_path_tool(
    mouse_buttons: &ButtonInput<MouseButton>,
    blocked_by_ui: bool,
    hovered_cell: Option<UVec2>,
    brush_drag: &mut BrushDragState,
    mut apply_path: impl FnMut(&[UVec2]),
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

    let path = if let Some(last_cell) = brush_drag.last_cell {
        rasterize_grid_path(last_cell, cell)
    } else {
        vec![cell]
    };
    if path.is_empty() {
        return;
    }
    apply_path(&path);
    brush_drag.last_cell = Some(cell);
}

fn rasterize_grid_path(from: UVec2, to: UVec2) -> Vec<UVec2> {
    let mut path = Vec::new();
    let mut x0 = from.x as i32;
    let mut y0 = from.y as i32;
    let x1 = to.x as i32;
    let y1 = to.y as i32;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let cell = UVec2::new(x0 as u32, y0 as u32);
        if path.last().copied() != Some(cell) {
            path.push(cell);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    let mut expanded = Vec::new();
    if let Some(first) = path.first().copied() {
        expanded.push(first);
    }
    for pair in path.windows(2) {
        let mut current = pair[0];
        let target = pair[1];
        while current != target {
            let dx = target.x as i32 - current.x as i32;
            let dy = target.y as i32 - current.y as i32;
            let next = if dx != 0 {
                UVec2::new((current.x as i32 + dx.signum()) as u32, current.y)
            } else {
                UVec2::new(current.x, (current.y as i32 + dy.signum()) as u32)
            };
            if expanded.last().copied() != Some(next) {
                expanded.push(next);
            }
            current = next;
        }
    }
    expanded
}

fn active_tool_toggle(
    selection_drag: &mut SelectionDragState,
    brush_drag: &mut BrushDragState,
    active_tool: &mut ActiveEditorTool,
    structure_edit: &mut StructureEditState,
    tool: EditorTool,
) {
    active_tool.selected = toggled_editor_tool(active_tool.selected, tool);
    structure_edit.selected_cell = None;
    clear_active_tool_state(selection_drag, brush_drag);
}

fn toggled_editor_tool(current: Option<EditorTool>, tool: EditorTool) -> Option<EditorTool> {
    if current == Some(tool) {
        None
    } else {
        Some(tool)
    }
}

fn editor_tool_for_hotkey(key: KeyCode) -> Option<EditorTool> {
    match key {
        KeyCode::KeyX => Some(EditorTool::EraseSolid),
        KeyCode::KeyC => Some(EditorTool::Scissors),
        _ => None,
    }
}

fn update_editor_cursor_overlays(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    active_tool: Res<ActiveEditorTool>,
    active_entity_category: Res<ActiveEntityCategory>,
    category_ui_registry: Res<EntityCategoryUiRegistry>,
    cell_settings: Res<CellToolSettings>,
    pipe_settings: Res<PipeToolSettings>,
    bridge_state: Res<BridgePlacementState>,
    structure_visuals: Res<crate::config::StructureVisualConfigMap>,
    cell_visual_layouts: Res<crate::config::CellVisualPlacementConfigMap>,
    main_menu: Res<MainMenuState>,
    world_load_state: Res<WorldLoadState>,
    overlay_ui_state: (
        Res<EditorIconSet>,
        Res<DebugMode>,
        Res<PanelManager>,
        Res<MainToolbarLayout>,
        Res<AssetServer>,
        Res<crate::plugins::ContentRegistry>,
    ),
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut overlay_set: ParamSet<(
        Single<(&mut Transform, &mut Visibility, &mut Sprite), With<BlueprintGhost>>,
        Single<(&mut Transform, &mut Visibility), With<EraseCellHighlight>>,
        Single<(&mut Node, &mut Visibility), With<EraseCursorOverlay>>,
    )>,
    mut overlay_text: Single<&mut Text, With<EraseCursorOverlayText>>,
) {
    let (icon_set, debug_mode, panel_manager, main_toolbar_layout, asset_server, content_registry) =
        overlay_ui_state;
    let selected_category_kind = active_entity_category
        .selected
        .as_ref()
        .and_then(|category_id| category_ui_registry.by_id(category_id))
        .map(|descriptor| descriptor.kind);

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
                *main_toolbar_layout,
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
            Some(EditorTool::Construct)
                if selected_category_kind == Some(EntityCategoryKind::Cells) =>
            {
                cell_settings.material.and_then(|material| {
                    content_registry
                    .cell_by_material(material)
                    .map(|descriptor| {
                        let ghost_path = descriptor
                            .sprite
                            .silhouette_path
                            .as_deref()
                            .unwrap_or(descriptor.sprite.image_path.as_str());
                        asset_server.load(ghost_path)
                    })
                })
            }
            Some(EditorTool::Construct) if selected_category_kind == Some(EntityCategoryKind::Gases) => {
                Some(match pipe_settings.selected {
                PipeToolKind::Pipe => icon_set.pipe_silhouette.clone(),
                PipeToolKind::Vent => icon_set.vent_silhouette.clone(),
                PipeToolKind::Bridge => icon_set.bridge_silhouette.clone(),
                })
            }
            Some(EditorTool::CreateGasSource) => Some(icon_set.source_silhouette.clone()),
            Some(EditorTool::CreateGasSink) => Some(icon_set.sink_silhouette.clone()),
            _ => None,
        };
        if !is_on_ui && !main_menu.open && !mouse_buttons.pressed(MouseButton::Left) {
            if let (Some(image), Some(cell)) = (ghost_image, world_cell) {
                ghost_sprite.image = image;
                let size_in_cells = match active_tool.selected {
                    Some(EditorTool::Construct)
                        if selected_category_kind == Some(EntityCategoryKind::Cells) =>
                    {
                        cell_settings
                            .material
                            .map(|material| {
                                crate::world::structures::cell_material_sprite_size_in_cells(
                                    material,
                                    &cell_visual_layouts,
                                )
                            })
                            .unwrap_or(UVec2::ONE)
                    }
                    Some(EditorTool::Construct)
                        if selected_category_kind == Some(EntityCategoryKind::Gases) =>
                    {
                        crate::world::structures::structure_footprint_size_in_cells(
                            selected_pipe_structure_kind(pipe_settings.selected),
                            bridge_state.rotation,
                            &structure_visuals,
                        )
                    }
                    Some(EditorTool::CreateGasSource) => {
                        crate::world::structures::structure_footprint_size_in_cells(
                            crate::plugins::default_plugin::gas_source_structure_kind(),
                            StructureRotation::Deg0,
                            &structure_visuals,
                        )
                    }
                    Some(EditorTool::CreateGasSink) => {
                        crate::world::structures::structure_footprint_size_in_cells(
                            crate::plugins::default_plugin::gas_sink_structure_kind(),
                            StructureRotation::Deg0,
                            &structure_visuals,
                        )
                    }
                    _ => UVec2::ONE,
                };
                let sprite_size_in_cells = match active_tool.selected {
                    Some(EditorTool::Construct)
                        if selected_category_kind == Some(EntityCategoryKind::Cells) =>
                    {
                        cell_settings
                            .material
                            .map(|material| {
                                crate::world::structures::cell_material_sprite_size_in_cells(
                                    material,
                                    &cell_visual_layouts,
                                )
                            })
                            .unwrap_or(UVec2::ONE)
                    }
                    Some(EditorTool::Construct)
                        if selected_category_kind == Some(EntityCategoryKind::Gases) =>
                    {
                        crate::world::structures::structure_sprite_size_in_cells(
                            selected_pipe_structure_kind(pipe_settings.selected),
                            bridge_state.rotation,
                            &structure_visuals,
                        )
                    }
                    Some(EditorTool::CreateGasSource) => {
                        crate::world::structures::structure_sprite_size_in_cells(
                            crate::plugins::default_plugin::gas_source_structure_kind(),
                            StructureRotation::Deg0,
                            &structure_visuals,
                        )
                    }
                    Some(EditorTool::CreateGasSink) => {
                        crate::world::structures::structure_sprite_size_in_cells(
                            crate::plugins::default_plugin::gas_sink_structure_kind(),
                            StructureRotation::Deg0,
                            &structure_visuals,
                        )
                    }
                    _ => UVec2::ONE,
                };
                ghost_sprite.custom_size = Some(Vec2::new(
                    sprite_size_in_cells.x.max(1) as f32 * CELL_SIZE,
                    sprite_size_in_cells.y.max(1) as f32 * CELL_SIZE,
                ));
                let translation = cell_center(cell.x, cell.y)
                    + Vec2::new(
                        size_in_cells.x.saturating_sub(1) as f32 * CELL_SIZE * 0.5,
                        size_in_cells.y.saturating_sub(1) as f32 * CELL_SIZE * 0.5,
                    );
                ghost_sprite.color = if selected_category_kind == Some(EntityCategoryKind::Gases)
                    && pipe_settings.selected == PipeToolKind::Bridge
                {
                    Color::srgba(1.0, 1.0, 1.0, 0.82)
                } else {
                    Color::WHITE
                };
                let mut transform = Transform::from_translation(translation.extend(1.8));
                transform.rotation = match (active_tool.selected, pipe_settings.selected) {
                    (Some(EditorTool::Construct), PipeToolKind::Bridge)
                        if selected_category_kind == Some(EntityCategoryKind::Gases) =>
                    {
                        match bridge_state.rotation {
                        StructureRotation::Deg0 => Quat::IDENTITY,
                        StructureRotation::Deg90 => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                        StructureRotation::Deg180 => Quat::from_rotation_z(std::f32::consts::PI),
                        StructureRotation::Deg270 => Quat::from_rotation_z(std::f32::consts::PI * 1.5),
                        }
                    }
                    _ => Quat::IDENTITY,
                };
                **ghost_transform = transform;
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
        if matches!(
            active_tool.selected,
            Some(EditorTool::EraseSolid) | Some(EditorTool::Scissors)
        ) && !main_menu.open
        {
            if let Some(cursor) = cursor_position {
                erase_node.left = Val::Px(cursor.x + 10.0);
                erase_node.top = Val::Px(cursor.y + 8.0);
                overlay_text.0 = if active_tool.selected == Some(EditorTool::Scissors) {
                    "8<".to_string()
                } else {
                    "X".to_string()
                };
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

fn emit_plugin_mouse_cell_events(
    mouse_buttons: &ButtonInput<MouseButton>,
    keyboard: &ButtonInput<KeyCode>,
    cursor_position: Option<Vec2>,
    hovered_world_cell: Option<(Vec2, UVec2)>,
    is_over_ui: bool,
    selected_tool: Option<EditorTool>,
    selected_category_id: Option<&ContentId>,
    plugin_events: &mut EventWriter<PluginRuntimeEvent>,
) {
    let (Some(screen_position), Some((world_position, cell))) =
        (cursor_position, hovered_world_cell)
    else {
        return;
    };

    let modifiers = InputModifiers {
        shift: keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight),
        ctrl: keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight),
        alt: keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight),
    };
    let active_tool_id = active_tool_content_id(selected_tool, selected_category_id);
    for (button, event_builder) in [
        (
            MouseButton::Left,
            PluginRuntimeEvent::MouseDownCell as fn(MouseCellEvent) -> PluginRuntimeEvent,
        ),
        (
            MouseButton::Right,
            PluginRuntimeEvent::MouseDownCell as fn(MouseCellEvent) -> PluginRuntimeEvent,
        ),
        (
            MouseButton::Middle,
            PluginRuntimeEvent::MouseDownCell as fn(MouseCellEvent) -> PluginRuntimeEvent,
        ),
    ] {
        if mouse_buttons.just_pressed(button) {
            plugin_events.write(event_builder(MouseCellEvent {
                button: Some(mouse_button_to_plugin(button)),
                cell,
                world_position,
                screen_position,
                modifiers,
                active_tool_id: active_tool_id.clone(),
                is_over_ui,
            }));
        }
    }
    for (button, event_builder) in [
        (
            MouseButton::Left,
            PluginRuntimeEvent::MouseUpCell as fn(MouseCellEvent) -> PluginRuntimeEvent,
        ),
        (
            MouseButton::Right,
            PluginRuntimeEvent::MouseUpCell as fn(MouseCellEvent) -> PluginRuntimeEvent,
        ),
        (
            MouseButton::Middle,
            PluginRuntimeEvent::MouseUpCell as fn(MouseCellEvent) -> PluginRuntimeEvent,
        ),
    ] {
        if mouse_buttons.just_released(button) {
            plugin_events.write(event_builder(MouseCellEvent {
                button: Some(mouse_button_to_plugin(button)),
                cell,
                world_position,
                screen_position,
                modifiers,
                active_tool_id: active_tool_id.clone(),
                is_over_ui,
            }));
        }
    }
    if mouse_buttons.pressed(MouseButton::Left)
        || mouse_buttons.pressed(MouseButton::Right)
        || mouse_buttons.pressed(MouseButton::Middle)
    {
        plugin_events.write(PluginRuntimeEvent::MouseMoveCell(MouseCellEvent {
            button: None,
            cell,
            world_position,
            screen_position,
            modifiers,
            active_tool_id,
            is_over_ui,
        }));
    }
}

fn mouse_button_to_plugin(button: MouseButton) -> PluginMouseButton {
    match button {
        MouseButton::Left => PluginMouseButton::Left,
        MouseButton::Right => PluginMouseButton::Right,
        MouseButton::Middle => PluginMouseButton::Middle,
        MouseButton::Back => PluginMouseButton::Other(3),
        MouseButton::Forward => PluginMouseButton::Other(4),
        MouseButton::Other(value) => PluginMouseButton::Other(value),
    }
}

fn active_tool_content_id(
    selected_tool: Option<EditorTool>,
    selected_category_id: Option<&ContentId>,
) -> Option<ContentId> {
    if selected_tool == Some(EditorTool::Construct) {
        return selected_category_id
            .cloned()
            .or_else(|| ContentId::parse(CONSTRUCT_TOOL_CONTENT_ID).ok());
    }
    let raw = match selected_tool? {
        EditorTool::Construct => CONSTRUCT_TOOL_CONTENT_ID,
        EditorTool::EraseSolid => "flux.core.tool.erase_solid",
        EditorTool::Scissors => "flux.default.tool.scissors",
        EditorTool::AddGas => "flux.core.tool.add_gas",
        EditorTool::ClearGas => "flux.core.tool.clear_gas",
        EditorTool::CreateGasSource => "flux.default.tool.gas_source",
        EditorTool::CreateGasSink => "flux.default.tool.gas_sink",
    };
    ContentId::parse(raw).ok()
}

fn emit_plugin_keyboard_events(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut plugin_events: EventWriter<PluginRuntimeEvent>,
) {
    let modifiers = InputModifiers {
        shift: keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight),
        ctrl: keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight),
        alt: keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight),
    };
    for key in keyboard.get_just_pressed() {
        plugin_events.write(PluginRuntimeEvent::KeyPressed {
            key: format!("{key:?}"),
            modifiers,
        });
    }
    for key in keyboard.get_just_released() {
        plugin_events.write(PluginRuntimeEvent::KeyReleased {
            key: format!("{key:?}"),
            modifiers,
        });
    }
}

fn viewport_cursor_to_cell(
    cursor: Vec2,
    camera_query: &Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<UVec2> {
    viewport_cursor_to_world_cell(cursor, camera_query).map(|(_, cell)| cell)
}

fn viewport_cursor_to_world_cell(
    cursor: Vec2,
    camera_query: &Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<(Vec2, UVec2)> {
    let (camera, camera_transform) = **camera_query;
    let world_pos = camera.viewport_to_world_2d(camera_transform, cursor).ok()?;
    world_to_cell(world_pos).map(|cell| (world_pos, cell))
}

pub(crate) fn is_cursor_over_ui(
    cursor: Vec2,
    window: &Window,
    debug_mode_active: bool,
    main_toolbar_layout: MainToolbarLayout,
    main_menu_open: bool,
    panel_manager: Option<&PanelManager>,
) -> bool {
    let toolbar_width = if main_toolbar_layout.width > 0.0 {
        main_toolbar_layout.width
    } else {
        MAIN_TOOLBAR_PADDING * 2.0
    };
    let toolbar_height = if main_toolbar_layout.height > 0.0 {
        main_toolbar_layout.height
    } else {
        MAIN_TOOLBAR_HEIGHT
    };
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
            toolbar_width,
            toolbar_height,
        ),
    ];

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

fn selected_pipe_structure_kind(
    pipe_tool: PipeToolKind,
) -> crate::world::structures::StructureKind {
    match pipe_tool {
        PipeToolKind::Pipe => crate::plugins::default_plugin::pipe_structure_kind(),
        PipeToolKind::Vent => crate::plugins::default_plugin::vent_structure_kind(),
        PipeToolKind::Bridge => crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
    }
}

#[derive(Clone, Copy)]
struct UiRectPx {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

