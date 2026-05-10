use std::path::Path;

use bevy::{render::render_asset::RenderAssetUsages, ui::RelativeCursorPosition};

fn normalized_cursor_is_inside(normalized: Option<Vec2>) -> bool {
    let Some(cursor) = normalized else {
        return false;
    };
    (0.0..=1.0).contains(&cursor.x) && (0.0..=1.0).contains(&cursor.y)
}

fn emit_main_menu_button_actions(
    menu_ui: Res<MainMenuUiState>,
    world_load_state: Res<WorldLoadState>,
    mut interactions: Query<
        (
            &Interaction,
            &MainMenuActionButton,
            &mut BackgroundColor,
            Option<&MainMenuButtonPalette>,
            Option<&crate::ui::toggle_switch::ToggleSwitchRoot>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut action_requests: EventWriter<MainMenuActionRequest>,
) {
    for (interaction, action_button, mut background, palette, toggle_root) in &mut interactions {
        let idle = palette.map(|value| value.idle).unwrap_or(MODAL_BUTTON_BG);
        let hover = palette.map(|value| value.hover).unwrap_or(MODAL_BUTTON_HOVER);
        let active = match (&action_button.0, toggle_root) {
            (MainMenuButtonAction::TogglePlugin(_), Some(root)) => root.interactive,
            (MainMenuButtonAction::TogglePlugin(_), None) => false,
            (MainMenuButtonAction::ReloadPlugins, _) => {
                plugin_reload_allowed(menu_ui.mode, world_load_state.has_world)
            }
            _ => true,
        };
        background.0 = if active {
            match *interaction {
                Interaction::Pressed | Interaction::Hovered => hover,
                Interaction::None => idle,
            }
        } else {
            idle
        };
        if active && *interaction == Interaction::Pressed {
            action_requests.write(MainMenuActionRequest(action_button.0.clone()));
        }
    }
}

fn update_main_menu_save_card_interactions(
    buttons: Res<ButtonInput<MouseButton>>,
    main_menu: Res<MainMenuState>,
    menu_ui: Res<MainMenuUiState>,
    preview_queue: Res<SavePreviewQueueState>,
    mut cards: Query<
        (
            &RelativeCursorPosition,
            &Node,
            &mut BackgroundColor,
            &MainMenuSaveCard,
        ),
    >,
    delete_buttons: Query<(&RelativeCursorPosition, &Node), With<MainMenuSaveCardDeleteButton>>,
    mut action_requests: EventWriter<MainMenuActionRequest>,
) {
    let cards_are_active = main_menu.open
        && matches!(menu_ui.screen, MainMenuScreen::Save | MainMenuScreen::Load)
        && !preview_queue.is_busy();
    let mut hovered_action = None;

    for (relative_cursor, node, mut background, card) in &mut cards {
        let hovered = node.display != Display::None
            && cards_are_active
            && normalized_cursor_is_inside(relative_cursor.normalized);
        background.0 = if hovered {
            MENU_MODAL_CARD_HOVER
        } else {
            MENU_MODAL_CARD_BG
        };
        if hovered {
            hovered_action = Some(card.primary_action.clone());
        }
    }

    if !cards_are_active || !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    for (relative_cursor, node) in &delete_buttons {
        if node.display != Display::None && normalized_cursor_is_inside(relative_cursor.normalized) {
            return;
        }
    }

    if let Some(action) = hovered_action {
        action_requests.write(MainMenuActionRequest(action));
    }
}

fn save_card_primary_action(screen: MainMenuScreen, save_id: &str) -> Option<MainMenuButtonAction> {
    match screen {
        MainMenuScreen::Save => Some(MainMenuButtonAction::SelectOverwrite(save_id.to_string())),
        MainMenuScreen::Load => Some(MainMenuButtonAction::SelectLoad(save_id.to_string())),
        _ => None,
    }
}

fn rebuild_main_menu_save_list(
    commands: &mut Commands,
    content_entity: Entity,
    screen: MainMenuScreen,
    saves: &[crate::save::SaveDescriptor],
    list_item_entities: &mut Vec<Entity>,
    images: &mut Assets<Image>,
) {
    let mut created = Vec::new();
    let saves = saves.to_vec();
    commands.entity(content_entity).with_children(|parent| {
        if saves.is_empty() {
            let row = parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(160.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(crate::ui::palette::TRANSPARENT),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("No saves found."),
                        TextFont::from_font_size(14.0),
                        TextColor(crate::ui::palette::TEXT_MUTED),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                })
                .id();
            created.push(row);
            return;
        }

        for descriptor in saves {
            let Some(primary_action) = save_card_primary_action(screen, &descriptor.id) else {
                continue;
            };
            let updated_text = format_save_datetime(descriptor.updated_at_unix_ms);
            let preview_handle =
                descriptor.preview_path.as_ref().and_then(|path| load_preview_ui_image(path, images));

            let card = parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(186.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        display: Display::Flex,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Stretch,
                        column_gap: Val::Px(16.0),
                        ..default()
                    },
                    BackgroundColor(MENU_MODAL_CARD_BG),
                    RelativeCursorPosition::default(),
                    MainMenuSaveCard { primary_action },
                ))
                .with_children(|row| {
                    row.spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        flex_grow: 1.0,
                        min_height: Val::Px(160.0),
                        row_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|left| {
                        left.spawn(Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|meta| {
                            meta.spawn((
                                Text::new(descriptor.display_name.clone()),
                                TextFont::from_font_size(18.0),
                                TextColor(crate::ui::palette::TEXT_PRIMARY),
                            ));
                            meta.spawn((
                                Text::new(format!("Updated: {updated_text}")),
                                TextFont::from_font_size(13.0),
                                TextColor(crate::ui::palette::TEXT_MUTED),
                            ));
                        });

                        left.spawn((
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(32.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(MODAL_BUTTON_BG),
                            MainMenuButtonPalette {
                                idle: MODAL_BUTTON_BG,
                                hover: MODAL_BUTTON_HOVER,
                            },
                            RelativeCursorPosition::default(),
                            MainMenuSaveCardDeleteButton,
                            MainMenuActionButton(MainMenuButtonAction::SelectDelete(
                                descriptor.id.clone(),
                            )),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Delete"),
                                TextFont::from_font_size(13.0),
                                TextColor(crate::ui::palette::TEXT_ON_DARK),
                            ));
                        });
                    });

                    row.spawn((
                        Node {
                            width: Val::Px(160.0),
                            min_width: Val::Px(160.0),
                            height: Val::Px(160.0),
                            min_height: Val::Px(160.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(MENU_MODAL_PREVIEW_BG),
                    ))
                    .with_children(|preview| {
                        if let Some(handle) = preview_handle {
                            preview.spawn((
                                ImageNode::new(handle),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                            ));
                        } else {
                            preview.spawn((
                                Text::new("Preview"),
                                TextFont::from_font_size(18.0),
                                TextColor(crate::ui::palette::TEXT_MUTED),
                            ));
                        }
                    });
                })
                .id();
            created.push(card);
        }
    });

    *list_item_entities = created;
}

