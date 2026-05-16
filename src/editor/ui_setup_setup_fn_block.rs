fn setup_editor_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    content_registry: Res<crate::plugins::ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    audio_settings: Res<AudioSettingsState>,
    sim_rate: Res<SimulationRateConfig>,
    gas_simulation: Res<GasSimulationConfig>,
    gas_visual_settings: Res<GasVisualSettings>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut active_entity_category: ResMut<ActiveEntityCategory>,
    mut category_ui_registry: ResMut<EntityCategoryUiRegistry>,
    mut main_toolbar_layout: ResMut<MainToolbarLayout>,
    mut panel_manager: ResMut<PanelManager>,
    mut panel_open_order: ResMut<PanelOpenOrder>,
    mut select_fields: ResMut<SelectFieldState>,
    mut slider_state: ResMut<SliderState>,
) {
    let fmt_f32 = |v: f32| {
        let s = format!("{:.3}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let sim_hz_initial = sim_rate.target_hz.clamp(1, 1000);
    let buoyancy_strength_initial = gas_simulation
        .solver_tuning
        .buoyancy_strength
        .clamp(0.0, 5.0);
    let buoyancy_radius_initial = u32::from(
        gas_simulation
            .solver_tuning
            .buoyancy_window_radius
            .clamp(1, 3),
    );
    let buoyancy_sigma_initial = gas_simulation
        .solver_tuning
        .buoyancy_window_sigma
        .clamp(0.5, 3.0);
    let buoyancy_gain_initial = gas_simulation.solver_tuning.buoyancy_gain.clamp(0.0, 10.0);
    let buoyancy_alpha_initial = gas_simulation.solver_tuning.buoyancy_alpha.clamp(0.0, 4.0);
    let buoyancy_cap_initial = gas_simulation
        .solver_tuning
        .buoyancy_force_cap
        .clamp(0.0, 2.0);
    let gamma_initial = gas_visual_settings.gamma.clamp(0.0, 10.0);
    let max_color_initial = gas_visual_settings
        .max_particles_for_max_color
        .clamp(1, 10_000);
    let gas_select_options = gas_select_options(&gas_registry);
    let sim_hz_initial_text = sim_hz_initial.to_string();
    let buoyancy_strength_initial_text = fmt_f32(buoyancy_strength_initial);
    let buoyancy_radius_initial_text = buoyancy_radius_initial.to_string();
    let buoyancy_sigma_initial_text = fmt_f32(buoyancy_sigma_initial);
    let buoyancy_gain_initial_text = fmt_f32(buoyancy_gain_initial);
    let buoyancy_alpha_initial_text = fmt_f32(buoyancy_alpha_initial);
    let buoyancy_cap_initial_text = fmt_f32(buoyancy_cap_initial);
    let gamma_initial_text = fmt_f32(gamma_initial);
    let max_color_initial_text = max_color_initial.to_string();
    let icon_set = EditorIconSet {
        add_gas: asset_server.load("sprites/ui/tool_add_gas.ktx2"),
        clear_gas: asset_server.load("sprites/ui/tool_clear_gas.ktx2"),
        source: asset_server.load(crate::plugins::default_plugin::structure_tool_icon_path(
            crate::plugins::default_plugin::gas_source_structure_kind(),
        )),
        sink: asset_server.load(crate::plugins::default_plugin::structure_tool_icon_path(
            crate::plugins::default_plugin::gas_sink_structure_kind(),
        )),
        pipe_silhouette: asset_server.load(
            crate::plugins::default_plugin::structure_silhouette_path(
                crate::plugins::default_plugin::pipe_structure_kind(),
            )
            .expect("pipe silhouette is registered"),
        ),
        vent_silhouette: asset_server.load(
            crate::plugins::default_plugin::structure_silhouette_path(
                crate::plugins::default_plugin::vent_structure_kind(),
            )
            .expect("vent silhouette is registered"),
        ),
        bridge_silhouette: asset_server.load(
            crate::plugins::default_plugin::structure_silhouette_path(
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
            )
            .expect("bridge silhouette is registered"),
        ),
        pump_silhouette: asset_server.load(
            crate::plugins::default_plugin::structure_silhouette_path(
                crate::plugins::default_plugin::gas_pump_structure_kind(),
            )
            .expect("pump silhouette is registered"),
        ),
        source_silhouette: asset_server.load(
            crate::plugins::default_plugin::structure_silhouette_path(
                crate::plugins::default_plugin::gas_source_structure_kind(),
            )
            .expect("source silhouette is registered"),
        ),
        sink_silhouette: asset_server.load(
            crate::plugins::default_plugin::structure_silhouette_path(
                crate::plugins::default_plugin::gas_sink_structure_kind(),
            )
            .expect("sink silhouette is registered"),
        ),
        select_arrow: asset_server.load("sprites/ui/select_arrow.ktx2"),
        main_menu_background: asset_server.load("sprites/ui/main_menu_background.png"),
    };
    commands.insert_resource(icon_set.clone());

    select_fields.register_field(SelectFieldConfig {
        id: GAS_SELECT_ADD_ID,
        options: gas_select_options.clone(),
        selected: 0,
    });
    select_fields.register_field(SelectFieldConfig {
        id: GAS_SELECT_SOURCE_ID,
        options: gas_select_options.clone(),
        selected: 0,
    });
    slider_state.register_slider(SliderConfig::with_default_width(
        SETTINGS_MUSIC_VOLUME_SLIDER_ID,
        0,
        100,
        1,
        audio_settings.runtime_music_volume_percent as i32,
    ));

    let mut category_items: Vec<_> = content_registry
        .entity_categories()
        .values()
        .cloned()
        .collect();
    let cells_category_id = crate::plugins::default_plugin::cells_category_content_id();
    let gases_category_id = crate::plugins::default_plugin::gases_category_content_id();
    category_items.sort_by(|left, right| {
        let left_priority = entity_category_toolbar_priority(&left.id, &cells_category_id, &gases_category_id);
        let right_priority =
            entity_category_toolbar_priority(&right.id, &cells_category_id, &gases_category_id);
        left_priority
            .cmp(&right_priority)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.id.cmp(&right.id))
    });

    let main_toolbar_width = if category_items.is_empty() {
        MAIN_TOOLBAR_PADDING * 2.0
    } else {
        (category_items.len() as f32 * MAIN_TOOL_BUTTON_SIZE)
            + ((category_items.len() as f32 - 1.0) * MAIN_TOOL_BUTTON_GAP)
            + (MAIN_TOOLBAR_PADDING * 2.0)
    };
    main_toolbar_layout.width = main_toolbar_width;
    main_toolbar_layout.height = MAIN_TOOLBAR_HEIGHT;

    let ui_categories = category_items
        .iter()
        .map(|category| {
            let cell_variants =
                collect_cell_variants_for_category(&content_registry, &asset_server, &category.id);
            let gas_variants =
                collect_pipe_variants_for_category(&content_registry, &asset_server, &category.id);
            let kind = if category.id == cells_category_id || !cell_variants.is_empty() {
                EntityCategoryKind::Cells
            } else if category.id == gases_category_id || !gas_variants.is_empty() {
                EntityCategoryKind::Gases
            } else {
                EntityCategoryKind::Other
            };
            (
                EntityCategoryUiDescriptor {
                    id: category.id.clone(),
                    label: category.label.clone(),
                    icon: asset_server.load(&category.icon_path),
                    panel_id: panel_id_for_entity_category(&category.id),
                    kind,
                },
                cell_variants,
                gas_variants,
            )
        })
        .collect::<Vec<_>>();

    let initial_category = ui_categories
        .iter()
        .find(|(descriptor, _, _)| descriptor.id == cells_category_id)
        .or_else(|| ui_categories.first())
        .map(|(descriptor, _, _)| descriptor.id.clone());
    active_entity_category.set(initial_category.clone());
    active_tool.selected = None;
    category_ui_registry.items = ui_categories
        .iter()
        .map(|(descriptor, _, _)| descriptor.clone())
        .collect();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MAIN_TOOLBAR_LEFT),
                bottom: Val::Px(MAIN_TOOLBAR_BOTTOM),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(MAIN_TOOL_BUTTON_GAP),
                width: Val::Px(main_toolbar_width),
                height: Val::Px(MAIN_TOOLBAR_HEIGHT),
                padding: UiRect::all(Val::Px(MAIN_TOOLBAR_PADDING)),
                ..default()
            },
            BackgroundColor(crate::ui::palette::TRANSPARENT),
            MainToolbarRoot,
        ))
        .with_children(|parent| {
            for (descriptor, _, _) in &ui_categories {
                spawn_entity_category_button(
                    parent,
                    descriptor.label.clone(),
                    descriptor.id.clone(),
                    descriptor.icon.clone(),
                    MAIN_TOOL_BUTTON_SIZE,
                    MAIN_TOOL_ICON_SIZE,
                );
            }
        });

    for (descriptor, cell_variants, gas_variants) in &ui_categories {
        panel_manager.spawn_panel(
            &mut commands,
            &mut panel_open_order,
            PanelSpec {
                id: descriptor.panel_id,
                title: descriptor.label.clone(),
                collapse_icon: None,
                corner: PanelCorner::BottomLeft,
                width: TOOL_VARIANT_PANEL_WIDTH,
                margin_x: MAIN_TOOLBAR_LEFT,
                margin_y: TOOL_VARIANT_PANEL_BOTTOM,
                stack_gap: DEFAULT_PANEL_STACK_GAP,
                controls: PanelControls {
                    show_collapse: false,
                    show_close: false,
                    custom_actions: vec![TOOL_VARIANT_PANEL_CLOSE_ACTION_ID],
                },
                scroll_policy: PanelScrollPolicy::AutoHalfScreen,
                background: PANEL_BG,
                header_background: crate::ui::palette::PANEL_HEADER_BG,
                initial_visible: false,
                initial_collapsed: false,
            },
            |parent| match descriptor.kind {
                EntityCategoryKind::Cells => {
                    spawn_cells_variant_panel_content(parent, cell_variants);
                }
                EntityCategoryKind::Gases => {
                    spawn_gases_variant_panel_content(parent, gas_variants);
                }
                EntityCategoryKind::Other => {}
            },
        );
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(DEBUG_TOOLBAR_LEFT),
                top: Val::Px(DEBUG_TOOLBAR_TOP),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(DEBUG_TOOL_BUTTON_GAP),
                width: Val::Px(DEBUG_TOOLBAR_WIDTH),
                height: Val::Px(DEBUG_TOOLBAR_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DebugToolbarRoot,
        ))
        .with_children(|parent| {
            spawn_tool_button(
                parent,
                "Add Gas",
                EditorTool::AddGas,
                icon_set.add_gas.clone(),
                DEBUG_TOOL_BUTTON_SIZE,
                DEBUG_TOOL_ICON_SIZE,
            );
            spawn_tool_button(
                parent,
                "Clear Gas",
                EditorTool::ClearGas,
                icon_set.clear_gas.clone(),
                DEBUG_TOOL_BUTTON_SIZE,
                DEBUG_TOOL_ICON_SIZE,
            );
            spawn_tool_button(
                parent,
                crate::plugins::default_plugin::structure_label(
                    crate::plugins::default_plugin::gas_source_structure_kind(),
                ),
                EditorTool::CreateGasSource,
                icon_set.source.clone(),
                DEBUG_TOOL_BUTTON_SIZE,
                DEBUG_TOOL_ICON_SIZE,
            );
            spawn_tool_button(
                parent,
                crate::plugins::default_plugin::structure_label(
                    crate::plugins::default_plugin::gas_sink_structure_kind(),
                ),
                EditorTool::CreateGasSink,
                icon_set.sink.clone(),
                DEBUG_TOOL_BUTTON_SIZE,
                DEBUG_TOOL_ICON_SIZE,
            );
        });

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: DEBUG_PANEL_ID,
            title: "Debug Panel".to_string(),
            collapse_icon: Some(icon_set.select_arrow.clone()),
            corner: PanelCorner::TopRight,
            width: DEBUG_PANEL_WIDTH,
            margin_x: DEBUG_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEFAULT_PANEL_STACK_GAP,
            controls: PanelControls {
                show_collapse: true,
                show_close: false,
                custom_actions: Vec::new(),
            },
            scroll_policy: PanelScrollPolicy::AutoHalfScreen,
            background: PANEL_BG,
            header_background: crate::ui::palette::PANEL_HEADER_BG,
            initial_visible: false,
            initial_collapsed: false,
        },
        |parent| {
            spawn_debug_panel_content(
                parent,
                icon_set.select_arrow.clone(),
                sim_hz_initial,
                sim_hz_initial_text.clone(),
                buoyancy_strength_initial,
                buoyancy_strength_initial_text.clone(),
                buoyancy_radius_initial,
                buoyancy_radius_initial_text.clone(),
                buoyancy_sigma_initial,
                buoyancy_sigma_initial_text.clone(),
                buoyancy_gain_initial,
                buoyancy_gain_initial_text.clone(),
                buoyancy_alpha_initial,
                buoyancy_alpha_initial_text.clone(),
                buoyancy_cap_initial,
                buoyancy_cap_initial_text.clone(),
                gamma_initial,
                gamma_initial_text.clone(),
                max_color_initial,
                max_color_initial_text.clone(),
            );
        },
    );

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: GAS_TOOL_PANEL_ID,
            title: "Gas Panel".to_string(),
            collapse_icon: Some(icon_set.select_arrow.clone()),
            corner: PanelCorner::TopRight,
            width: GAS_PANEL_WIDTH,
            margin_x: GAS_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEFAULT_PANEL_STACK_GAP,
            controls: PanelControls {
                show_collapse: true,
                show_close: false,
                custom_actions: Vec::new(),
            },
            scroll_policy: PanelScrollPolicy::AutoHalfScreen,
            background: PANEL_BG,
            header_background: crate::ui::palette::PANEL_HEADER_BG,
            initial_visible: false,
            initial_collapsed: false,
        },
        |parent| {
            spawn_gas_tool_panel_content(parent, &gas_select_options, icon_set.select_arrow.clone());
        },
    );

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: STRUCTURE_TOOL_PANEL_ID,
            title: "Structure Panel".to_string(),
            collapse_icon: Some(icon_set.select_arrow.clone()),
            corner: PanelCorner::TopRight,
            width: STRUCTURE_PANEL_WIDTH,
            margin_x: STRUCTURE_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEFAULT_PANEL_STACK_GAP,
            controls: PanelControls {
                show_collapse: true,
                show_close: false,
                custom_actions: Vec::new(),
            },
            scroll_policy: PanelScrollPolicy::Never,
            background: PANEL_BG,
            header_background: crate::ui::palette::PANEL_HEADER_BG,
            initial_visible: false,
            initial_collapsed: false,
        },
        |parent| {
            spawn_structure_tool_panel_content(
                parent,
                &gas_select_options,
                icon_set.select_arrow.clone(),
            );
        },
    );

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(crate::ui::palette::TRANSPARENT),
            GlobalZIndex(1500),
            Visibility::Hidden,
            MainMenuRoot,
            ModalRoot,
            ModalBackdropController,
            ModalBackdropSpec::panel_frosted(
                ModalBackdropSource::Asset(icon_set.main_menu_background.clone()),
                crate::ui::modal::MAIN_MENU_PANEL_TINT,
            ),
        ))
        .with_children(|parent| {
            spawn_modal_backdrop_chrome(parent);
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Relative,
                        width: Val::Px(780.0),
                        height: Val::Px(640.0),
                        display: Display::Flex,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ImageNode::default(),
                    BackgroundColor(crate::ui::modal::MAIN_MENU_PANEL_TINT),
                    modal_panel_box_shadow(),
                    ModalPanelSurface,
                ))
                .with_children(|panel| {
                    spawn_modal_panel_backdrop(panel);
                    panel.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(10.0),
                            padding: UiRect::all(Val::Px(18.0)),
                            ..default()
                        },
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            Text::new("Main Menu"),
                            TextFont::from_font_size(24.0),
                            TextColor(crate::ui::palette::TEXT_HEADER),
                            TextLayout::new_with_justify(JustifyText::Center),
                            MainMenuTitleText,
                        ));
                        panel.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            Text::new(""),
                            TextFont::from_font_size(14.0),
                            TextColor(crate::ui::palette::TEXT_SECONDARY),
                            TextLayout::new_with_justify(JustifyText::Center),
                            MainMenuStatusText,
                        ));

                        panel
                            .spawn((
                                Node {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Column,
                                    width: Val::Percent(100.0),
                                    align_items: AlignItems::Center,
                                    row_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuRootActions,
                            ))
                            .with_children(|actions| {
                                spawn_main_menu_action_button(
                                    actions,
                                    "Continue",
                                    MainMenuButtonAction::Continue,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "New Game",
                                    MainMenuButtonAction::NewGame,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Save",
                                    MainMenuButtonAction::OpenSaveScreen,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Load",
                                    MainMenuButtonAction::OpenLoadScreen,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Plugins",
                                    MainMenuButtonAction::OpenPluginsScreen,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Settings",
                                    MainMenuButtonAction::OpenSettingsScreen,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Exit To Main",
                                    MainMenuButtonAction::ExitToMainMenu,
                                    220.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Exit",
                                    MainMenuButtonAction::ExitApp,
                                    220.0,
                                );
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuSaveNameRow,
                            ))
                            .with_children(|row| {
                                row.spawn((
                                    Text::new("Save Name:"),
                                    TextFont::from_font_size(14.0),
                                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                                    TextLayout::new_with_justify(JustifyText::Center),
                                ));
                                row.spawn((
                                    Button,
                                    Node {
                                        min_width: Val::Px(520.0),
                                        height: Val::Px(34.0),
                                        justify_content: JustifyContent::FlexStart,
                                        align_items: AlignItems::Center,
                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                        ..default()
                                    },
                                    BackgroundColor(MENU_MODAL_INPUT_BG),
                                    TextInputField::new_string(
                                        "New Save",
                                        64,
                                        crate::ui::input_field::InputAllowedChars::Any,
                                    ),
                                    TextInputStyle {
                                        idle_bg: MENU_MODAL_INPUT_BG,
                                        focused_bg: MENU_MODAL_INPUT_FOCUSED,
                                    },
                                    bevy::ui::RelativeCursorPosition::default(),
                                    MainMenuSaveNameInputField,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("New Save"),
                                        TextFont::from_font_size(14.0),
                                        TextColor(crate::ui::palette::TEXT_PRIMARY),
                                        TextInputDisplay,
                                    ));
                                });
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuSaveActions,
                            ))
                            .with_children(|actions| {
                                spawn_main_menu_action_button(
                                    actions,
                                    "Back",
                                    MainMenuButtonAction::BackToRoot,
                                    140.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Create New Save",
                                    MainMenuButtonAction::CreateNewSave,
                                    220.0,
                                );
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuLoadActions,
                            ))
                            .with_children(|actions| {
                                spawn_main_menu_action_button(
                                    actions,
                                    "Back",
                                    MainMenuButtonAction::BackToRoot,
                                    140.0,
                                );
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(0.0),
                                    padding: UiRect::axes(Val::Px(24.0), Val::Px(0.0)),
                                    margin: UiRect::bottom(Val::Px(-10.0)),
                                    ..default()
                                },
                                MainMenuSettingsActions,
                            ))
                            .with_children(|actions| {
                                spawn_main_menu_settings_tab_button(
                                    actions,
                                    "Graphics",
                                    MainMenuButtonAction::SettingsTabGraphics,
                                    160.0,
                                );
                                spawn_main_menu_settings_tab_button(
                                    actions,
                                    "Sound",
                                    MainMenuButtonAction::SettingsTabSound,
                                    160.0,
                                );
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::FlexStart,
                                    padding: UiRect::all(Val::Px(22.0)),
                                    border: UiRect::all(Val::Px(0.0)),
                                    row_gap: Val::Px(14.0),
                                    flex_grow: 1.0,
                                    justify_content: JustifyContent::FlexStart,
                                    ..default()
                                },
                                BackgroundColor(MENU_MODAL_CARD_BG),
                                MainMenuSettingsGraphicsContent,
                            ))
                            .with_children(|content| {
                                content.spawn((
                                    Text::new(
                                        "Graphics settings are coming soon.\nThis tab is intentionally simple for tab testing.",
                                    ),
                                    TextFont::from_font_size(15.0),
                                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                                    TextLayout::new_with_justify(JustifyText::Center),
                                ));
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::FlexStart,
                                    padding: UiRect::all(Val::Px(22.0)),
                                    border: UiRect::all(Val::Px(0.0)),
                                    row_gap: Val::Px(14.0),
                                    flex_grow: 1.0,
                                    justify_content: JustifyContent::FlexStart,
                                    ..default()
                                },
                                BackgroundColor(MENU_MODAL_CARD_BG),
                                MainMenuSettingsSoundContent,
                            ))
                            .with_children(|content| {
                                content.spawn((
                                    Text::new("Music Volume"),
                                    TextFont::from_font_size(16.0),
                                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                                ));
                                content
                                    .spawn((Node {
                                        display: Display::Flex,
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        column_gap: Val::Px(12.0),
                                        ..default()
                                    },))
                                    .with_children(|slider_row| {
                                        spawn_slider(
                                            slider_row,
                                            SliderConfig::with_default_width(
                                                SETTINGS_MUSIC_VOLUME_SLIDER_ID,
                                                0,
                                                100,
                                                1,
                                                audio_settings.runtime_music_volume_percent as i32,
                                            ),
                                        );
                                        slider_row.spawn((
                                            Text::new(
                                                audio_settings.runtime_music_volume_percent.to_string(),
                                            ),
                                            TextFont::from_font_size(16.0),
                                            TextColor(crate::ui::palette::TEXT_PRIMARY),
                                            MainMenuSettingsMusicVolumeValueText,
                                        ));
                                    });
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuSettingsFooterActions,
                            ))
                            .with_children(|actions| {
                                spawn_main_menu_action_button(
                                    actions,
                                    "Back",
                                    MainMenuButtonAction::BackToRoot,
                                    140.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Save",
                                    MainMenuButtonAction::SaveSettings,
                                    140.0,
                                );
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuPluginsActions,
                            ))
                            .with_children(|actions| {
                                spawn_main_menu_action_button(
                                    actions,
                                    "Reload",
                                    MainMenuButtonAction::ReloadPlugins,
                                    140.0,
                                );
                                spawn_main_menu_action_button(
                                    actions,
                                    "Back",
                                    MainMenuButtonAction::BackToRoot,
                                    140.0,
                                );
                            });

                        panel
                            .spawn((
                                Node {
                                    display: Display::None,
                                    flex_direction: FlexDirection::Row,
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(8.0),
                                    ..default()
                                },
                                MainMenuConfirmActions,
                            ))
                            .with_children(|actions| {
                                actions
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(160.0),
                                            height: Val::Px(38.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(MODAL_BUTTON_BG),
                                        MainMenuActionButton(MainMenuButtonAction::ConfirmPrimary),
                                        MainMenuConfirmPrimaryLabel,
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new("Yes"),
                                            TextFont::from_font_size(15.0),
                                            TextColor(crate::ui::palette::TEXT_ON_DARK),
                                            TextLayout::new_with_justify(JustifyText::Center),
                                        ));
                                    });
                                actions
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(160.0),
                                            height: Val::Px(38.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(MODAL_BUTTON_BG),
                                        MainMenuActionButton(MainMenuButtonAction::ConfirmSecondary),
                                        MainMenuConfirmSecondaryLabel,
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new("No"),
                                            TextFont::from_font_size(15.0),
                                            TextColor(crate::ui::palette::TEXT_ON_DARK),
                                            TextLayout::new_with_justify(JustifyText::Center),
                                        ));
                                    });
                                actions
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(160.0),
                                            height: Val::Px(38.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(MODAL_BUTTON_BG),
                                        MainMenuActionButton(MainMenuButtonAction::ConfirmCancel),
                                        MainMenuConfirmCancelLabel,
                                    ))
                                    .with_children(|button| {
                                        button.spawn((
                                            Text::new("Cancel"),
                                            TextFont::from_font_size(15.0),
                                            TextColor(crate::ui::palette::TEXT_ON_DARK),
                                            TextLayout::new_with_justify(JustifyText::Center),
                                        ));
                                    });
                            });

                        panel.spawn((
                            Node {
                                display: Display::None,
                                position_type: PositionType::Relative,
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                ..default()
                            },
                            MainMenuSaveListRoot,
                        ))
                        .with_children(|list_root| {
                            let viewport = list_root
                                .spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        display: Display::Flex,
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Stretch,
                                        padding: UiRect::right(Val::Px(10.0)),
                                        overflow: Overflow::scroll_y(),
                                        min_height: Val::Px(0.0),
                                        ..default()
                                    },
                                    bevy::ui::ScrollPosition::default(),
                                    bevy::ui::RelativeCursorPosition::default(),
                                    ScrollAreaViewport::modal(100),
                                    MainMenuSaveListViewport,
                                ))
                                .with_children(|viewport| {
                                    viewport.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            display: Display::Flex,
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Stretch,
                                            row_gap: Val::Px(10.0),
                                            ..default()
                                        },
                                        MainMenuSaveListContent,
                                    ));
                                })
                                .id();
                            spawn_scroll_area_scrollbar(list_root, viewport);
                        });

                        panel.spawn((
                            Node {
                                display: Display::None,
                                position_type: PositionType::Relative,
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                min_height: Val::Px(0.0),
                                ..default()
                            },
                            MainMenuPluginsListRoot,
                        ))
                        .with_children(|list_root| {
                            let viewport = list_root
                                .spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        display: Display::Flex,
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Stretch,
                                        padding: UiRect::right(Val::Px(10.0)),
                                        overflow: Overflow::scroll_y(),
                                        min_height: Val::Px(0.0),
                                        ..default()
                                    },
                                    bevy::ui::ScrollPosition::default(),
                                    bevy::ui::RelativeCursorPosition::default(),
                                    ScrollAreaViewport::modal(110),
                                    MainMenuPluginsListViewport,
                                ))
                                .with_children(|viewport| {
                                    viewport.spawn((
                                        Node {
                                            width: Val::Percent(100.0),
                                            display: Display::Flex,
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Stretch,
                                            row_gap: Val::Px(10.0),
                                            ..default()
                                        },
                                        MainMenuPluginsListContent,
                                    ));
                                })
                                .id();
                            spawn_scroll_area_scrollbar(list_root, viewport);
                        });
                    });
                });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                display: Display::None,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(TOOLTIP_BG),
            GlobalZIndex(2000),
            UiTooltipRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(crate::ui::palette::TEXT_ON_DARK),
                UiTooltipText,
            ));
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                display: Display::None,
                min_width: Val::Px(94.0),
                padding: UiRect::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(TOOLTIP_BG),
            GlobalZIndex(2000),
            SelectionSizeTooltip,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(crate::ui::palette::TEXT_ON_DARK),
                TextLayout::new_with_justify(JustifyText::Center),
                SelectionSizeTooltipText,
            ));
        });
}

