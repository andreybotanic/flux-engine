fn setup_editor_overlays(mut commands: Commands, asset_server: Res<AssetServer>) {
    let brick_silhouette = asset_server.load(
        crate::plugins::default_plugin::cell_silhouette_path(CellMaterial::Brick)
            .expect("brick silhouette is registered"),
    );
    commands.spawn((
        Sprite {
            image: brick_silhouette,
            custom_size: Some(Vec2::splat(CELL_SIZE)),
            color: crate::ui::palette::TEXT_ON_DARK,
            ..default()
        },
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

