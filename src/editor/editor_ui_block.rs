fn update_tool_button_tooltip(
    window: Single<&Window, With<PrimaryWindow>>,
    main_menu: Res<MainMenuState>,
    mut tooltip_node: Single<&mut Node, With<UiTooltipRoot>>,
    mut tooltip_text: Single<&mut Text, With<UiTooltipText>>,
    button_query: Query<(&Interaction, &ToolButtonMeta), With<Button>>,
) {
    let node = &mut *tooltip_node;
    if main_menu.open {
        node.display = Display::None;
        return;
    }

    let mut hovered_text = None;
    for (interaction, meta) in &button_query {
        if *interaction == Interaction::Hovered {
            hovered_text = Some(meta.label.clone());
            break;
        }
    }

    let Some(label) = hovered_text else {
        node.display = Display::None;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        node.display = Display::None;
        return;
    };

    tooltip_text.0 = label;
    node.display = Display::Flex;
    node.left = Val::Px((cursor.x + 14.0).min(window.width() - 130.0));
    node.top = Val::Px((cursor.y + 16.0).min(window.height() - 34.0));
}

fn update_selection_size_tooltip(
    selection_drag: Res<SelectionDragState>,
    active_tool: Res<ActiveEditorTool>,
    main_menu: Res<MainMenuState>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut tooltip_node: Single<&mut Node, With<SelectionSizeTooltip>>,
    mut tooltip_text: Single<&mut Text, With<SelectionSizeTooltipText>>,
) {
    let node = &mut *tooltip_node;
    if main_menu.open {
        node.display = Display::None;
        return;
    }

    let Some(tool) = active_tool.selected else {
        node.display = Display::None;
        return;
    };
    if !matches!(tool, EditorTool::AddGas | EditorTool::ClearGas) || !selection_drag.active {
        node.display = Display::None;
        return;
    }

    let (Some(start), Some(end)) = (selection_drag.start, selection_drag.current) else {
        node.display = Display::None;
        return;
    };
    let (min, max) = normalized_rect(start, end);
    let w = max.x - min.x + 1;
    let h = max.y - min.y + 1;
    let area = w * h;

    let min_center = cell_center(min.x, min.y);
    let max_center = cell_center(max.x, max.y);
    let center_world = ((min_center + max_center) * 0.5).extend(0.0);

    let (camera, camera_transform) = *camera_query;
    let Ok(center_screen) = camera.world_to_viewport(camera_transform, center_world) else {
        node.display = Display::None;
        return;
    };

    tooltip_text.0 = format!("{w}X{h}\n{area}");
    node.display = Display::Flex;
    node.left = Val::Px(center_screen.x - 42.0);
    node.top = Val::Px(center_screen.y - 24.0);
}

fn handle_tool_variant_panel_actions(
    mut action_events: EventReader<PanelHeaderActionEvent>,
    mut tool_variant_panels: ResMut<ToolVariantPanelState>,
    category_ui_registry: Res<EntityCategoryUiRegistry>,
    mut panel_manager: ResMut<PanelManager>,
    mut panel_open_order: ResMut<PanelOpenOrder>,
) {
    for event in action_events.read() {
        if event.action_id != TOOL_VARIANT_PANEL_CLOSE_ACTION_ID {
            continue;
        }

        if let Some(category) = category_ui_registry.by_panel_id(event.panel_id) {
            tool_variant_panels.mark_closed(&category.id);
            panel_manager.set_visible(
                category.panel_id,
                false,
                &mut panel_open_order,
            );
        }
    }
}

