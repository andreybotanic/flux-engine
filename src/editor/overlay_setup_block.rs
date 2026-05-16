fn setup_editor_overlays(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::NONE, Vec2::splat(CELL_SIZE)),
        Transform::from_xyz(0.0, 0.0, 1.8),
        Visibility::Hidden,
        BlueprintGhost,
    ));

    commands.spawn((
        Sprite::from_color(
            crate::ui::palette::DANGER_ACCENT,
            Vec2::splat(CELL_SIZE - 2.0),
        ),
        Transform::from_xyz(0.0, 0.0, 1.79),
        Visibility::Hidden,
        EraseCellHighlight,
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            Visibility::Hidden,
            EraseCursorOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("X"),
                TextFont::from_font_size(24.0),
                TextColor(crate::ui::palette::DANGER_TEXT),
                EraseCursorOverlayText,
            ));
        });
}

