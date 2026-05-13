fn refresh_main_menu_ui(
    mut commands: Commands,
    main_menu: Res<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    slider_state: Res<SliderState>,
    audio_settings: Res<AudioSettingsState>,
    shared_resources: (
        Res<PluginRegistryState>,
        Res<EnabledPluginSet>,
        Res<WorldLoadState>,
        Res<EditorIconSet>,
    ),
    mut images: ResMut<Assets<Image>>,
    mut text_color_query: Query<&mut TextColor>,
    modal_state: (
        Single<&mut Visibility, With<MainMenuRoot>>,
        Single<&mut ModalBackdropSpec, With<MainMenuRoot>>,
    ),
    mut text_set: ParamSet<(
        Single<&mut Text, With<MainMenuTitleText>>,
        Single<&mut Text, With<MainMenuStatusText>>,
        Single<&mut Text, With<MainMenuSettingsMusicVolumeValueText>>,
        Query<&mut Text>,
    )>,
    menu_nodes: (
        Single<Entity, With<MainMenuRootActions>>,
        Single<Entity, With<MainMenuPluginsActions>>,
        Single<Entity, With<MainMenuSaveActions>>,
        Single<Entity, With<MainMenuLoadActions>>,
        Single<Entity, With<MainMenuConfirmActions>>,
        Single<Entity, With<MainMenuSaveNameRow>>,
        Single<Entity, With<MainMenuSaveListRoot>>,
        Single<Entity, With<MainMenuPluginsListRoot>>,
        Single<Entity, With<MainMenuSettingsActions>>,
        Single<Entity, With<MainMenuSettingsFooterActions>>,
        Single<Entity, With<MainMenuSettingsGraphicsContent>>,
        Single<Entity, With<MainMenuSettingsSoundContent>>,
    ),
    mut node_by_entity: Query<&mut Node, Without<Button>>,
    mut action_button_nodes: Query<
        (
            &MainMenuActionButton,
            &mut Node,
            &mut BackgroundColor,
            Option<&mut MainMenuButtonPalette>,
            Option<&MainMenuSettingsTabButton>,
            Option<&Children>,
        ),
        (
            With<Button>,
            Without<MainMenuRootActions>,
            Without<MainMenuPluginsActions>,
            Without<MainMenuSaveActions>,
            Without<MainMenuLoadActions>,
            Without<MainMenuConfirmActions>,
            Without<MainMenuSaveNameRow>,
            Without<MainMenuSaveListRoot>,
            Without<MainMenuPluginsListRoot>,
        ),
    >,
    confirm_button_set: (
        Single<&Children, With<MainMenuConfirmPrimaryLabel>>,
        Single<&Children, With<MainMenuConfirmSecondaryLabel>>,
        Single<&Children, With<MainMenuConfirmCancelLabel>>,
    ),
    mut save_list_entities: (
        Single<Entity, With<MainMenuSaveListContent>>,
        Single<
            &mut bevy::ui::ScrollPosition,
            (
                With<MainMenuSaveListViewport>,
                Without<MainMenuPluginsListViewport>,
            ),
        >,
    ),
    mut plugin_list_entities: (
        Single<Entity, With<MainMenuPluginsListContent>>,
        Single<
            &mut bevy::ui::ScrollPosition,
            (
                With<MainMenuPluginsListViewport>,
                Without<MainMenuSaveListViewport>,
            ),
        >,
    ),
) {
    let (plugin_registry_state, enabled_plugins, world_load_state, icon_set) = shared_resources;
    let (mut root_visibility, mut backdrop_spec) = modal_state;
    let (
        root_actions_entity,
        plugins_actions_entity,
        save_actions_entity,
        load_actions_entity,
        confirm_actions_entity,
        save_name_row_entity,
        save_list_root_entity,
        plugins_list_root_entity,
        settings_actions_entity,
        settings_footer_entity,
        settings_graphics_entity,
        settings_sound_entity,
    ) = menu_nodes;
    let mut set_node_display = |entity: Entity, display: Display| {
        if let Ok(mut node) = node_by_entity.get_mut(entity) {
            node.display = display;
        }
    };

    **root_visibility = if main_menu.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if !main_menu.open {
        set_node_display(*root_actions_entity, Display::None);
        set_node_display(*plugins_actions_entity, Display::None);
        set_node_display(*save_actions_entity, Display::None);
        set_node_display(*load_actions_entity, Display::None);
        set_node_display(*confirm_actions_entity, Display::None);
        set_node_display(*save_name_row_entity, Display::None);
        set_node_display(*save_list_root_entity, Display::None);
        set_node_display(*plugins_list_root_entity, Display::None);
        set_node_display(*settings_actions_entity, Display::None);
        set_node_display(*settings_footer_entity, Display::None);
        set_node_display(*settings_graphics_entity, Display::None);
        set_node_display(*settings_sound_entity, Display::None);
        return;
    }

    let screen = menu_ui.screen;
    let mode = menu_ui.mode;
    **backdrop_spec = match mode {
        MainMenuMode::Main => ModalBackdropSpec::panel_frosted(
            ModalBackdropSource::Asset(icon_set.main_menu_background.clone()),
            crate::ui::modal::MAIN_MENU_PANEL_TINT,
        )
        .with_panel_overlay_tint(crate::ui::modal::MAIN_MENU_PANEL_OVERLAY_TINT),
        MainMenuMode::InGame | MainMenuMode::Hidden => ModalBackdropSpec::fullscreen_blur(
            ModalBackdropSource::WorldSnapshot,
            crate::ui::modal::IN_GAME_MENU_PANEL_TINT,
        ),
    };

    {
        let mut title_text = text_set.p0();
        title_text.0 = match screen {
            MainMenuScreen::Root => match mode {
                MainMenuMode::Main => "Main Menu".to_string(),
                MainMenuMode::InGame => "Game Menu".to_string(),
                MainMenuMode::Hidden => "Menu".to_string(),
            },
            MainMenuScreen::Plugins => "Plugins".to_string(),
            MainMenuScreen::Settings => "Settings".to_string(),
            MainMenuScreen::Save => "Save World".to_string(),
            MainMenuScreen::Load => "Load World".to_string(),
            MainMenuScreen::Confirm => "Confirm Action".to_string(),
        };
    }

    let status = if screen == MainMenuScreen::Confirm {
        menu_ui.confirm_text.clone()
    } else if !menu_ui.status_text.is_empty() {
        menu_ui.status_text.clone()
    } else {
        String::new()
    };
    {
        let mut status_text = text_set.p1();
        status_text.0 = status;
    }

    set_node_display(
        *root_actions_entity,
        if screen == MainMenuScreen::Root {
            Display::Flex
        } else {
            Display::None
        },
    );
    for (action_button, mut node, mut background, palette, settings_tab_marker, children) in &mut action_button_nodes {
        node.display = match screen {
            MainMenuScreen::Root => match (&action_button.0, mode) {
                (_, MainMenuMode::Hidden) => Display::None,
                (MainMenuButtonAction::Continue, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::OpenSaveScreen, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::OpenPluginsScreen, _) if root_plugins_button_visible(mode) => {
                    Display::Flex
                }
                (MainMenuButtonAction::OpenSettingsScreen, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::OpenSettingsScreen, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::ExitToMainMenu, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::ExitApp, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::NewGame, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::OpenLoadScreen, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::ExitApp, MainMenuMode::Main) => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Plugins => match action_button.0 {
                MainMenuButtonAction::BackToRoot | MainMenuButtonAction::TogglePlugin(_) => {
                    Display::Flex
                }
                MainMenuButtonAction::ReloadPlugins => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Settings => match action_button.0 {
                MainMenuButtonAction::BackToRoot
                | MainMenuButtonAction::SettingsTabGraphics
                | MainMenuButtonAction::SettingsTabSound
                | MainMenuButtonAction::SaveSettings => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Save => match action_button.0 {
                    MainMenuButtonAction::BackToRoot | MainMenuButtonAction::CreateNewSave => {
                        Display::Flex
                    }
                MainMenuButtonAction::SelectDelete(_) => {
                    Display::Flex
                }
                _ => Display::None,
            },
            MainMenuScreen::Load => match action_button.0 {
                MainMenuButtonAction::BackToRoot => Display::Flex,
                MainMenuButtonAction::SelectDelete(_) => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Confirm => match action_button.0 {
                MainMenuButtonAction::ConfirmPrimary
                | MainMenuButtonAction::ConfirmSecondary
                | MainMenuButtonAction::ConfirmCancel => Display::Flex,
                _ => Display::None,
            },
        };
        if settings_tab_marker.is_some() {
            let is_active_tab = match action_button.0 {
                MainMenuButtonAction::SettingsTabGraphics => {
                    menu_ui.settings_tab == MainMenuSettingsTab::Graphics
                }
                MainMenuButtonAction::SettingsTabSound => {
                    menu_ui.settings_tab == MainMenuSettingsTab::Sound
                }
                _ => false,
            };
            let idle = if is_active_tab {
                MENU_MODAL_CARD_BG
            } else {
                MODAL_BUTTON_BG
            };
            let hover = if is_active_tab {
                MENU_MODAL_INPUT_BG
            } else {
                MODAL_BUTTON_HOVER
            };
            if let Some(mut palette) = palette {
                palette.idle = idle;
                palette.hover = hover;
            }
            background.0 = idle;
            node.border.bottom = Val::Px(0.0);
            node.margin.bottom = Val::Px(0.0);
            if let Some(children) = children {
                let text_color = if is_active_tab {
                    crate::ui::palette::TEXT_PRIMARY
                } else {
                    crate::ui::palette::TEXT_ON_DARK
                };
                for child in children.iter() {
                    if let Ok(mut color) = text_color_query.get_mut(child) {
                        color.0 = text_color;
                    }
                }
            }
        }
    }
    set_node_display(
        *plugins_actions_entity,
        if screen == MainMenuScreen::Plugins {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *settings_actions_entity,
        if screen == MainMenuScreen::Settings {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *settings_footer_entity,
        if screen == MainMenuScreen::Settings {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *save_actions_entity,
        if screen == MainMenuScreen::Save {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *load_actions_entity,
        if screen == MainMenuScreen::Load {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *confirm_actions_entity,
        if screen == MainMenuScreen::Confirm {
            Display::Flex
        } else {
            Display::None
        },
    );

    if screen == MainMenuScreen::Confirm {
        let mut primary_label = "Yes";
        let mut secondary_label = "No";
        let mut show_cancel = false;
        match menu_ui.confirm_state.as_ref() {
            Some(MainMenuConfirmState::UnsavedChanges(_)) => {
                primary_label = "Save";
                secondary_label = "Don't Save";
                show_cancel = true;
            }
            Some(MainMenuConfirmState::OverwriteSave(_)) => {
                primary_label = "Overwrite";
                secondary_label = "Cancel";
            }
            Some(MainMenuConfirmState::DeleteSave(_)) => {
                primary_label = "Delete";
                secondary_label = "Cancel";
            }
            None => {}
        }

        let primary_text_entity = confirm_button_set.0.iter().next();
        if let Some(entity) = primary_text_entity {
            if let Ok(mut text) = text_set.p3().get_mut(entity) {
                text.0 = primary_label.to_string();
            }
        }
        let secondary_text_entity = confirm_button_set.1.iter().next();
        if let Some(entity) = secondary_text_entity {
            if let Ok(mut text) = text_set.p3().get_mut(entity) {
                text.0 = secondary_label.to_string();
            }
        }
        let cancel_text_entity = confirm_button_set.2.iter().next();
        if let Some(entity) = cancel_text_entity {
            if let Ok(mut text) = text_set.p3().get_mut(entity) {
                text.0 = "Cancel".to_string();
            }
        }

        for (action_button, mut node, ..) in &mut action_button_nodes {
            if matches!(action_button.0, MainMenuButtonAction::ConfirmCancel) {
                node.display = if show_cancel {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
    set_node_display(
        *save_name_row_entity,
        if screen == MainMenuScreen::Save {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *save_list_root_entity,
        if matches!(screen, MainMenuScreen::Save | MainMenuScreen::Load) {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *plugins_list_root_entity,
        if screen == MainMenuScreen::Plugins {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *settings_graphics_entity,
        if screen == MainMenuScreen::Settings
            && menu_ui.settings_tab == MainMenuSettingsTab::Graphics
        {
            Display::Flex
        } else {
            Display::None
        },
    );
    set_node_display(
        *settings_sound_entity,
        if screen == MainMenuScreen::Settings
            && menu_ui.settings_tab == MainMenuSettingsTab::Sound
        {
            Display::Flex
        } else {
            Display::None
        },
    );
    {
        let mut settings_value_text = text_set.p2();
        let slider_value = slider_state
            .value(SETTINGS_MUSIC_VOLUME_SLIDER_ID)
            .map(|value| value.max(0) as u32)
            .unwrap_or(audio_settings.runtime_music_volume_percent);
        settings_value_text.0 = slider_value.to_string();
    }

    if matches!(screen, MainMenuScreen::Save | MainMenuScreen::Load) {
        if !menu_ui.needs_save_list_refresh {
            return;
        }
        menu_ui.needs_save_list_refresh = false;

        for entity in menu_ui.list_item_entities.drain(..) {
            commands.entity(entity).despawn();
        }
        save_list_entities.1.offset_y = 0.0;

        let saves = menu_ui.saves.clone();
        rebuild_main_menu_save_list(
            &mut commands,
            *save_list_entities.0,
            screen,
            &saves,
            &mut menu_ui.list_item_entities,
            &mut images,
        );
        return;
    }

    if screen != MainMenuScreen::Plugins {
        return;
    }
    if !menu_ui.needs_plugin_list_refresh {
        return;
    }
    menu_ui.needs_plugin_list_refresh = false;

    for entity in menu_ui.plugin_item_entities.drain(..) {
        commands.entity(entity).despawn();
    }
    plugin_list_entities.1.offset_y = 0.0;

    rebuild_main_menu_plugin_list(
        &mut commands,
        *plugin_list_entities.0,
        &plugin_registry_state,
        &enabled_plugins,
        mode,
        world_load_state.has_world,
        &mut menu_ui.plugin_item_entities,
    );
}