fn handle_editor_ui_actions(
    mut interactions: Query<(&Interaction, &EditorUiAction), (Changed<Interaction>, With<Button>)>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut active_entity_category: ResMut<ActiveEntityCategory>,
    category_ui_registry: Res<EntityCategoryUiRegistry>,
    mut cell_settings: ResMut<CellToolSettings>,
    mut pipe_settings: ResMut<PipeToolSettings>,
    mut gas_settings: ResMut<GasToolSettings>,
    mut gas_simulation: ResMut<GasSimulationConfig>,
    mut debug_overlay: ResMut<DebugOverlaySettings>,
    mut structure_edit: ResMut<StructureEditState>,
    mut select_fields: ResMut<SelectFieldState>,
    mut input_set: ParamSet<(
        Single<&mut TextInputField, With<GasAmountInputField>>,
        Single<&mut TextInputField, With<SourceAmountInputField>>,
        Single<&mut TextInputField, With<SinkAmountInputField>>,
        Single<&mut TextInputField, With<GasGammaInputField>>,
        Single<&mut TextInputField, With<GasMaxColorParticlesInputField>>,
    )>,
    mut tool_variant_panels: ResMut<ToolVariantPanelState>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
    mut plugin_events: EventWriter<PluginRuntimeEvent>,
) {
    let mut unfocus_inputs = || {
        let mut gas_input = input_set.p0();
        gas_input.focused = false;
        let mut source_input = input_set.p1();
        source_input.focused = false;
        let mut sink_input = input_set.p2();
        sink_input.focused = false;
        let mut gamma_input = input_set.p3();
        gamma_input.focused = false;
        let mut max_color_particles_input = input_set.p4();
        max_color_particles_input.focused = false;
    };

    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            EditorUiAction::SelectTool(next_tool) => {
                active_tool.selected = Some(*next_tool);
                plugin_events.write(PluginRuntimeEvent::ToolSelected {
                    tool_id: active_tool_content_id(
                        Some(*next_tool),
                        active_entity_category.selected.as_ref(),
                    ),
                });
                structure_edit.selected_cell = None;
                select_fields.close_all();
                unfocus_inputs();
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
            EditorUiAction::SelectEntityCategory(category_id) => {
                unfocus_inputs();
                active_entity_category.set(Some(category_id.clone()));
                active_tool.selected = Some(EditorTool::Construct);
                tool_variant_panels.mark_open(category_id);
                plugin_events.write(PluginRuntimeEvent::ToolSelected {
                    tool_id: active_tool_content_id(
                        Some(EditorTool::Construct),
                        active_entity_category.selected.as_ref(),
                    ),
                });
                structure_edit.selected_cell = None;
                select_fields.close_all();
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
            EditorUiAction::SelectCellMaterial(next_material) => {
                unfocus_inputs();
                cell_settings.material = Some(*next_material);
            }
            EditorUiAction::SelectPipeTool(next_pipe_tool) => {
                unfocus_inputs();
                pipe_settings.selected = *next_pipe_tool;
                active_tool.selected = Some(EditorTool::Construct);
                if let Some(gases_category) = category_ui_registry
                    .items
                    .iter()
                    .find(|item| item.kind == EntityCategoryKind::Gases)
                {
                    active_entity_category.set(Some(gases_category.id.clone()));
                    tool_variant_panels.mark_open(&gases_category.id);
                }
                plugin_events.write(PluginRuntimeEvent::ToolSelected {
                    tool_id: active_tool_content_id(
                        Some(EditorTool::Construct),
                        active_entity_category.selected.as_ref(),
                    ),
                });
                structure_edit.selected_cell = None;
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
            EditorUiAction::ToggleReplace => {
                unfocus_inputs();
                gas_settings.replace = !gas_settings.replace;
            }
            EditorUiAction::ToggleBuoyancy => {
                unfocus_inputs();
                gas_simulation.solver_tuning.enable_buoyancy =
                    !gas_simulation.solver_tuning.enable_buoyancy;
            }
            EditorUiAction::ToggleShowMomentumVectors => {
                unfocus_inputs();
                debug_overlay.show_momentum_vectors = !debug_overlay.show_momentum_vectors;
            }
        }
    }
}

fn refresh_editor_ui(
    ui_state: (
        Res<ActiveEditorTool>,
        Res<ActiveEntityCategory>,
        Res<EntityCategoryUiRegistry>,
        Res<CellToolSettings>,
        Res<PipeToolSettings>,
        Res<ToolVariantPanelState>,
        Res<MainMenuState>,
        Res<DebugMode>,
        Res<WorldLoadState>,
        ResMut<UiScrollBlockState>,
    ),
    sim_metrics: (
        Res<DebugGasMetrics>,
        Res<SimulationControl>,
        Res<SimulationPerfStats>,
    ),
    sim_step: Res<SimulationStep>,
    ui_context: (
        Res<DebugOverlaySettings>,
        Res<crate::simulation::backend::SimulationBackendConfig>,
        Res<OverlayMode>,
    ),
    mut gas_simulation: ResMut<GasSimulationConfig>,
    mut sim_rate: ResMut<SimulationRateConfig>,
    mut gas_visual_settings: ResMut<GasVisualSettings>,
    mut gas_settings: ResMut<GasToolSettings>,
    mut source_settings: ResMut<SourceStructureToolSettings>,
    mut sink_settings: ResMut<SinkStructureToolSettings>,
    mut structure_edit: ResMut<StructureEditState>,
    mut structures: ResMut<PlacedStructureMap>,
    select_fields: Res<SelectFieldState>,
    world: Res<WorldGrid>,
    gas_registry: Res<GasRegistry>,
    mut ui: RefreshEditorUiSystemParams,
) {
    let (
        active_tool,
        active_entity_category,
        category_ui_registry,
        cell_settings,
        pipe_settings,
        tool_variant_panels,
        main_menu,
        debug_mode,
        world_load_state,
        mut ui_scroll_block,
    ) = ui_state;
    let (debug_metrics, _sim_control, sim_perf) = sim_metrics;
    let selected_tool = active_tool.selected;
    let selected_category_id = active_entity_category.selected.as_ref();
    let selected_category_kind = selected_category_id
        .and_then(|category_id| category_ui_registry.by_id(category_id))
        .map(|descriptor| descriptor.kind);
    let (debug_overlay, sim_backend, overlay_mode) = ui_context;
    ui_scroll_block.block_panel_scrolling = main_menu.open;

    if let Some(selected) = select_fields.selected_index(GAS_SELECT_ADD_ID) {
        gas_settings.gas_index = selected;
    }
    if let Some(selected) = select_fields.selected_index(GAS_SELECT_SOURCE_ID) {
        source_settings.gas_index = selected;
    }

    if let Some(amount) = ui.gas_input.parsed_u32() {
        gas_settings.amount = amount;
    }
    if let Some(amount) = ui.source_input.parsed_u32() {
        source_settings.amount = amount.max(1);
    }
    if let Some(amount) = ui.sink_input.parsed_u32() {
        sink_settings.amount = amount.max(1);
    }
    if gas_registry.count() > 0 && gas_settings.gas_index >= gas_registry.count() {
        gas_settings.gas_index = gas_registry.count() - 1;
    }
    if gas_registry.count() > 0 && source_settings.gas_index >= gas_registry.count() {
        source_settings.gas_index = gas_registry.count() - 1;
    }
    if let Some(gamma) = ui.input_set.p0().parsed_f32() {
        let next_gamma = gamma.clamp(0.0, 10.0);
        if (gas_visual_settings.gamma - next_gamma).abs() > f32::EPSILON {
            gas_visual_settings.gamma = next_gamma;
        }
    }
    if let Some(max_particles) = ui.input_set.p1().parsed_u32() {
        let next_max_particles = max_particles.clamp(1, 10_000);
        if gas_visual_settings.max_particles_for_max_color != next_max_particles {
            gas_visual_settings.max_particles_for_max_color = next_max_particles;
        }
    }
    if let Some(target_hz) = ui.input_set.p2().parsed_u32() {
        sim_rate.target_hz = target_hz.clamp(1, 1000);
    }
    if let Some(value) = ui.input_set.p3().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_strength = value.clamp(0.0, 5.0);
    }
    if let Some(value) = ui.input_set.p4().parsed_u32() {
        gas_simulation.solver_tuning.buoyancy_window_radius = value.clamp(1, 3) as u8;
    }
    if let Some(value) = ui.input_set.p5().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_window_sigma = value.clamp(0.5, 3.0);
    }
    if let Some(value) = ui.input_set.p6().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_gain = value.clamp(0.0, 10.0);
    }
    if let Some(value) = ui.input_set.p7().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_alpha = value.clamp(0.0, 4.0);
    }
    if let Some(value) = ui.buoyancy_cap_input.parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_force_cap = value.clamp(0.0, 2.0);
    }

    if let Some(cell) = structure_edit.selected_cell {
        let selected = structures
            .editable_structure_at(cell.x, cell.y)
            .map(|structure| (structure.id, structure.params));
        match selected {
            Some((id, StructureParams::GasSource { gas_index, amount })) => {
                let desired_gas_index = if gas_registry.count() == 0 {
                    0
                } else {
                    source_settings.gas_index.min(gas_registry.count() - 1)
                };
                let desired_amount = source_settings.amount.max(1);
                let _ = world;
                if desired_gas_index != gas_index || desired_amount != amount {
                    let _ = structures.update_gas_source(id, desired_gas_index, desired_amount);
                }
            }
            Some((id, StructureParams::GasSink { amount })) => {
                let desired_amount = sink_settings.amount.max(1);
                if desired_amount != amount {
                    let _ = structures.update_gas_sink(id, desired_amount);
                }
            }
            _ => {
                structure_edit.selected_cell = None;
            }
        }
    }

    for (action, mut bg) in &mut ui.button_query {
        if matches!(
            action,
            EditorUiAction::ToggleBuoyancy | EditorUiAction::ToggleShowMomentumVectors
        ) {
            continue;
        }
        bg.0 = match action {
            EditorUiAction::SelectTool(action_tool) if Some(*action_tool) == selected_tool => {
                BUTTON_ACTIVE
            }
            EditorUiAction::SelectEntityCategory(category_id)
                if selected_tool == Some(EditorTool::Construct)
                    && selected_category_id == Some(category_id) =>
            {
                BUTTON_ACTIVE
            }
            EditorUiAction::SelectCellMaterial(material)
                if cell_settings.material == Some(*material) =>
            {
                if selected_tool == Some(EditorTool::Construct)
                    && selected_category_kind == Some(EntityCategoryKind::Cells)
                {
                    BUTTON_ACTIVE
                } else {
                    BUTTON_IDLE
                }
            }
            EditorUiAction::SelectPipeTool(kind)
                if selected_tool == Some(EditorTool::Construct)
                    && selected_category_kind == Some(EntityCategoryKind::Gases)
                    && *kind == pipe_settings.selected =>
            {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleReplace if gas_settings.replace => BUTTON_ACTIVE,
            _ => BUTTON_IDLE,
        };
    }

    {
        let mut main_toolbar_root = ui.visibility_set.p0();
        **main_toolbar_root = if world_load_state.has_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut debug_toolbar_root = ui.visibility_set.p1();
        **debug_toolbar_root = if world_load_state.has_world && debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for category in &category_ui_registry.items {
        let is_selected = selected_category_id
            .map(|selected| selected == &category.id)
            .unwrap_or(false);
        let visible = category_variant_panel_visible(
            selected_tool,
            selected_category_id,
            &category.id,
            world_load_state.has_world,
            tool_variant_panels.is_closed(&category.id),
        ) && is_selected;
        ui.panel_manager
            .set_visible(category.panel_id, visible, &mut ui.panel_open_order);
    }

    let debug_panel_visible = world_load_state.has_world && debug_mode.active;
    ui.panel_manager.set_visible(
        DEBUG_PANEL_ID,
        debug_panel_visible,
        &mut ui.panel_open_order,
    );

    let gas_tool_panel_visible = world_load_state.has_world
        && debug_mode.active
        && selected_tool == Some(EditorTool::AddGas);
    ui.panel_manager.set_visible(
        GAS_TOOL_PANEL_ID,
        gas_tool_panel_visible,
        &mut ui.panel_open_order,
    );
    let editing_structure = structure_edit.selected_cell.and_then(|cell| {
        structures
            .editable_structure_at(cell.x, cell.y)
            .map(|s| (cell, s.params))
    });
    let structure_panel_visible =
        world_load_state.has_world && debug_mode.active && editing_structure.is_some();
    ui.panel_manager.set_visible(
        STRUCTURE_TOOL_PANEL_ID,
        structure_panel_visible,
        &mut ui.panel_open_order,
    );

    {
        let mut main_menu_root = ui.visibility_set.p2();
        **main_menu_root = if main_menu.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    let replace_text_value = if gas_settings.replace {
        "Replace: On".to_string()
    } else {
        "Replace: Off".to_string()
    };
    let iterations_text_value = format_iterations_text(sim_step.0);
    let step_ms_text_value = format!("{:.3}", sim_perf.last_step_ms);
    let step_avg_ms_text_value = format!("{:.3}", sim_perf.avg_step_ms);
    let pipe_ms_text_value = format!("{:.3}", sim_perf.last_pipe_step_ms);
    let pipe_avg_ms_text_value = format!("{:.3}", sim_perf.avg_pipe_step_ms);
    let actual_hz_text_value = format!("{:.1}", sim_perf.actual_hz);
    let gpu_compute_ms_text_value = format!("{:.3}", sim_perf.last_gpu_compute_ms);
    let gpu_upload_ms_text_value = format!("{:.3}", sim_perf.last_upload_to_gpu_ms);
    let gpu_readback_ms_text_value = format!("{:.3}", sim_perf.last_readback_from_gpu_ms);
    let gpu_total_ms_text_value = format!("{:.3}", sim_perf.last_step_total_ms);
    let mass_error_text_value = format_mass_error_text(debug_metrics.mass_error);

    {
        let mut buoyancy_switch = ui.toggle_switch_set.p0();
        if buoyancy_switch.on != gas_simulation.solver_tuning.enable_buoyancy
            || !buoyancy_switch.interactive
            || !buoyancy_switch.label.is_empty()
        {
            buoyancy_switch.on = gas_simulation.solver_tuning.enable_buoyancy;
            buoyancy_switch.interactive = true;
            buoyancy_switch.label.clear();
        }
    }

    {
        let mut impulse_switch = ui.toggle_switch_set.p1();
        if impulse_switch.on != debug_overlay.show_momentum_vectors
            || !impulse_switch.interactive
            || !impulse_switch.label.is_empty()
        {
            impulse_switch.on = debug_overlay.show_momentum_vectors;
            impulse_switch.interactive = true;
            impulse_switch.label.clear();
        }
    }

    {
        let show_gpu_rows = gpu_time_rows_visible(sim_backend.backend);
        for mut row_node in &mut ui.node_set.p2() {
            row_node.display = if show_gpu_rows {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    {
        let mut overlay_root = ui.node_set.p3();
        overlay_root.display = if overlay_mode.is_gas() {
            Display::Flex
        } else {
            Display::None
        };
    }

    let structure_mode = match (selected_tool, editing_structure) {
        (_, Some((cell, StructureParams::GasSource { .. }))) => {
            let mut src = ui.node_set.p0();
            src.display = Display::Flex;
            let mut sink = ui.node_set.p1();
            sink.display = Display::None;
            format!("Editing Source at ({}, {})", cell.x, cell.y)
        }
        (_, Some((cell, StructureParams::GasSink { .. }))) => {
            let mut src = ui.node_set.p0();
            src.display = Display::None;
            let mut sink = ui.node_set.p1();
            sink.display = Display::Flex;
            format!("Editing Sink at ({}, {})", cell.x, cell.y)
        }
        _ => {
            let mut src = ui.node_set.p0();
            src.display = Display::None;
            let mut sink = ui.node_set.p1();
            sink.display = Display::None;
            "No structure selected".to_string()
        }
    };
    for (
        mut text,
        replace,
        iterations,
        step_ms,
        step_avg_ms,
        pipe_ms,
        pipe_avg_ms,
        actual_hz,
        gpu_compute_ms,
        gpu_upload_ms,
        gpu_readback_ms,
        gpu_total_ms,
        mass_error,
        structure_mode_label,
    ) in &mut ui.text_values
    {
        if replace.is_some() {
            text.0 = replace_text_value.clone();
        } else if iterations.is_some() {
            text.0 = iterations_text_value.clone();
        } else if step_ms.is_some() {
            text.0 = step_ms_text_value.clone();
        } else if step_avg_ms.is_some() {
            text.0 = step_avg_ms_text_value.clone();
        } else if pipe_ms.is_some() {
            text.0 = pipe_ms_text_value.clone();
        } else if pipe_avg_ms.is_some() {
            text.0 = pipe_avg_ms_text_value.clone();
        } else if actual_hz.is_some() {
            text.0 = actual_hz_text_value.clone();
        } else if gpu_compute_ms.is_some() {
            text.0 = gpu_compute_ms_text_value.clone();
        } else if gpu_upload_ms.is_some() {
            text.0 = gpu_upload_ms_text_value.clone();
        } else if gpu_readback_ms.is_some() {
            text.0 = gpu_readback_ms_text_value.clone();
        } else if gpu_total_ms.is_some() {
            text.0 = gpu_total_ms_text_value.clone();
        } else if mass_error.is_some() {
            text.0 = mass_error_text_value.clone();
        } else if structure_mode_label.is_some() {
            text.0 = structure_mode.clone();
        }
    }
}

fn format_iterations_text(iterations: u64) -> String {
    iterations.to_string()
}

fn format_mass_error_text(mass_error: f32) -> String {
    format!("{mass_error:.4}")
}

fn gpu_time_rows_visible(backend: crate::simulation::backend::SimulationBackend) -> bool {
    matches!(backend, crate::simulation::backend::SimulationBackend::Gpu)
}

fn category_variant_panel_visible(
    selected_tool: Option<EditorTool>,
    selected_category_id: Option<&ContentId>,
    panel_category_id: &ContentId,
    world_loaded: bool,
    panel_closed: bool,
) -> bool {
    world_loaded
        && !panel_closed
        && selected_tool == Some(EditorTool::Construct)
        && selected_category_id
            .map(|selected| selected == panel_category_id)
            .unwrap_or(false)
}

#[derive(SystemParam)]
struct RefreshEditorUiSystemParams<'w, 's> {
    gas_input: Single<'w, &'static TextInputField, With<GasAmountInputField>>,
    source_input: Single<'w, &'static TextInputField, With<SourceAmountInputField>>,
    sink_input: Single<'w, &'static TextInputField, With<SinkAmountInputField>>,
    input_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static TextInputField, With<GasGammaInputField>>,
            Single<'w, &'static TextInputField, With<GasMaxColorParticlesInputField>>,
            Single<'w, &'static TextInputField, With<SimulationHzInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyStrengthInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyWindowRadiusInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyWindowSigmaInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyGainInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyAlphaInputField>>,
        ),
    >,
    buoyancy_cap_input: Single<'w, &'static TextInputField, With<BuoyancyForceCapInputField>>,
    button_query:
        Query<'w, 's, (&'static EditorUiAction, &'static mut BackgroundColor), With<Button>>,
    panel_manager: ResMut<'w, PanelManager>,
    panel_open_order: ResMut<'w, PanelOpenOrder>,
    visibility_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Visibility, With<MainToolbarRoot>>,
            Single<'w, &'static mut Visibility, With<DebugToolbarRoot>>,
            Single<'w, &'static mut Visibility, With<MainMenuRoot>>,
        ),
    >,
    text_values: Query<
        'w,
        's,
        (
            &'static mut Text,
            Option<&'static GasReplaceLabel>,
            Option<&'static SimulationIterationsLabel>,
            Option<&'static SimulationStepMsLabel>,
            Option<&'static SimulationStepAvgMsLabel>,
            Option<&'static SimulationPipeMsLabel>,
            Option<&'static SimulationPipeAvgMsLabel>,
            Option<&'static SimulationActualHzLabel>,
            Option<&'static SimulationGpuComputeMsLabel>,
            Option<&'static SimulationGpuUploadMsLabel>,
            Option<&'static SimulationGpuReadbackMsLabel>,
            Option<&'static SimulationGpuTotalMsLabel>,
            Option<&'static SimulationMassErrorLabel>,
            Option<&'static StructureModeLabel>,
        ),
    >,
    toggle_switch_set: ParamSet<
        'w,
        's,
        (
            Single<
                'w,
                &'static mut crate::ui::toggle_switch::ToggleSwitchRoot,
                With<DebugBuoyancySwitch>,
            >,
            Single<
                'w,
                &'static mut crate::ui::toggle_switch::ToggleSwitchRoot,
                With<DebugImpulseSwitch>,
            >,
        ),
    >,
    node_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Node, With<StructureSourceSection>>,
            Single<'w, &'static mut Node, With<StructureSinkSection>>,
            Query<'w, 's, &'static mut Node, With<DebugGpuTimeRow>>,
            Single<'w, &'static mut Node, With<DebugGasOverlayBlockRoot>>,
        ),
    >,
}

#[cfg(test)]
mod editor_ui_block_tests {
    use super::{category_variant_panel_visible, format_mass_error_text, gpu_time_rows_visible};
    use crate::editor::EditorTool;
    use crate::plugins::ContentId;

    #[test]
    fn gpu_time_rows_are_visible_only_for_gpu_backend() {
        assert!(gpu_time_rows_visible(
            crate::simulation::backend::SimulationBackend::Gpu
        ));
        assert!(!gpu_time_rows_visible(
            crate::simulation::backend::SimulationBackend::Cpu
        ));
    }

    #[test]
    fn mass_error_text_only_contains_mass_error_metric() {
        let text = format_mass_error_text(0.1234);
        assert_eq!(text, "0.1234");
        assert!(!text.contains("Anisotropy"));
        assert!(!text.contains("Radial waves"));
    }

    #[test]
    fn variant_panel_visibility_respects_selected_tool_and_close_state() {
        let cells = ContentId::parse("flux.default.category.cells").expect("cells id");
        let gases = ContentId::parse("flux.default.category.gases").expect("gases id");
        assert!(category_variant_panel_visible(
            Some(EditorTool::Construct),
            Some(&cells),
            &cells,
            true,
            false
        ));
        assert!(!category_variant_panel_visible(
            Some(EditorTool::Construct),
            Some(&cells),
            &cells,
            true,
            true
        ));
        assert!(!category_variant_panel_visible(
            Some(EditorTool::Construct),
            Some(&cells),
            &cells,
            false,
            false
        ));
        assert!(!category_variant_panel_visible(
            Some(EditorTool::Construct),
            Some(&gases),
            &cells,
            true,
            false
        ));
        assert!(!category_variant_panel_visible(
            Some(EditorTool::AddGas),
            Some(&cells),
            &cells,
            true,
            false
        ));
    }
}
