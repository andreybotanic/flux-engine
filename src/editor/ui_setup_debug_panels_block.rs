const DEBUG_FIELD_MIN_WIDTH: f32 = 92.0;
const DEBUG_SWITCH_WIDTH: f32 = 154.0;

#[allow(clippy::too_many_arguments)]
fn spawn_debug_panel_content(
    parent: &mut ChildSpawnerCommands,
    select_arrow: Handle<Image>,
    sim_hz_initial: u32,
    sim_hz_initial_text: String,
    buoyancy_strength_initial: f32,
    buoyancy_strength_initial_text: String,
    buoyancy_radius_initial: u32,
    buoyancy_radius_initial_text: String,
    buoyancy_sigma_initial: f32,
    buoyancy_sigma_initial_text: String,
    buoyancy_gain_initial: f32,
    buoyancy_gain_initial_text: String,
    buoyancy_alpha_initial: f32,
    buoyancy_alpha_initial_text: String,
    buoyancy_cap_initial: f32,
    buoyancy_cap_initial_text: String,
    gamma_initial: f32,
    gamma_initial_text: String,
    max_color_initial: u32,
    max_color_initial_text: String,
) {
    spawn_debug_value_row(
        parent,
        "Iterations:",
        "0",
        SimulationIterationsLabel,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_u32_row(
        parent,
        "Simulation Hz:",
        sim_hz_initial,
        sim_hz_initial_text,
        TextInputField::new_u32(sim_hz_initial, 1, 1000, 4),
        SimulationHzInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );

    let mut time_block = crate::ui::collapsible_block::CollapsibleBlockConfig::new("Time", false);
    time_block.arrow_icon = Some(select_arrow.clone());
    crate::ui::collapsible_block::spawn_collapsible_block(parent, time_block, |block| {
            spawn_debug_value_row(
                block,
                "Step ms:",
                "0.000",
                SimulationStepMsLabel,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row(
                block,
                "Step avg ms:",
                "0.000",
                SimulationStepAvgMsLabel,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row(
                block,
                "Pipe ms:",
                "0.000",
                SimulationPipeMsLabel,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row(
                block,
                "Pipe avg ms:",
                "0.000",
                SimulationPipeAvgMsLabel,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row(
                block,
                "Actual Hz:",
                "0.0",
                SimulationActualHzLabel,
                DEBUG_PANEL_TEXT_COLOR,
            );

            spawn_debug_value_row_with_row_marker(
                block,
                "GPU compute ms:",
                "0.000",
                SimulationGpuComputeMsLabel,
                DebugGpuTimeRow,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row_with_row_marker(
                block,
                "GPU upload ms:",
                "0.000",
                SimulationGpuUploadMsLabel,
                DebugGpuTimeRow,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row_with_row_marker(
                block,
                "GPU readback ms:",
                "0.000",
                SimulationGpuReadbackMsLabel,
                DebugGpuTimeRow,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row_with_row_marker(
                block,
                "GPU total ms:",
                "0.000",
                SimulationGpuTotalMsLabel,
                DebugGpuTimeRow,
                DEBUG_PANEL_TEXT_COLOR,
            );
        });

    let mut gas_sim_block =
        crate::ui::collapsible_block::CollapsibleBlockConfig::new("Gas simulation", true);
    gas_sim_block.arrow_icon = Some(select_arrow.clone());
    crate::ui::collapsible_block::spawn_collapsible_block(parent, gas_sim_block, |block| {
            spawn_debug_switch_row(
                block,
                "Buoyancy:",
                true,
                EditorUiAction::ToggleBuoyancy,
                DebugBuoyancySwitch,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_switch_row(
                block,
                "Show impulses:",
                false,
                EditorUiAction::ToggleShowMomentumVectors,
                DebugImpulseSwitch,
                DEBUG_PANEL_TEXT_COLOR,
            );

            spawn_debug_f32_row(
                block,
                "Buoyancy strength:",
                buoyancy_strength_initial,
                buoyancy_strength_initial_text,
                TextInputField::new_f32(buoyancy_strength_initial, 0.0, 5.0, 6, 3),
                BuoyancyStrengthInputField,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_u32_row(
                block,
                "Buoyancy radius:",
                buoyancy_radius_initial,
                buoyancy_radius_initial_text,
                TextInputField::new_u32(buoyancy_radius_initial, 1, 3, 1),
                BuoyancyWindowRadiusInputField,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_f32_row(
                block,
                "Buoyancy sigma:",
                buoyancy_sigma_initial,
                buoyancy_sigma_initial_text,
                TextInputField::new_f32(buoyancy_sigma_initial, 0.5, 3.0, 6, 3),
                BuoyancyWindowSigmaInputField,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_f32_row(
                block,
                "Buoyancy gain:",
                buoyancy_gain_initial,
                buoyancy_gain_initial_text,
                TextInputField::new_f32(buoyancy_gain_initial, 0.0, 10.0, 6, 3),
                BuoyancyGainInputField,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_f32_row(
                block,
                "Buoyancy alpha:",
                buoyancy_alpha_initial,
                buoyancy_alpha_initial_text,
                TextInputField::new_f32(buoyancy_alpha_initial, 0.0, 4.0, 6, 3),
                BuoyancyAlphaInputField,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_f32_row(
                block,
                "Buoyancy cap:",
                buoyancy_cap_initial,
                buoyancy_cap_initial_text,
                TextInputField::new_f32(buoyancy_cap_initial, 0.0, 2.0, 6, 3),
                BuoyancyForceCapInputField,
                DEBUG_PANEL_TEXT_COLOR,
            );
            spawn_debug_value_row(
                block,
                "Mass error:",
                "0.0000",
                SimulationMassErrorLabel,
                DEBUG_PANEL_TEXT_COLOR,
            );
        });

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            DebugGasOverlayBlockRoot,
        ))
        .with_children(|overlay_root| {
            let mut overlay_block =
                crate::ui::collapsible_block::CollapsibleBlockConfig::new("Gas overlay", true);
            overlay_block.arrow_icon = Some(select_arrow.clone());
            crate::ui::collapsible_block::spawn_collapsible_block(overlay_root, overlay_block, |block| {
                    spawn_debug_f32_row(
                        block,
                        "Gamma:",
                        gamma_initial,
                        gamma_initial_text,
                        TextInputField::new_f32(gamma_initial, 0.0, 10.0, 6, 3),
                        GasGammaInputField,
                        DEBUG_PANEL_TEXT_COLOR,
                    );
                    spawn_debug_u32_row(
                        block,
                        "Max color at:",
                        max_color_initial,
                        max_color_initial_text,
                        TextInputField::new_u32(max_color_initial, 1, 10_000, 5),
                        GasMaxColorParticlesInputField,
                        DEBUG_PANEL_TEXT_COLOR,
                    );
                });
        });
}

fn debug_row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        display: Display::Flex,
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_debug_value_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    initial_value: &str,
    marker: M,
    label_color: Color,
) {
    spawn_debug_value_row_with_row_marker(
        parent,
        label,
        initial_value,
        marker,
        (),
        label_color,
    );
}

fn spawn_debug_value_row_with_row_marker<M: Component, R: Bundle>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    initial_value: &str,
    marker: M,
    row_marker: R,
    label_color: Color,
) {
    parent.spawn((debug_row_node(), row_marker)).with_children(|row| {
        row.spawn((
            Text::new(label),
            TextFont::from_font_size(13.0),
            TextColor(label_color),
        ));
        row.spawn((
            Text::new(initial_value.to_string()),
            TextFont::from_font_size(13.0),
            TextColor(label_color),
            marker,
        ));
    });
}

fn spawn_debug_switch_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    initial_on: bool,
    action: EditorUiAction,
    marker: M,
    label_color: Color,
) {
    parent.spawn(debug_row_node()).with_children(|row| {
        row.spawn((
            Text::new(label),
            TextFont::from_font_size(13.0),
            TextColor(label_color),
        ));
        row.spawn(Node {
            width: Val::Px(DEBUG_SWITCH_WIDTH),
            display: Display::Flex,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        })
        .with_children(|switch_holder| {
            crate::ui::toggle_switch::spawn_toggle_switch_button(
                switch_holder,
                crate::ui::toggle_switch::ToggleSwitchConfig::new(initial_on, true, ""),
                (action, marker),
            );
        });
    });
}

fn spawn_debug_f32_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    _initial: f32,
    initial_text: String,
    field: TextInputField,
    marker: M,
    label_color: Color,
) {
    parent.spawn(debug_row_node()).with_children(|row| {
        row.spawn((
            Text::new(label),
            TextFont::from_font_size(13.0),
            TextColor(label_color),
        ));
        row.spawn((
            Button,
            Node {
                min_width: Val::Px(DEBUG_FIELD_MIN_WIDTH),
                height: Val::Px(30.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            field,
            TextInputStyle {
                idle_bg: BUTTON_IDLE,
                focused_bg: INPUT_FOCUSED,
            },
            bevy::ui::RelativeCursorPosition::default(),
            marker,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(initial_text.clone()),
                TextFont::from_font_size(13.0),
                TextColor(label_color),
                TextInputDisplay,
            ));
        });
    });
}

fn spawn_debug_u32_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    _initial: u32,
    initial_text: String,
    field: TextInputField,
    marker: M,
    label_color: Color,
) {
    parent.spawn(debug_row_node()).with_children(|row| {
        row.spawn((
            Text::new(label),
            TextFont::from_font_size(13.0),
            TextColor(label_color),
        ));
        row.spawn((
            Button,
            Node {
                min_width: Val::Px(DEBUG_FIELD_MIN_WIDTH),
                height: Val::Px(30.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            field,
            TextInputStyle {
                idle_bg: BUTTON_IDLE,
                focused_bg: INPUT_FOCUSED,
            },
            bevy::ui::RelativeCursorPosition::default(),
            marker,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(initial_text.clone()),
                TextFont::from_font_size(13.0),
                TextColor(label_color),
                TextInputDisplay,
            ));
        });
    });
}
