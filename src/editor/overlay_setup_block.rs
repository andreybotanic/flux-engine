fn setup_editor_overlays(mut commands: Commands, asset_server: Res<AssetServer>) {
    let brick_silhouette = asset_server.load("sprites/ui/silhouette_brick.png");
    commands.spawn((
        Sprite {
            image: brick_silhouette,
            custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
            color: Color::WHITE,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.8),
        Visibility::Hidden,
        BlueprintGhost,
    ));

    commands.spawn((
        Sprite::from_color(
            Color::srgba(1.0, 0.24, 0.24, 0.28),
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
                TextColor(Color::srgba(1.0, 0.25, 0.25, 0.95)),
            ));
        });
}

