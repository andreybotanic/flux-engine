fn refresh_main_menu_ui(
    mut commands: Commands,
    main_menu: Res<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    mut root_visibility: Single<&mut Visibility, With<MainMenuRoot>>,
    mut text_set: ParamSet<(
        Single<&mut Text, With<MainMenuTitleText>>,
        Single<&mut Text, With<MainMenuStatusText>>,
        Query<&mut Text>,
    )>,
    mut node_set: ParamSet<(
        Single<&mut Node, With<MainMenuRootActions>>,
        Single<&mut Node, With<MainMenuSaveActions>>,
        Single<&mut Node, With<MainMenuLoadActions>>,
        Single<&mut Node, With<MainMenuConfirmActions>>,
        Single<&mut Node, With<MainMenuSaveNameRow>>,
        Single<&mut Node, With<MainMenuSaveListRoot>>,
        Query<(&MainMenuActionButton, &mut Node), With<Button>>,
    )>,
    confirm_button_set: (
        Single<&Children, With<MainMenuConfirmPrimaryLabel>>,
        Single<&Children, With<MainMenuConfirmSecondaryLabel>>,
        Single<&Children, With<MainMenuConfirmCancelLabel>>,
    ),
    save_list_root: Single<Entity, With<MainMenuSaveListRoot>>,
) {
    **root_visibility = if main_menu.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if !main_menu.open {
        return;
    }

    let screen = menu_ui.screen;
    let mode = menu_ui.mode;
    {
        let mut title_text = text_set.p0();
        title_text.0 = match screen {
            MainMenuScreen::Root => match mode {
                MainMenuMode::Main => "Main Menu".to_string(),
                MainMenuMode::InGame => "Game Menu".to_string(),
                MainMenuMode::Hidden => "Menu".to_string(),
            },
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

    {
        let mut root_actions_visibility = node_set.p0();
        root_actions_visibility.display = if screen == MainMenuScreen::Root {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (action_button, mut node) in &mut node_set.p6() {
        node.display = match screen {
            MainMenuScreen::Root => match (&action_button.0, mode) {
                (_, MainMenuMode::Hidden) => Display::None,
                (MainMenuButtonAction::Continue, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::OpenSaveScreen, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::ExitToMainMenu, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::ExitApp, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::NewGame, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::OpenLoadScreen, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::ExitApp, MainMenuMode::Main) => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Save => match action_button.0 {
                MainMenuButtonAction::BackToRoot
                | MainMenuButtonAction::CreateNewSave
                | MainMenuButtonAction::SelectOverwrite(_) => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Load => match action_button.0 {
                MainMenuButtonAction::BackToRoot | MainMenuButtonAction::SelectLoad(_) => {
                    Display::Flex
                }
                _ => Display::None,
            },
            MainMenuScreen::Confirm => match action_button.0 {
                MainMenuButtonAction::ConfirmPrimary
                | MainMenuButtonAction::ConfirmSecondary
                | MainMenuButtonAction::ConfirmCancel => Display::Flex,
                _ => Display::None,
            },
        };
    }
    {
        let mut save_actions_visibility = node_set.p1();
        save_actions_visibility.display = if screen == MainMenuScreen::Save {
            Display::Flex
        } else {
            Display::None
        };
    }
    {
        let mut load_actions_visibility = node_set.p2();
        load_actions_visibility.display = if screen == MainMenuScreen::Load {
            Display::Flex
        } else {
            Display::None
        };
    }
    {
        let mut confirm_actions_visibility = node_set.p3();
        confirm_actions_visibility.display = if screen == MainMenuScreen::Confirm {
            Display::Flex
        } else {
            Display::None
        };
    }

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
                show_cancel = false;
            }
            None => {}
        }

        let primary_text_entity = confirm_button_set.0.iter().next();
        if let Some(entity) = primary_text_entity {
            if let Ok(mut text) = text_set.p2().get_mut(entity) {
                text.0 = primary_label.to_string();
            }
        }
        let secondary_text_entity = confirm_button_set.1.iter().next();
        if let Some(entity) = secondary_text_entity {
            if let Ok(mut text) = text_set.p2().get_mut(entity) {
                text.0 = secondary_label.to_string();
            }
        }
        let cancel_text_entity = confirm_button_set.2.iter().next();
        if let Some(entity) = cancel_text_entity {
            if let Ok(mut text) = text_set.p2().get_mut(entity) {
                text.0 = "Cancel".to_string();
            }
        }

        for (action_button, mut node) in &mut node_set.p6() {
            if matches!(action_button.0, MainMenuButtonAction::ConfirmCancel) {
                node.display = if show_cancel {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
    {
        let mut save_name_row_visibility = node_set.p4();
        save_name_row_visibility.display = if screen == MainMenuScreen::Save {
            Display::Flex
        } else {
            Display::None
        };
    }
    {
        let mut save_list_visibility = node_set.p5();
        save_list_visibility.display =
            if matches!(screen, MainMenuScreen::Save | MainMenuScreen::Load) {
                Display::Flex
            } else {
                Display::None
            };
    }

    if !matches!(screen, MainMenuScreen::Save | MainMenuScreen::Load) {
        return;
    }
    if !menu_ui.needs_save_list_refresh {
        return;
    }
    menu_ui.needs_save_list_refresh = false;

    for entity in menu_ui.list_item_entities.drain(..) {
        commands.entity(entity).despawn();
    }

    let saves = menu_ui.saves.clone();
    let mut created = Vec::new();
    let screen_for_buttons = screen;
    commands.entity(*save_list_root).with_children(|parent| {
        if saves.is_empty() {
            let row = parent
                .spawn((
                    Node {
                        width: Val::Px(730.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("No saves found."),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgba(0.15, 0.16, 0.18, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                })
                .id();
            created.push(row);
            return;
        }

        for descriptor in saves {
            let action = match screen_for_buttons {
                MainMenuScreen::Save => {
                    MainMenuButtonAction::SelectOverwrite(descriptor.id.clone())
                }
                MainMenuScreen::Load => MainMenuButtonAction::SelectLoad(descriptor.id.clone()),
                _ => continue,
            };

            let label = format!(
                "{}  |  id={}  |  updated={}",
                descriptor.display_name, descriptor.id, descriptor.updated_at_unix_ms
            );
            let entity = parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(730.0),
                        height: Val::Px(34.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    MainMenuActionButton(action),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(label),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                })
                .id();
            created.push(entity);
        }
    });
    menu_ui.list_item_entities = created;
}

