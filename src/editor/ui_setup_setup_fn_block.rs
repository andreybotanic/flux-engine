fn setup_editor_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gas_registry: Res<GasRegistry>,
    sim_rate: Res<SimulationRateConfig>,
    gas_simulation: Res<GasSimulationConfig>,
    gas_visual_settings: Res<GasVisualSettings>,
    mut panel_manager: ResMut<PanelManager>,
    mut panel_open_order: ResMut<PanelOpenOrder>,
    mut select_fields: ResMut<SelectFieldState>,
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
    let gas_select_options: Vec<String> = gas_registry
        .all()
        .iter()
        .map(|gas| gas.id.to_uppercase())
        .collect();
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
        build: asset_server.load("sprites/ui/tool_build.png"),
        gases: asset_server.load("sprites/world/pipe_mask_10.png"),
        erase: asset_server.load("sprites/ui/tool_erase.png"),
        pipe: asset_server.load("sprites/world/pipe_mask_10.png"),
        vent: asset_server.load("sprites/world/tile_vent.png"),
        bridge: asset_server.load("sprites/ui/tool_bridge.png"),
        add_gas: asset_server.load("sprites/ui/tool_add_gas.png"),
        clear_gas: asset_server.load("sprites/ui/tool_clear_gas.png"),
        source: asset_server.load("sprites/world/tile_gas_source.png"),
        sink: asset_server.load("sprites/world/tile_gas_sink.png"),
        brick: asset_server.load("sprites/world/tile_brick.png"),
        metal: asset_server.load("sprites/world/tile_metal.png"),
        brick_silhouette: asset_server.load("sprites/world/silhouette_brick.png"),
        metal_silhouette: asset_server.load("sprites/world/silhouette_metal.png"),
        pipe_silhouette: asset_server.load("sprites/world/pipe_silhouette_mask_00.png"),
        vent_silhouette: asset_server.load("sprites/world/silhouette_vent.png"),
        bridge_silhouette: asset_server.load("sprites/world/bridge_silhouette.png"),
        source_silhouette: asset_server.load("sprites/world/tile_gas_source.png"),
        sink_silhouette: asset_server.load("sprites/world/tile_gas_sink.png"),
        select_arrow: asset_server.load("sprites/ui/select_arrow.png"),
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

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MAIN_TOOLBAR_LEFT),
                bottom: Val::Px(MAIN_TOOLBAR_BOTTOM),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(MAIN_TOOLBAR_WIDTH),
                height: Val::Px(MAIN_TOOLBAR_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            MainToolbarRoot,
        ))
        .with_children(|parent| {
            spawn_tool_button(
                parent,
                "Build",
                EditorTool::BuildSolid,
                icon_set.build.clone(),
            );
            spawn_tool_button(
                parent,
                "Gases",
                EditorTool::Gases,
                icon_set.gases.clone(),
            );
            spawn_tool_button(
                parent,
                "Erase",
                EditorTool::EraseSolid,
                icon_set.erase.clone(),
            );
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MAIN_TOOLBAR_LEFT),
                bottom: Val::Px(CELL_TYPE_PANEL_BOTTOM),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(CELL_TYPE_PANEL_WIDTH),
                height: Val::Px(CELL_TYPE_PANEL_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            GasesTypePanelRoot,
        ))
        .with_children(|parent| {
            spawn_pipe_tool_button(parent, "Pipe", PipeToolKind::Pipe, icon_set.pipe.clone());
            spawn_pipe_tool_button(parent, "Vent", PipeToolKind::Vent, icon_set.vent.clone());
            spawn_pipe_tool_button(parent, "Bridge", PipeToolKind::Bridge, icon_set.bridge.clone());
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MAIN_TOOLBAR_LEFT),
                bottom: Val::Px(CELL_TYPE_PANEL_BOTTOM),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(CELL_TYPE_PANEL_WIDTH),
                height: Val::Px(CELL_TYPE_PANEL_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            CellTypePanelRoot,
        ))
        .with_children(|parent| {
            spawn_cell_material_button(
                parent,
                "Brick",
                CellMaterial::Brick,
                icon_set.brick.clone(),
            );
            spawn_cell_material_button(
                parent,
                "Metal",
                CellMaterial::Metal,
                icon_set.metal.clone(),
            );
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(DEBUG_TOOLBAR_LEFT),
                top: Val::Px(DEBUG_TOOLBAR_TOP),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
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
            );
            spawn_tool_button(
                parent,
                "Clear Gas",
                EditorTool::ClearGas,
                icon_set.clear_gas.clone(),
            );
            spawn_tool_button(
                parent,
                "Create Gas Source",
                EditorTool::CreateGasSource,
                icon_set.source.clone(),
            );
            spawn_tool_button(
                parent,
                "Create Gas Sink",
                EditorTool::CreateGasSink,
                icon_set.sink.clone(),
            );
        });

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: DEBUG_PANEL_ID,
            title: "Debug Panel".to_string(),
            corner: PanelCorner::TopRight,
            width: DEBUG_PANEL_WIDTH,
            margin_x: DEBUG_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEBUG_AND_GAS_PANEL_GAP,
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
            spawn_debug_panel_content(
                parent,
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
            corner: PanelCorner::TopRight,
            width: GAS_PANEL_WIDTH,
            margin_x: GAS_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEBUG_AND_GAS_PANEL_GAP,
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
            corner: PanelCorner::TopRight,
            width: STRUCTURE_PANEL_WIDTH,
            margin_x: STRUCTURE_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEBUG_AND_GAS_PANEL_GAP,
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
            BackgroundColor(MODAL_OVERLAY_BG),
            GlobalZIndex(1500),
            Visibility::Hidden,
            MainMenuRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                ImageNode::new(asset_server.load("sprites/ui/main_menu_background.png")),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                MainMenuBackdrop,
            ));
            parent
                .spawn((
                    Node {
                        width: Val::Px(780.0),
                        height: Val::Px(640.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        ..default()
                    },
                    BackgroundColor(MODAL_BG),
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
                                BackgroundColor(BUTTON_IDLE),
                                TextInputField::new_string(
                                    "New Save",
                                    64,
                                    crate::ui::input_field::InputAllowedChars::Any,
                                ),
                                TextInputStyle {
                                    idle_bg: BUTTON_IDLE,
                                    focused_bg: INPUT_FOCUSED,
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

