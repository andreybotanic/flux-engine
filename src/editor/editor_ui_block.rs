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
            hovered_text = Some(meta.label);
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

    tooltip_text.0 = label.to_string();
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

fn handle_editor_ui_actions(
    mut interactions: Query<(&Interaction, &EditorUiAction), (Changed<Interaction>, With<Button>)>,
    mut active_tool: ResMut<ActiveEditorTool>,
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
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
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

        match *action {
            EditorUiAction::SelectTool(next_tool) => {
                active_tool.selected = Some(next_tool);
                structure_edit.selected_cell = None;
                select_fields.close_all();
                unfocus_inputs();
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
            EditorUiAction::SelectCellMaterial(next_material) => {
                unfocus_inputs();
                cell_settings.material = next_material;
            }
            EditorUiAction::SelectPipeTool(next_pipe_tool) => {
                unfocus_inputs();
                pipe_settings.selected = next_pipe_tool;
                active_tool.selected = Some(EditorTool::Gases);
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
        Res<CellToolSettings>,
        Res<PipeToolSettings>,
        Res<MainMenuState>,
        Res<DebugMode>,
        Res<WorldLoadState>,
    ),
    sim_metrics: (
        Res<DebugGasMetrics>,
        Res<SimulationControl>,
        Res<SimulationPerfStats>,
    ),
    sim_step: Res<SimulationStep>,
    mut gas_simulation: ResMut<GasSimulationConfig>,
    mut sim_rate: ResMut<SimulationRateConfig>,
    debug_overlay: Res<DebugOverlaySettings>,
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
    let (active_tool, cell_settings, pipe_settings, main_menu, debug_mode, world_load_state) =
        ui_state;
    let (debug_metrics, sim_control, sim_perf) = sim_metrics;
    let selected_tool = active_tool.selected;

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
        bg.0 = match action {
            EditorUiAction::SelectTool(action_tool) if Some(*action_tool) == selected_tool => {
                BUTTON_ACTIVE
            }
            EditorUiAction::SelectCellMaterial(material) if *material == cell_settings.material => {
                BUTTON_ACTIVE
            }
            EditorUiAction::SelectPipeTool(kind)
                if selected_tool == Some(EditorTool::Gases) && *kind == pipe_settings.selected =>
            {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleReplace if gas_settings.replace => BUTTON_ACTIVE,
            EditorUiAction::ToggleBuoyancy if gas_simulation.solver_tuning.enable_buoyancy => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleShowMomentumVectors if debug_overlay.show_momentum_vectors => {
                BUTTON_ACTIVE
            }
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
        let mut cell_type_panel_root = ui.visibility_set.p1();
        **cell_type_panel_root =
            if world_load_state.has_world && selected_tool == Some(EditorTool::BuildSolid) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
    }

    {
        let mut gases_type_panel_root = ui.visibility_set.p2();
        **gases_type_panel_root =
            if world_load_state.has_world && selected_tool == Some(EditorTool::Gases) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
    }

    {
        let mut debug_toolbar_root = ui.visibility_set.p3();
        **debug_toolbar_root = if world_load_state.has_world && debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
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
    let editing_structure = structure_edit
        .selected_cell
        .and_then(|cell| structures.editable_structure_at(cell.x, cell.y).map(|s| (cell, s.params)));
    let structure_panel_visible =
        world_load_state.has_world && debug_mode.active && editing_structure.is_some();
    ui.panel_manager.set_visible(
        STRUCTURE_TOOL_PANEL_ID,
        structure_panel_visible,
        &mut ui.panel_open_order,
    );

    {
        let mut main_menu_root = ui.visibility_set.p4();
        **main_menu_root = if main_menu.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut replace_text = ui.text_set_primary.p0();
        replace_text.0 = if gas_settings.replace {
            "Replace: On".to_string()
        } else {
            "Replace: Off".to_string()
        };
    }

    {
        let mut buoyancy_toggle_text = ui.text_set_primary.p1();
        buoyancy_toggle_text.0 = if gas_simulation.solver_tuning.enable_buoyancy {
            "Buoyancy: On".to_string()
        } else {
            "Buoyancy: Off".to_string()
        };
    }

    {
        let mut perf_text = ui.text_set_primary.p2();
        let speed_mult = sim_control.speed.multiplier();
        perf_text.0 = format!(
            "Iterations: {} | Step ms: {:.3} | avg: {:.3} | Target Hz: {} x {} = {:.1} | Actual Hz: {:.1} | GPU compute/upload/readback/total: {:.3}/{:.3}/{:.3}/{:.3} ms",
            sim_step.0,
            sim_perf.last_step_ms,
            sim_perf.avg_step_ms,
            sim_rate.target_hz,
            speed_mult,
            sim_perf.target_hz_effective,
            sim_perf.actual_hz,
            sim_perf.last_gpu_compute_ms,
            sim_perf.last_upload_to_gpu_ms,
            sim_perf.last_readback_from_gpu_ms,
            sim_perf.last_step_total_ms
        );
    }

    {
        let mut metrics_text = ui.text_set_primary.p3();
        let vectors_mode = if debug_overlay.show_momentum_vectors {
            "Impulse vectors: On"
        } else {
            "Impulse vectors: Off"
        };
        metrics_text.0 = format!(
            "{} | Anisotropy: {:.4} | Radial waves: {:.4} | Mass err H2/O2/CO2: {:.4} / {:.4} / {:.4}",
            vectors_mode,
            debug_metrics.anisotropy_score,
            debug_metrics.radial_wave_score,
            debug_metrics.mass_error_h2,
            debug_metrics.mass_error_o2,
            debug_metrics.mass_error_co2
        );
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
    {
        let mut mode_text = ui.text_set_primary.p4();
        mode_text.0 = structure_mode;
    }

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
    button_query: Query<
        'w,
        's,
        (&'static EditorUiAction, &'static mut BackgroundColor),
        With<Button>,
    >,
    panel_manager: ResMut<'w, PanelManager>,
    panel_open_order: ResMut<'w, PanelOpenOrder>,
    visibility_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Visibility, With<MainToolbarRoot>>,
            Single<'w, &'static mut Visibility, With<CellTypePanelRoot>>,
            Single<'w, &'static mut Visibility, With<GasesTypePanelRoot>>,
            Single<'w, &'static mut Visibility, With<DebugToolbarRoot>>,
            Single<'w, &'static mut Visibility, With<MainMenuRoot>>,
        ),
    >,
    text_set_primary: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Text, With<GasReplaceLabel>>,
            Single<'w, &'static mut Text, With<BuoyancyToggleLabel>>,
            Single<'w, &'static mut Text, With<SimulationPerfLabel>>,
            Single<'w, &'static mut Text, With<WaveMetricsLabel>>,
            Single<'w, &'static mut Text, With<StructureModeLabel>>,
        ),
    >,
    node_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Node, With<StructureSourceSection>>,
            Single<'w, &'static mut Node, With<StructureSinkSection>>,
        ),
    >,
}

