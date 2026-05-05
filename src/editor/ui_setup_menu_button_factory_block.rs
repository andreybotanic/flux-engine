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