fn panel_id_for_entity_category(category_id: &ContentId) -> PanelId {
    let sanitized = category_id
        .as_str()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let leaked = Box::leak(format!("entity_category_panel_{sanitized}").into_boxed_str());
    PanelId::new(leaked)
}

fn entity_category_toolbar_priority(
    category_id: &ContentId,
    cells_category_id: &ContentId,
    gases_category_id: &ContentId,
) -> u8 {
    if category_id == cells_category_id {
        0
    } else if category_id == gases_category_id {
        1
    } else {
        2
    }
}

fn collect_cell_variants_for_category(
    content_registry: &crate::plugins::ContentRegistry,
    asset_server: &AssetServer,
    category_id: &ContentId,
) -> Vec<(String, CellMaterial, Handle<Image>)> {
    let mut descriptors = content_registry
        .cells()
        .values()
        .filter(|descriptor| descriptor.category_id.as_ref() == Some(category_id))
        .collect::<Vec<_>>();
    descriptors.sort_by(|left, right| left.id.cmp(&right.id));

    descriptors
        .into_iter()
        .map(|descriptor| {
            let label = if descriptor.plugin_id == PluginId::default_plugin() {
                crate::plugins::default_plugin::cell_label(descriptor.material).to_string()
            } else {
                descriptor.id.as_str().to_string()
            };
            (
                label,
                descriptor.material,
                asset_server.load(descriptor.sprite.image_path.as_str()),
            )
        })
        .collect()
}

