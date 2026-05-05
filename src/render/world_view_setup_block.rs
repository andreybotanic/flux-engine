/// Runs `setup_world_view` logic.
pub fn setup_world_view(
    mut commands: Commands,
    simulation_images: Res<GasSimulationImages>,
    world: Res<WorldGrid>,
    structures: Res<GasStructureGrid>,
    world_load_state: Res<WorldLoadState>,
    asset_server: Res<AssetServer>,
    cell_visuals: Res<CellTypeVisualConfig>,
) {
    let show_world = world_load_state.has_world;
    let visuals = WorldVisualAssets {
        backdrop_noise: asset_server.load("sprites/world/backdrop_noise.png"),
        brick: asset_server.load("sprites/world/tile_brick.png"),
        metal: asset_server.load("sprites/world/tile_metal.png"),
        boundary: asset_server.load("sprites/world/tile_boundary.png"),
        source: asset_server.load("sprites/world/tile_gas_source.png"),
        sink: asset_server.load("sprites/world/tile_gas_sink.png"),
    };
    commands.insert_resource(visuals.clone());

    let world_size = world_dimensions();

    commands.spawn((
        Sprite {
            image: visuals.backdrop_noise.clone(),
            custom_size: Some(world_size + Vec2::splat(CELL_SIZE * 6.0)),
            color: BACKDROP_MAIN_COLOR,
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: BACKDROP_TILE_SIZE,
            },
            ..default()
        },
        Transform::from_xyz(14.0, -18.0, -2.0),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        BackdropLayer,
    ));

    commands.spawn((
        Sprite {
            image: visuals.backdrop_noise.clone(),
            custom_size: Some(world_size),
            color: BOARD_MAIN_COLOR,
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: BACKDROP_TILE_SIZE,
            },
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        BoardLayer,
    ));

    commands.spawn((
        Sprite {
            image: simulation_images.texture_f2_a.clone(),
            custom_size: Some(world_size),
            color: Color::srgba(1.0, 0.25, 0.1, 0.88),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
        Visibility::Hidden,
        GasOverlaySprite,
    ));

    commands.spawn((
        Sprite {
            image: simulation_images.texture_f1_a.clone(),
            custom_size: Some(world_size),
            color: Color::WHITE,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.8),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        GasMainOverlaySprite,
    ));

    let mut wall_entities = WallEntities::default();
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            if let CellKind::Solid(material) = world.cell(x, y) {
                let entity = spawn_wall_sprite(
                    &mut commands,
                    &visuals,
                    &cell_visuals,
                    x,
                    y,
                    material,
                    show_world,
                );
                wall_entities.by_cell.insert((x, y), entity);
            }
        }
    }
    commands.insert_resource(wall_entities);

    let mut structure_entities = GasStructureEntities::default();
    for (x, y, structure) in structures.iter_cells() {
        let entity = spawn_gas_structure_sprite(&mut commands, &visuals, x, y, structure, show_world);
        structure_entities.by_cell.insert((x, y), entity);
    }
    commands.insert_resource(structure_entities);
    commands.spawn((
        Sprite::from_color(
            Color::srgba(1.0, 0.93, 0.30, 0.36),
            Vec2::splat(CELL_SIZE - 2.0),
        ),
        Transform::from_xyz(0.0, 0.0, 0.91),
        Visibility::Hidden,
        GasStructureEditHighlight,
    ));
}

fn spawn_wall_sprite(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    cell_visuals: &CellTypeVisualConfig,
    x: u32,
    y: u32,
    material: CellMaterial,
    show_world: bool,
) -> Entity {
    let image = match material {
        CellMaterial::Boundary => visuals.boundary.clone(),
        CellMaterial::Brick => visuals.brick.clone(),
        CellMaterial::Metal => visuals.metal.clone(),
    };
    let main_tint = cell_visuals.main_tint(material);
    let gas_tint = cell_visuals.gas_tint(material);

    commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(Vec2::splat(CELL_SIZE)),
                color: main_tint,
                ..default()
            },
            Transform::from_translation(cell_center(x, y).extend(0.5)),
            if show_world {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            WallVisual {
                main_tint,
                gas_tint,
            },
        ))
        .id()
}

fn spawn_gas_structure_sprite(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    x: u32,
    y: u32,
    structure: GasStructureCell,
    show_world: bool,
) -> Entity {
    let image = match structure {
        GasStructureCell::Source { .. } => visuals.source.clone(),
        GasStructureCell::Sink { .. } => visuals.sink.clone(),
    };
    commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(Vec2::splat(CELL_SIZE)),
                color: Color::WHITE,
                ..default()
            },
            Transform::from_translation(cell_center(x, y).extend(0.9)),
            if show_world {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            GasStructureVisual,
        ))
        .id()
}

pub(crate) fn sync_wall_visuals(
    mut commands: Commands,
    world: Res<WorldGrid>,
    visuals: Res<WorldVisualAssets>,
    cell_visuals: Res<CellTypeVisualConfig>,
    mut wall_entities: ResMut<WallEntities>,
    mut changes: EventReader<WorldCellChanged>,
) {
    for change in changes.read() {
        let x = change.cell.x;
        let y = change.cell.y;
        let key = (x, y);
        let current_cell = world.cell(x, y);

        match (current_cell, wall_entities.by_cell.get(&key).copied()) {
            (CellKind::Solid(material), None) => {
                let entity =
                    spawn_wall_sprite(&mut commands, &visuals, &cell_visuals, x, y, material, true);
                wall_entities.by_cell.insert(key, entity);
            }
            (CellKind::Empty, Some(entity)) => {
                commands.entity(entity).despawn();
                wall_entities.by_cell.remove(&key);
            }
            (CellKind::Solid(material), Some(entity)) => {
                commands.entity(entity).despawn();
                let next_entity =
                    spawn_wall_sprite(&mut commands, &visuals, &cell_visuals, x, y, material, true);
                wall_entities.by_cell.insert(key, next_entity);
            }
            _ => {}
        }
    }
}

pub(crate) fn sync_gas_structure_visuals(
    mut commands: Commands,
    structures: Res<GasStructureGrid>,
    world_load_state: Res<WorldLoadState>,
    visuals: Res<WorldVisualAssets>,
    mut structure_entities: ResMut<GasStructureEntities>,
) {
    if !structures.is_changed() && !world_load_state.is_changed() {
        return;
    }

    for entity in structure_entities.by_cell.values().copied() {
        commands.entity(entity).despawn();
    }
    structure_entities.by_cell.clear();

    if !world_load_state.has_world {
        return;
    }

    for (x, y, structure) in structures.iter_cells() {
        let entity =
            spawn_gas_structure_sprite(&mut commands, &visuals, x, y, structure, world_load_state.has_world);
        structure_entities.by_cell.insert((x, y), entity);
    }
}

pub(crate) fn sync_structure_edit_highlight(
    world_load_state: Res<WorldLoadState>,
    structure_edit: Res<StructureEditState>,
    structures: Res<GasStructureGrid>,
    mut highlight: Single<(&mut Transform, &mut Visibility), With<GasStructureEditHighlight>>,
) {
    let (transform, visibility) = &mut *highlight;
    if !world_load_state.has_world {
        **visibility = Visibility::Hidden;
        return;
    }
    let Some(cell) = structure_edit.selected_cell else {
        **visibility = Visibility::Hidden;
        return;
    };
    if structures.cell(cell.x, cell.y).is_none() {
        **visibility = Visibility::Hidden;
        return;
    }
    **transform = Transform::from_translation(cell_center(cell.x, cell.y).extend(0.91));
    **visibility = Visibility::Visible;
}