fn load_preview_ui_image(path: &Path, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let decoded = image::open(path).ok()?;
    Some(images.add(Image::from_dynamic(
        decoded,
        true,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )))
}

fn format_save_datetime(unix_ms: i64) -> String {
    use time::{OffsetDateTime, UtcOffset};

    let ns = (unix_ms as i128).saturating_mul(1_000_000);
    let Ok(datetime_utc) = OffsetDateTime::from_unix_timestamp_nanos(ns) else {
        return format!("unix_ms={unix_ms}");
    };
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let local = datetime_utc.to_offset(offset);
    let month: u8 = local.month().into();
    format!(
        "{:02}.{:02}.{:04} {:02}:{:02}",
        local.day(),
        month,
        local.year(),
        local.hour(),
        local.minute()
    )
}

#[cfg(test)]
mod main_menu_save_list_tests {
    use super::save_card_primary_action;
    use super::MainMenuButtonAction;
    use crate::save::MainMenuScreen;

    #[test]
    fn save_card_primary_action_matches_current_screen() {
        assert_eq!(
            save_card_primary_action(MainMenuScreen::Save, "slot-1"),
            Some(MainMenuButtonAction::SelectOverwrite("slot-1".to_string()))
        );
        assert_eq!(
            save_card_primary_action(MainMenuScreen::Load, "slot-1"),
            Some(MainMenuButtonAction::SelectLoad("slot-1".to_string()))
        );
        assert_eq!(save_card_primary_action(MainMenuScreen::Root, "slot-1"), None);
    }
}
