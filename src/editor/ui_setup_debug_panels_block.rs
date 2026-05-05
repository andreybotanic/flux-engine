#[allow(clippy::too_many_arguments)]
fn spawn_debug_panel_content(
    parent: &mut ChildSpawnerCommands,
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
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::ToggleBuoyancy,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("Buoyancy: On"),
                TextFont::from_font_size(13.0),
                TextColor(DEBUG_PANEL_TEXT_COLOR),
                BuoyancyToggleLabel,
            ));
        });

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(220.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::ToggleShowMomentumVectors,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("Show impulses"),
                TextFont::from_font_size(13.0),
                TextColor(DEBUG_PANEL_TEXT_COLOR),
            ));
        });

    parent.spawn((
        Text::new(
            "Iterations: 0 | Step ms: 0.000 | avg: 0.000 | Target Hz: 30 x 1 = 30 | Actual Hz: 0",
        ),
        TextFont::from_font_size(13.0),
        TextColor(DEBUG_PANEL_TEXT_COLOR),
        SimulationPerfLabel,
    ));

    parent.spawn((
        Text::new(
            "Anisotropy: 0.0000 | Radial waves: 0.0000 | Mass err H2/O2/CO2: 0.0000 / 0.0000 / 0.0000",
        ),
        TextFont::from_font_size(13.0),
        TextColor(DEBUG_PANEL_TEXT_COLOR),
        WaveMetricsLabel,
    ));

    parent
        .spawn((Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new("Simulation Hz:"),
                TextFont::from_font_size(13.0),
                TextColor(DEBUG_PANEL_TEXT_COLOR),
            ));

            row.spawn((
                Button,
                Node {
                    min_width: Val::Px(92.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                    ..default()
                },
                BackgroundColor(BUTTON_IDLE),
                TextInputField::new_u32(sim_hz_initial, 1, 1000, 4),
                TextInputStyle {
                    idle_bg: BUTTON_IDLE,
                    focused_bg: INPUT_FOCUSED,
                },
                bevy::ui::RelativeCursorPosition::default(),
                SimulationHzInputField,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(sim_hz_initial_text.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(DEBUG_PANEL_TEXT_COLOR),
                    TextInputDisplay,
                ));
            });
        });

    spawn_debug_f32_row(
        parent,
        "Buoyancy strength:",
        buoyancy_strength_initial,
        buoyancy_strength_initial_text,
        TextInputField::new_f32(buoyancy_strength_initial, 0.0, 5.0, 6, 3),
        BuoyancyStrengthInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_u32_row(
        parent,
        "Buoyancy radius:",
        buoyancy_radius_initial,
        buoyancy_radius_initial_text,
        TextInputField::new_u32(buoyancy_radius_initial, 1, 3, 1),
        BuoyancyWindowRadiusInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy sigma:",
        buoyancy_sigma_initial,
        buoyancy_sigma_initial_text,
        TextInputField::new_f32(buoyancy_sigma_initial, 0.5, 3.0, 6, 3),
        BuoyancyWindowSigmaInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy gain:",
        buoyancy_gain_initial,
        buoyancy_gain_initial_text,
        TextInputField::new_f32(buoyancy_gain_initial, 0.0, 10.0, 6, 3),
        BuoyancyGainInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy alpha:",
        buoyancy_alpha_initial,
        buoyancy_alpha_initial_text,
        TextInputField::new_f32(buoyancy_alpha_initial, 0.0, 4.0, 6, 3),
        BuoyancyAlphaInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy cap:",
        buoyancy_cap_initial,
        buoyancy_cap_initial_text,
        TextInputField::new_f32(buoyancy_cap_initial, 0.0, 2.0, 6, 3),
        BuoyancyForceCapInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Gamma:",
        gamma_initial,
        gamma_initial_text,
        TextInputField::new_f32(gamma_initial, 0.0, 10.0, 6, 3),
        GasGammaInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_u32_row(
        parent,
        "Max color at:",
        max_color_initial,
        max_color_initial_text,
        TextInputField::new_u32(max_color_initial, 1, 10_000, 5),
        GasMaxColorParticlesInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
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
    parent
        .spawn((Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont::from_font_size(13.0),
                TextColor(label_color),
            ));
            row.spawn((
                Button,
                Node {
                    min_width: Val::Px(92.0),
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
    parent
        .spawn((Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont::from_font_size(13.0),
                TextColor(label_color),
            ));
            row.spawn((
                Button,
                Node {
                    min_width: Val::Px(92.0),
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

