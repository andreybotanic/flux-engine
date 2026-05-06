/// Runs `setup_world_view` logic.
pub fn setup_world_view(
    mut commands: Commands,
    simulation_images: Res<GasSimulationImages>,
    world: Res<WorldGrid>,
    structures: Res<GasStructureGrid>,
    pipes: Res<PipeGrid>,
    world_load_state: Res<WorldLoadState>,
    overlay_mode: Res<OverlayMode>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pipe_highlight_materials:
        ResMut<Assets<crate::render::pipe_highlight_material::PipeHighlightMaterial>>,
    cell_visuals: Res<CellTypeVisualConfig>,
) {
    let show_world = world_load_state.has_world;
    let pipe_masks = (0u8..=0b1111)
        .map(|mask| asset_server.load(format!("sprites/world/pipe_mask_{mask:02}.png")))
        .collect::<Vec<_>>();
    let vent_world = asset_server.load("sprites/world/tile_vent.png");
    let vent_overlay = asset_server.load("sprites/world/gas_in_out.png");
    let pipe_highlight = crate::render::pipe_highlight_material::PipeHighlightRenderAssets {
        quad: meshes.add(bevy::math::primitives::Rectangle::new(CELL_SIZE, CELL_SIZE)),
        materials: pipe_masks
            .iter()
            .cloned()
            .map(|mask| {
                pipe_highlight_materials.add(
                    crate::render::pipe_highlight_material::PipeHighlightMaterial::from_pipe_mask(mask),
                )
            })
            .collect(),
    };
    let visuals = WorldVisualAssets {
        backdrop_noise: asset_server.load("sprites/world/backdrop_noise.png"),
        brick: asset_server.load("sprites/world/tile_brick.png"),
        metal: asset_server.load("sprites/world/tile_metal.png"),
        boundary: asset_server.load("sprites/world/tile_boundary.png"),
        source: asset_server.load("sprites/world/tile_gas_source.png"),
        sink: asset_server.load("sprites/world/tile_gas_sink.png"),
        pipe_masks,
        vent_world,
        vent_overlay,
        pipe_highlight,
    };
    commands.insert_resource(visuals.clone());

    let world_size = world_dimensions();
    let fade_sprite_size = world_size + Vec2::splat(CELL_SIZE * WORLD_FADE_WIDTH_CELLS * 2.0);
    let world_fade_mask = images.add(build_world_fade_mask_image(world_size));

    commands.spawn((
        Sprite {
            image: visuals.backdrop_noise.clone(),
            custom_size: Some(world_size),
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
            image: world_fade_mask,
            custom_size: Some(fade_sprite_size),
            color: Color::WHITE,
            ..default()
        },
        // Keep fade mask above world sprites so edge darkening is smooth and continuous.
        Transform::from_xyz(0.0, 0.0, 1.2),
        if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        WorldFadeMaskLayer,
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
    let boundary_main_tint = cell_visuals.main_tint(CellMaterial::Boundary);
    let boundary_gas_tint = cell_visuals.gas_tint(CellMaterial::Boundary);
    spawn_outer_border_layers(
        &mut commands,
        &visuals,
        boundary_main_tint,
        boundary_gas_tint,
        show_world,
    );

    let mut structure_entities = GasStructureEntities::default();
    for (x, y, structure) in structures.iter_cells() {
        let entity = spawn_gas_structure_sprite(&mut commands, &visuals, x, y, structure, show_world);
        structure_entities.by_cell.insert((x, y), entity);
    }
    commands.insert_resource(structure_entities);

    let mut pipe_entities = PipeEntities::default();
    for (x, y, pipe_cell) in pipes.iter_cells() {
        spawn_pipe_visual_bundle(
            &mut commands,
            &visuals,
            &mut pipe_entities,
            x,
            y,
            pipe_cell,
            show_world,
            *overlay_mode,
        );
    }
    commands.insert_resource(pipe_entities);
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

fn spawn_outer_border_layers(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    boundary_main_tint: Color,
    boundary_gas_tint: Color,
    show_world: bool,
) {
    let min_x = 0_i32;
    let min_y = 0_i32;
    let max_x = WORLD_WIDTH as i32 - 1;
    let max_y = WORLD_HEIGHT as i32 - 1;

    for layer in 1..=OUTER_BORDER_LAYERS {
        let ring_min_x = min_x - layer as i32;
        let ring_min_y = min_y - layer as i32;
        let ring_max_x = max_x + layer as i32;
        let ring_max_y = max_y + layer as i32;

        for y in ring_min_y..=ring_max_y {
            for x in ring_min_x..=ring_max_x {
                let is_in_previous_ring = x >= ring_min_x + 1
                    && x <= ring_max_x - 1
                    && y >= ring_min_y + 1
                    && y <= ring_max_y - 1;
                if is_in_previous_ring {
                    continue;
                }
                commands.spawn((
                    Sprite {
                        image: visuals.boundary.clone(),
                        custom_size: Some(Vec2::splat(CELL_SIZE)),
                        color: boundary_main_tint,
                        ..default()
                    },
                    Transform::from_translation(outer_cell_center(x, y).extend(0.2)),
                    if show_world {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    OuterBorderVisual {
                        main_tint: boundary_main_tint,
                        gas_tint: boundary_gas_tint,
                    },
                ));
            }
        }
    }
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

fn spawn_pipe_visual_bundle(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    entities: &mut PipeEntities,
    x: u32,
    y: u32,
    pipe_cell: PipeCell,
    show_world: bool,
    overlay_mode: OverlayMode,
) {
    let center = cell_center(x, y);
    let pipe_visual = PipeWorldVisual {
        main_tint: Color::srgba(0.42, 0.50, 0.56, 0.96),
        gas_tint: Color::srgba(0.74, 0.82, 0.88, 0.96),
        pipe_tint: Color::srgba(0.96, 0.985, 1.0, 1.0),
    };
    if pipe_cell.has_pipe {
        let mask = (pipe_cell.connections & 0b1111) as usize;
        let pipe_tint = pipe_sprite_tint(overlay_mode, &pipe_visual);
        let entity = commands
            .spawn((
                Sprite {
                    image: visuals.pipe_masks[mask].clone(),
                    custom_size: Some(Vec2::splat(CELL_SIZE)),
                    color: pipe_tint,
                    ..default()
                },
                Transform::from_translation(center.extend(pipe_world_z(overlay_mode))),
                world_layer_visibility(show_world),
                pipe_visual,
            ))
            .id();
        entities.pipes.insert((x, y), entity);

        let highlight_entity = crate::render::pipe_highlight_material::spawn_pipe_highlight_entity(
            commands,
            &visuals.pipe_highlight,
            mask,
            Transform::from_translation(center.extend(pipe_highlight_z())),
            pipe_highlight_visibility(show_world, overlay_mode),
        );
        commands.entity(highlight_entity).insert(PipeHighlightOverlayVisual);
        entities.pipe_highlights.insert((x, y), highlight_entity);

        let gas_overlay = commands
            .spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(CELL_SIZE * 0.7)),
                Transform::from_translation(center.extend(1.05)),
                Visibility::Hidden,
                PipeGasOverlayVisual,
            ))
            .id();
        entities.gas_overlays.insert((x, y), gas_overlay);

        let gas_overlay_border = commands
            .spawn((
                Sprite::from_color(Color::WHITE, Vec2::splat(CELL_SIZE * 0.7)),
                Transform::from_translation(center.extend(1.04)),
                Visibility::Hidden,
                PipeGasOverlayBorderVisual,
            ))
            .id();
        entities
            .gas_overlay_borders
            .insert((x, y), gas_overlay_border);
    }

    if pipe_cell.has_vent {
        let world_entity = commands
            .spawn((
                Sprite {
                    image: visuals.vent_world.clone(),
                    custom_size: Some(Vec2::splat(CELL_SIZE)),
                    color: Color::WHITE,
                    ..default()
                },
                Transform::from_translation(center.extend(0.95)),
                vent_world_visibility(show_world, overlay_mode),
                VentWorldVisual,
            ))
            .id();
        entities.vents.insert((x, y), world_entity);

        let overlay_entity = commands
            .spawn((
                Sprite {
                    image: visuals.vent_overlay.clone(),
                    custom_size: Some(Vec2::splat(CELL_SIZE * 0.92)),
                    color: pipe_overlay_vent_tint(),
                    ..default()
                },
                Transform::from_translation(center.extend(1.15)),
                Visibility::Hidden,
                PipeVentOverlayVisual,
            ))
            .id();
        entities.vent_overlays.insert((x, y), overlay_entity);
    }
}

fn world_layer_visibility(show_world: bool) -> Visibility {
    if show_world {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn pipe_sprite_tint(overlay_mode: OverlayMode, pipe_visual: &PipeWorldVisual) -> Color {
    match overlay_mode {
        OverlayMode::Main => pipe_visual.main_tint,
        OverlayMode::Gas => pipe_visual.gas_tint,
        OverlayMode::Pipes => pipe_visual.pipe_tint,
    }
}

fn pipe_highlight_visibility(show_world: bool, overlay_mode: OverlayMode) -> Visibility {
    if show_world && overlay_mode == OverlayMode::Pipes {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn pipe_highlight_z() -> f32 {
    0.97
}

fn pipe_world_z(overlay_mode: OverlayMode) -> f32 {
    match overlay_mode {
        OverlayMode::Pipes => 0.92,
        OverlayMode::Main | OverlayMode::Gas => 0.45,
    }
}

fn pipe_overlay_vent_tint() -> Color {
    Color::srgba(1.0, 0.88, 0.28, 1.0)
}

fn vent_world_visibility(show_world: bool, overlay_mode: OverlayMode) -> Visibility {
    let _ = overlay_mode;
    if show_world {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

pub(crate) fn sync_wall_visuals(
    mut commands: Commands,
    world: Res<WorldGrid>,
    world_load_state: Res<WorldLoadState>,
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
                let entity = spawn_wall_sprite(
                    &mut commands,
                    &visuals,
                    &cell_visuals,
                    x,
                    y,
                    material,
                    world_load_state.has_world,
                );
                wall_entities.by_cell.insert(key, entity);
            }
            (CellKind::Empty, Some(entity)) => {
                commands.entity(entity).despawn();
                wall_entities.by_cell.remove(&key);
            }
            (CellKind::Solid(material), Some(entity)) => {
                commands.entity(entity).despawn();
                let next_entity = spawn_wall_sprite(
                    &mut commands,
                    &visuals,
                    &cell_visuals,
                    x,
                    y,
                    material,
                    world_load_state.has_world,
                );
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

pub(crate) fn sync_pipe_world_visuals(
    mut commands: Commands,
    pipes: Res<PipeGrid>,
    world_load_state: Res<WorldLoadState>,
    overlay_mode: Res<OverlayMode>,
    visuals: Res<WorldVisualAssets>,
    mut pipe_entities: ResMut<PipeEntities>,
) {
    if !pipes.is_changed() && !world_load_state.is_changed() && !overlay_mode.is_changed() {
        return;
    }

    for entity in pipe_entities
        .pipes
        .values()
        .chain(pipe_entities.pipe_highlights.values())
        .chain(pipe_entities.vents.values())
        .chain(pipe_entities.gas_overlays.values())
        .chain(pipe_entities.gas_overlay_borders.values())
        .chain(pipe_entities.vent_overlays.values())
        .copied()
        .collect::<Vec<_>>()
    {
        commands.entity(entity).despawn();
    }
    pipe_entities.pipes.clear();
    pipe_entities.pipe_highlights.clear();
    pipe_entities.vents.clear();
    pipe_entities.gas_overlays.clear();
    pipe_entities.gas_overlay_borders.clear();
    pipe_entities.vent_overlays.clear();

    if !world_load_state.has_world {
        return;
    }

    for (x, y, pipe_cell) in pipes.iter_cells() {
        spawn_pipe_visual_bundle(
            &mut commands,
            &visuals,
            &mut pipe_entities,
            x,
            y,
            pipe_cell,
            world_load_state.has_world,
            *overlay_mode,
        );
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

fn outer_cell_center(x: i32, y: i32) -> Vec2 {
    crate::world::grid::world_origin()
        + Vec2::new((x as f32 + 0.5) * CELL_SIZE, (y as f32 + 0.5) * CELL_SIZE)
}

#[cfg(test)]
fn build_pipe_mask_image(mask: u8) -> Image {
    let size = 64u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let thickness = 14i32;
    let half = size as i32 / 2;
    let hub_radius = 10i32;

    let draw_rect = |data: &mut [u8], min_x: i32, min_y: i32, max_x: i32, max_y: i32| {
        for y in min_y.max(0)..max_y.min(size as i32) {
            for x in min_x.max(0)..max_x.min(size as i32) {
                let idx = ((y as u32 * size + x as u32) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    };
    let draw_circle = |data: &mut [u8], cx: i32, cy: i32, radius: i32| {
        let r2 = radius * radius;
        for y in (cy - radius).max(0)..=(cy + radius).min(size as i32 - 1) {
            for x in (cx - radius).max(0)..=(cx + radius).min(size as i32 - 1) {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let idx = ((y as u32 * size + x as u32) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    };

    let arm_half = thickness / 2;

    if mask == 0 {
        draw_circle(&mut data, half, half, hub_radius);
    } else {
        if (mask & 0b0001) != 0 {
            draw_rect(
                &mut data,
                half - arm_half,
                half,
                half + arm_half,
                size as i32,
            );
        }
        if (mask & 0b0010) != 0 {
            draw_rect(
                &mut data,
                half,
                half - arm_half,
                size as i32,
                half + arm_half,
            );
        }
        if (mask & 0b0100) != 0 {
            draw_rect(&mut data, half - arm_half, 0, half + arm_half, half);
        }
        if (mask & 0b1000) != 0 {
            draw_rect(&mut data, 0, half - arm_half, half, half + arm_half);
        }

        draw_circle(&mut data, half, half, hub_radius);
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
fn build_vent_overlay_image() -> Image {
    build_vent_image(true)
}

#[cfg(test)]
fn build_vent_image(with_arrows: bool) -> Image {
    let size = 64u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let draw_rect = |data: &mut [u8], min_x: i32, min_y: i32, max_x: i32, max_y: i32| {
        for y in min_y.max(0)..max_y.min(size as i32) {
            for x in min_x.max(0)..max_x.min(size as i32) {
                let idx = ((y as u32 * size + x as u32) * 4) as usize;
                data[idx] = 255;
                data[idx + 1] = 255;
                data[idx + 2] = 255;
                data[idx + 3] = 255;
            }
        }
    };
    let draw_line = |data: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32| {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).max(1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = x0 as f32 + dx as f32 * t;
            let y = y0 as f32 + dy as f32 * t;
            let half = thickness / 2;
            draw_rect(
                data,
                x.round() as i32 - half,
                y.round() as i32 - half,
                x.round() as i32 + half + 1,
                y.round() as i32 + half + 1,
            );
        }
    };
    let draw_arrow_head =
        |data: &mut [u8], tip_x: i32, tip_y: i32, dir_x: i32, dir_y: i32, size_px: i32, thickness: i32| {
            let (left_x, left_y, right_x, right_y) = match (dir_x, dir_y) {
                (0, -1) => (
                    tip_x - size_px,
                    tip_y + size_px,
                    tip_x + size_px,
                    tip_y + size_px,
                ),
                (0, 1) => (
                    tip_x - size_px,
                    tip_y - size_px,
                    tip_x + size_px,
                    tip_y - size_px,
                ),
                _ => (tip_x, tip_y, tip_x, tip_y),
            };
            draw_line(data, tip_x, tip_y, left_x, left_y, thickness);
            draw_line(data, tip_x, tip_y, right_x, right_y, thickness);
        };

    if with_arrows {
        let border = 4;
        let left = 12;
        let top = 18;
        let right = 52;
        let bottom = 58;
        draw_rect(&mut data, left, top, right, top + border);
        draw_rect(&mut data, left, bottom - border, right, bottom);
        draw_rect(&mut data, left, top, left + border, bottom);
        draw_rect(&mut data, right - border, top, right, bottom);

        let shaft_x = 32;
        let shaft_thickness = 4;
        draw_rect(&mut data, shaft_x - 2, 8, shaft_x + 2, 42);
        draw_arrow_head(&mut data, shaft_x, 4, 0, -1, 7, shaft_thickness);
        draw_arrow_head(&mut data, shaft_x, 46, 0, 1, 7, shaft_thickness);
    } else {
        draw_rect(&mut data, 10, 16, 54, 48);
        for offset in [18, 26, 34, 42] {
            draw_rect(&mut data, offset, 18, offset + 3, 46);
        }
        for offset in [22, 32, 42] {
            draw_rect(&mut data, 12, offset, 52, offset + 2);
        }
    }

    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

