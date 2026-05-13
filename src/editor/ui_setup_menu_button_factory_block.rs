fn spawn_main_menu_action_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: MainMenuButtonAction,
    width: f32,
) -> Entity {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(width),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(MODAL_BUTTON_BG),
            MainMenuActionButton(action),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont::from_font_size(15.0),
                TextColor(crate::ui::palette::TEXT_ON_DARK),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        })
        .id()
}

fn spawn_main_menu_settings_tab_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: MainMenuButtonAction,
    width: f32,
) -> Entity {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(width),
                height: Val::Px(42.0),
                border: UiRect::all(Val::Px(0.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderRadius {
                top_left: Val::Px(10.0),
                top_right: Val::Px(10.0),
                bottom_left: Val::Px(0.0),
                bottom_right: Val::Px(0.0),
            },
            BackgroundColor(MENU_MODAL_CARD_BG),
            MainMenuButtonPalette {
                idle: MENU_MODAL_CARD_BG,
                hover: MENU_MODAL_INPUT_BG,
            },
            MainMenuSettingsTabButton,
            MainMenuActionButton(action),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont::from_font_size(15.0),
                TextColor(crate::ui::palette::TEXT_ON_DARK),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        })
        .id()
}