fn collect_pipe_variants_for_category(
    content_registry: &crate::plugins::ContentRegistry,
    asset_server: &AssetServer,
    category_id: &ContentId,
) -> Vec<(String, PipeToolKind, Handle<Image>, f32)> {
    let mut descriptors = content_registry
        .structures()
        .values()
        .filter(|descriptor| descriptor.category_id.as_ref() == Some(category_id))
        .collect::<Vec<_>>();
    descriptors.sort_by(|left, right| left.id.cmp(&right.id));

    descriptors
        .into_iter()
        .filter_map(|descriptor| {
            let pipe_kind = if descriptor.kind == crate::plugins::default_plugin::pipe_structure_kind()
            {
                Some(PipeToolKind::Pipe)
            } else if descriptor.kind == crate::plugins::default_plugin::vent_structure_kind() {
                Some(PipeToolKind::Vent)
            } else if descriptor.kind
                == crate::plugins::default_plugin::gas_pipe_bridge_structure_kind()
            {
                Some(PipeToolKind::Bridge)
            } else if descriptor.kind == crate::plugins::default_plugin::gas_pump_structure_kind() {
                Some(PipeToolKind::Pump)
            } else {
                None
            }?;
            let label = if descriptor.plugin_id == PluginId::default_plugin() {
                crate::plugins::default_plugin::structure_label(descriptor.kind).to_string()
            } else {
                descriptor.id.as_str().to_string()
            };
            let size = descriptor.visual.size_in_cells;
            let aspect_ratio = size.x.max(1) as f32 / size.y.max(1) as f32;
            Some((
                label,
                pipe_kind,
                asset_server.load(descriptor.sprite.image_path.as_str()),
                aspect_ratio,
            ))
        })
        .collect()
}
