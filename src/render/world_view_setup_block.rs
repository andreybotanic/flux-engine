/// Runs `setup_world_view` logic.
pub fn setup_world_view(
    mut commands: Commands,
    simulation_images: Res<GasSimulationImages>,
    world: Res<WorldGrid>,
    structures: Res<PlacedStructureMap>,
    content_registry: Res<ContentRegistry>,
    world_load_state: Res<WorldLoadState>,
    overlay_mode: Res<OverlayMode>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pipe_highlight_materials:
        ResMut<Assets<crate::render::pipe_highlight_material::PipeHighlightMaterial>>,
    cell_visuals: Res<CellTypeVisualConfig>,
    cell_visual_layouts: Res<crate::config::CellVisualPlacementConfigMap>,
    structure_visuals: Res<crate::config::StructureVisualConfigMap>,
) {
    let boundary_material = content_registry
        .cell_by_tag(crate::plugins::content::CORE_TAG_CELL_BOUNDARY)
        .map(|descriptor| descriptor.material)
        .or_else(|| boundary_material_from_world(&world))
        .or_else(|| content_registry.cells().values().next().map(|descriptor| descriptor.material))
        .expect("world rendering requires at least one registered cell material");
    let boundary_sprite_path = wall_sprite_path_for_material(&content_registry, boundary_material);
    let vent_world_path = structure_sprite_path_for_tag(
        &content_registry,
        crate::plugins::content::CORE_TAG_STRUCTURE_VENT,
    )
    .unwrap_or_else(|| "sprites/ui/tool_gases.ktx2".to_string());
    let bridge_world_path = structure_sprite_path_for_tag(
        &content_registry,
        crate::plugins::content::CORE_TAG_STRUCTURE_PIPE_BRIDGE,
    )
    .unwrap_or_else(|| "sprites/ui/tool_gases.ktx2".to_string());
    let show_world = world_load_state.has_world;
    let pipe_masks = (0u8..=0b1111)
        .map(|mask| asset_server.load(crate::plugins::default_plugin::pipe_mask_sprite_path(mask)))
        .collect::<Vec<_>>();
    let vent_world = asset_server.load(vent_world_path);
    let port_overlay_bidir =
        asset_server.load(crate::plugins::default_plugin::pipe_connection_overlay_sprite_path());
    let port_overlay_in =
        asset_server.load(crate::plugins::default_plugin::pipe_connection_in_overlay_sprite_path());
    let port_overlay_out = asset_server.load(
        crate::plugins::default_plugin::pipe_connection_out_overlay_sprite_path(),
    );
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
        vent_material: pipe_highlight_materials.add(
            crate::render::pipe_highlight_material::PipeHighlightMaterial::from_pipe_mask(
                vent_world.clone(),
            ),
        ),
        bridge_material: pipe_highlight_materials.add(
            crate::render::pipe_highlight_material::PipeHighlightMaterial::from_pipe_mask(
                asset_server.load(bridge_world_path),
            ),
        ),
    };
    let visuals = WorldVisualAssets {
        backdrop_noise: asset_server.load("sprites/world/backdrop_noise.png"),
        boundary: asset_server.load(boundary_sprite_path),
        pipe_masks,
        port_overlay_bidir,
        port_overlay_in,
        port_overlay_out,
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
                    &content_registry,
                    &asset_server,
                    &cell_visuals,
                    &cell_visual_layouts,
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
    let boundary_main_tint = cell_visuals.main_tint(boundary_material);
    let boundary_gas_tint = cell_visuals.gas_tint(boundary_material);
    spawn_outer_border_layers(
        &mut commands,
        &visuals,
        boundary_main_tint,
        boundary_gas_tint,
        boundary_material,
        &cell_visual_layouts,
        show_world,
    );

    let structure_draw_ranks = build_structure_draw_ranks(&structures, &structure_visuals);
    let mut structure_entities = GasStructureEntities::default();
    for structure in structures.iter() {
        if let Some(entity) =
            spawn_structure_sprite(
                &mut commands,
                &visuals,
                &content_registry,
                &asset_server,
                &structure_visuals,
                structure,
                *structure_draw_ranks.get(&structure.id).unwrap_or(&0),
                show_world,
            )
        {
            structure_entities
                .by_id
                .insert(structure_visual_key(structure), entity);
        }
    }
    commands.insert_resource(structure_entities);

    let mut pipe_entities = PipeEntities::default();
    let legacy_pipes_overlay = legacy_pipes_overlay_active(*overlay_mode, None);
    for structure in structures.iter() {
        spawn_structure_pipe_visuals(
            &mut commands,
            &visuals,
            &structure_visuals,
            &mut pipe_entities,
            &structures,
            structure,
            *structure_draw_ranks.get(&structure.id).unwrap_or(&0),
            show_world,
            *overlay_mode,
            legacy_pipes_overlay,
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
    boundary_material: CellMaterial,
    cell_visual_layouts: &crate::config::CellVisualPlacementConfigMap,
    show_world: bool,
) {
    let min_x = 0_i32;
    let min_y = 0_i32;
    let max_x = WORLD_WIDTH as i32 - 1;
    let max_y = WORLD_HEIGHT as i32 - 1;
    let boundary_priority = cell_visual_layouts.get(boundary_material).draw_priority;
    let mut draw_rank = 0usize;

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
                        custom_size: Some(size_in_world(
                            cell_visual_layouts.get(boundary_material).size_in_cells,
                        )),
                        color: boundary_main_tint,
                        ..default()
                    },
                    Transform::from_translation(
                        outer_cell_center(x, y)
                            .extend(appearance_z(boundary_priority, draw_rank)),
                    ),
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
                draw_rank += 1;
            }
        }
    }
}

fn spawn_wall_sprite(
    commands: &mut Commands,
    content_registry: &ContentRegistry,
    asset_server: &AssetServer,
    cell_visuals: &CellTypeVisualConfig,
    cell_visual_layouts: &crate::config::CellVisualPlacementConfigMap,
    x: u32,
    y: u32,
    material: CellMaterial,
    show_world: bool,
) -> Entity {
    let image = if let Some(descriptor) = content_registry.cell_by_material(material) {
        asset_server.load(descriptor.sprite.image_path.as_str())
    } else {
        asset_server.load(wall_sprite_path_for_material(content_registry, material))
    };
    let main_tint = cell_visuals.main_tint(material);
    let gas_tint = cell_visuals.gas_tint(material);
    let visual_layout = cell_visual_layouts.get(material);
    let draw_rank = linear_index(x, y);

    commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(size_in_world(visual_layout.size_in_cells)),
                color: main_tint,
                ..default()
            },
            Transform::from_translation(
                cell_center(x, y).extend(appearance_z(visual_layout.draw_priority, draw_rank)),
            ),
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

fn boundary_material_from_world(world: &WorldGrid) -> Option<CellMaterial> {
    match world.cell(0, 0) {
        CellKind::Solid(material) => Some(material),
        CellKind::Empty => None,
    }
}

fn wall_sprite_path_for_material(
    content_registry: &ContentRegistry,
    material: CellMaterial,
) -> String {
    content_registry
        .cell_by_material(material)
        .map(|descriptor| descriptor.sprite.image_path.clone())
        .unwrap_or_else(|| crate::plugins::default_plugin::cell_sprite_path(material).to_string())
}

fn spawn_structure_sprite(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    content_registry: &ContentRegistry,
    asset_server: &AssetServer,
    structure_visuals: &crate::config::StructureVisualConfigMap,
    structure: &PlacedStructure,
    draw_rank: usize,
    show_world: bool,
) -> Option<Entity> {
    let _descriptor = content_registry.structure_by_kind(structure.kind)?;
    let _ = visuals;
    let image = asset_server.load(
        crate::plugins::default_plugin::structure_state_sprite_path(
            structure.kind,
            structure.state,
        ),
    );
    let visual_layout = structure_visuals.get(structure.kind);
    let mut transform = Transform::from_translation(
        structure_visual_center(structure).extend(appearance_z(visual_layout.draw_priority, draw_rank)),
    );
    apply_state_sprite_transform(
        &mut transform,
        crate::plugins::default_plugin::structure_state_sprite_transform(
            structure.kind,
            structure.state,
        ),
    );
    let entity = commands
        .spawn((
            Sprite {
                image,
                custom_size: Some(size_in_world(
                    crate::world::structures::structure_sprite_size_in_cells(
                        structure.kind,
                        structure.rotation,
                        structure_visuals,
                    ),
                )),
                color: Color::WHITE,
                ..default()
            },
            transform,
            if show_world {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            GasStructureVisual,
        ))
        .id();
    Some(entity)
}

fn spawn_structure_pipe_visuals(
    commands: &mut Commands,
    visuals: &WorldVisualAssets,
    structure_visuals: &crate::config::StructureVisualConfigMap,
    entities: &mut PipeEntities,
    structures: &PlacedStructureMap,
    structure: &PlacedStructure,
    draw_rank: usize,
    show_world: bool,
    overlay_mode: OverlayMode,
    legacy_pipes_overlay: bool,
) {
    let _ = (structure_visuals, draw_rank);
    if crate::plugins::default_plugin::is_pipe_structure(structure.kind) {
            let x = structure.origin.x;
            let y = structure.origin.y;
            let center = cell_center(x, y);
            let _ = structures;
            let mask = (structure.state.value() & 0b1111) as usize;

            let highlight_entity =
                crate::render::pipe_highlight_material::spawn_pipe_highlight_entity(
                    commands,
                    &visuals.pipe_highlight,
                    mask,
                    Transform::from_translation(center.extend(pipe_highlight_z())),
                    pipe_highlight_visibility(show_world, overlay_mode, legacy_pipes_overlay),
                );
            commands
                .entity(highlight_entity)
                .insert(PipeHighlightOverlayVisual);
            entities.pipe_highlights.insert((x, y), highlight_entity);
            spawn_pipe_gas_overlay_slots(commands, entities, x, y, center);
    } else if crate::plugins::default_plugin::is_vent_structure(structure.kind) {
            let x = structure.origin.x;
            let y = structure.origin.y;
            let center = cell_center(x, y);
            let overlay_entity = commands
                .spawn((
                    Sprite {
                        image: visuals.port_overlay_bidir.clone(),
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
    } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind) {
            let Some(center_cell) = crate::world::structures::bridge_center_cell(structure.origin, structure.rotation)
            else {
                return;
            };
            spawn_pipe_gas_overlay_slots(
                commands,
                entities,
                center_cell.x,
                center_cell.y,
                cell_center(center_cell.x, center_cell.y),
            );
            for connection_cell in bridge_connection_cells_for_render(structure.origin, structure.rotation) {
                entities
                    .vent_overlays
                    .entry((connection_cell.x, connection_cell.y))
                    .or_insert_with(|| {
                        commands
                            .spawn((
                                Sprite {
                                    image: visuals.port_overlay_bidir.clone(),
                                    custom_size: Some(Vec2::splat(CELL_SIZE * 0.92)),
                                    color: pipe_overlay_vent_tint(),
                                    ..default()
                                },
                                Transform::from_translation(
                                    cell_center(connection_cell.x, connection_cell.y).extend(1.15),
                                ),
                                Visibility::Hidden,
                                PipeVentOverlayVisual,
                            ))
                            .id()
                    });
            }
    } else if crate::plugins::default_plugin::is_gas_pump_structure(structure.kind) {
            let (input_cell, output_cell) =
                pump_port_cells_for_render(structure.origin, structure.rotation);
            for (cell, image) in [
                (input_cell, visuals.port_overlay_in.clone()),
                (output_cell, visuals.port_overlay_out.clone()),
            ] {
                entities
                    .vent_overlays
                    .entry((cell.x, cell.y))
                    .or_insert_with(|| {
                        commands
                            .spawn((
                                Sprite {
                                    image,
                                    custom_size: Some(Vec2::splat(CELL_SIZE * 0.92)),
                                    color: pipe_overlay_vent_tint(),
                                    ..default()
                                },
                                Transform::from_translation(
                                    cell_center(cell.x, cell.y).extend(1.15),
                                ),
                                Visibility::Hidden,
                                PipeVentOverlayVisual,
                            ))
                            .id()
                    });
            }
    }
}

fn spawn_pipe_gas_overlay_slots(
    commands: &mut Commands,
    entities: &mut PipeEntities,
    x: u32,
    y: u32,
    center: Vec2,
) {
    if entities.gas_overlays.contains_key(&(x, y)) {
        return;
    }

    let overlays = [
        commands
            .spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(CELL_SIZE * 0.7)),
                Transform::from_translation(center.extend(1.05)),
                Visibility::Hidden,
                PipeGasOverlayVisual,
            ))
            .id(),
        commands
            .spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(CELL_SIZE * 0.7)),
                Transform::from_translation(center.extend(1.05)),
                Visibility::Hidden,
                PipeGasOverlayVisual,
            ))
            .id(),
    ];
    let borders = [
        commands
            .spawn((
                Sprite::from_color(Color::WHITE, Vec2::splat(CELL_SIZE * 0.7)),
                Transform::from_translation(center.extend(1.04)),
                Visibility::Hidden,
                PipeGasOverlayBorderVisual,
            ))
            .id(),
        commands
            .spawn((
                Sprite::from_color(Color::WHITE, Vec2::splat(CELL_SIZE * 0.7)),
                Transform::from_translation(center.extend(1.04)),
                Visibility::Hidden,
                PipeGasOverlayBorderVisual,
            ))
            .id(),
    ];
    entities.gas_overlays.insert((x, y), overlays);
    entities.gas_overlay_borders.insert((x, y), borders);
}

fn bridge_connection_cells_for_render(origin: UVec2, rotation: StructureRotation) -> Vec<UVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => vec![
            UVec2::new(origin.x, origin.y),
            UVec2::new(origin.x + 2, origin.y),
        ],
        StructureRotation::Deg90 | StructureRotation::Deg270 => vec![
            UVec2::new(origin.x, origin.y),
            UVec2::new(origin.x, origin.y + 2),
        ],
    }
}

fn pump_port_cells_for_render(origin: UVec2, rotation: StructureRotation) -> (UVec2, UVec2) {
    match rotation {
        StructureRotation::Deg0 => (origin, UVec2::new(origin.x + 1, origin.y)),
        StructureRotation::Deg90 => (origin, UVec2::new(origin.x, origin.y + 1)),
        StructureRotation::Deg180 => (UVec2::new(origin.x + 1, origin.y), origin),
        StructureRotation::Deg270 => (UVec2::new(origin.x, origin.y + 1), origin),
    }
}

#[cfg(test)]
fn bridge_visual_size(
    rotation: StructureRotation,
    structure_visuals: &crate::config::StructureVisualConfigMap,
) -> Vec2 {
    let size = crate::world::structures::structure_sprite_size_in_cells(
        crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
        rotation,
        structure_visuals,
    );
    Vec2::new(size.x.max(1) as f32 * CELL_SIZE, size.y.max(1) as f32 * CELL_SIZE)
}

#[cfg(test)]
fn bridge_visual_transform(
    center_cell: UVec2,
    rotation: StructureRotation,
    appearance_z: f32,
) -> Transform {
    let mut transform =
        Transform::from_translation(cell_center(center_cell.x, center_cell.y).extend(appearance_z));
    transform.rotation = match rotation {
        StructureRotation::Deg0 => Quat::IDENTITY,
        StructureRotation::Deg90 => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        StructureRotation::Deg180 => Quat::from_rotation_z(std::f32::consts::PI),
        StructureRotation::Deg270 => Quat::from_rotation_z(std::f32::consts::PI * 1.5),
    };
    transform
}

#[cfg(test)]
fn pipe_overlay_slot_offset(slot: usize, total_slots: usize) -> Vec2 {
    if total_slots <= 1 {
        Vec2::ZERO
    } else if slot == 0 {
        Vec2::new(0.0, CELL_SIZE * 0.18)
    } else {
        Vec2::new(0.0, -CELL_SIZE * 0.18)
    }
}

fn world_layer_visibility(show_world: bool) -> Visibility {
    if show_world {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn pipe_sprite_tint(
    overlay_mode: OverlayMode,
    pipe_visual: &PipeWorldVisual,
    legacy_pipes_overlay: bool,
) -> Color {
    if legacy_pipes_overlay && crate::plugins::default_plugin::is_pipes_overlay_mode(overlay_mode) {
        pipe_visual.pipe_tint
    } else {
        match overlay_mode {
            OverlayMode::Main => pipe_visual.main_tint,
            OverlayMode::Gas => pipe_visual.gas_tint,
            OverlayMode::Plugin(_) => pipe_visual.main_tint,
        }
    }
}

fn pipe_highlight_visibility(
    show_world: bool,
    overlay_mode: OverlayMode,
    legacy_pipes_overlay: bool,
) -> Visibility {
    if show_world
        && legacy_pipes_overlay
        && crate::plugins::default_plugin::is_pipes_overlay_mode(overlay_mode)
    {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn pipe_highlight_z() -> f32 {
    0.97
}

fn structure_visual_center(structure: &PlacedStructure) -> Vec2 {
    let descriptor = structure.descriptor();
    let Some((min, max)) = descriptor.local_bounds() else {
        return cell_center(structure.origin.x, structure.origin.y);
    };
    let local_center = Vec2::new(
        (min.x + max.x) as f32 * 0.5,
        (min.y + max.y) as f32 * 0.5,
    );
    cell_center(structure.origin.x, structure.origin.y) + local_center * CELL_SIZE
}

fn apply_state_sprite_transform(
    transform: &mut Transform,
    state_transform: flux_plugin_sdk::EntitySpriteTransform,
) {
    match state_transform {
        flux_plugin_sdk::EntitySpriteTransform::None => {}
        flux_plugin_sdk::EntitySpriteTransform::Rot90 => {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        }
        flux_plugin_sdk::EntitySpriteTransform::Rot180 => {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::PI);
        }
        flux_plugin_sdk::EntitySpriteTransform::Rot270 => {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::PI * 1.5);
        }
        flux_plugin_sdk::EntitySpriteTransform::FlipX => {
            transform.scale.x *= -1.0;
        }
        flux_plugin_sdk::EntitySpriteTransform::FlipY => {
            transform.scale.y *= -1.0;
        }
    }
}

fn appearance_z(draw_priority: i32, draw_rank: usize) -> f32 {
    const APPEARANCE_BASE_Z: f32 = 0.20;
    const APPEARANCE_PRIORITY_STEP: f32 = 0.0001;
    const APPEARANCE_TIEBREAK_STEP: f32 = 0.0000001;
    APPEARANCE_BASE_Z
        + draw_priority as f32 * APPEARANCE_PRIORITY_STEP
        + draw_rank as f32 * APPEARANCE_TIEBREAK_STEP
}

fn size_in_world(size_in_cells: UVec2) -> Vec2 {
    Vec2::new(
        size_in_cells.x.max(1) as f32 * CELL_SIZE,
        size_in_cells.y.max(1) as f32 * CELL_SIZE,
    )
}

fn build_structure_draw_ranks(
    structures: &PlacedStructureMap,
    structure_visuals: &crate::config::StructureVisualConfigMap,
) -> std::collections::HashMap<PlacedStructureId, usize> {
    let mut ordered = structures.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|structure| {
        (
            structure_visuals.get(structure.kind).draw_priority,
            structure.id.0,
        )
    });
    ordered
        .into_iter()
        .enumerate()
        .map(|(rank, structure)| (structure.id, rank))
        .collect()
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
    content_registry: Res<ContentRegistry>,
    world_load_state: Res<WorldLoadState>,
    asset_server: Res<AssetServer>,
    cell_visuals: Res<CellTypeVisualConfig>,
    cell_visual_layouts: Res<crate::config::CellVisualPlacementConfigMap>,
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
                    &content_registry,
                    &asset_server,
                    &cell_visuals,
                    &cell_visual_layouts,
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
                    &content_registry,
                    &asset_server,
                    &cell_visuals,
                    &cell_visual_layouts,
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
    structures: Res<PlacedStructureMap>,
    content_registry: Res<ContentRegistry>,
    world_load_state: Res<WorldLoadState>,
    visuals: Res<WorldVisualAssets>,
    asset_server: Res<AssetServer>,
    structure_visuals: Res<crate::config::StructureVisualConfigMap>,
    mut structure_entities: ResMut<GasStructureEntities>,
) {
    if !structures.is_changed() && !world_load_state.is_changed() {
        return;
    }

    for entity in structure_entities.by_id.values().copied() {
        commands.entity(entity).despawn();
    }
    structure_entities.by_id.clear();

    if !world_load_state.has_world {
        return;
    }

    let structure_draw_ranks = build_structure_draw_ranks(&structures, &structure_visuals);
    for structure in structures.iter() {
        if let Some(entity) = spawn_structure_sprite(
            &mut commands,
            &visuals,
            &content_registry,
            &asset_server,
            &structure_visuals,
            structure,
            *structure_draw_ranks.get(&structure.id).unwrap_or(&0),
            world_load_state.has_world,
        )
        {
            structure_entities
                .by_id
                .insert(structure_visual_key(structure), entity);
        }
    }
}

fn structure_visual_key(structure: &PlacedStructure) -> PlacedStructureId {
    structure.id
}

fn structure_sprite_path_for_tag(
    content_registry: &ContentRegistry,
    raw_tag: &str,
) -> Option<String> {
    content_registry
        .structure_by_tag(raw_tag)
        .map(|descriptor| descriptor.sprite.image_path.clone())
}

pub(crate) fn sync_pipe_world_visuals(
    mut commands: Commands,
    structures: Res<PlacedStructureMap>,
    world_load_state: Res<WorldLoadState>,
    overlay_mode: Res<OverlayMode>,
    graph_scene: Option<Res<super::overlay_graph_runtime::OverlayGraphSceneState>>,
    visuals: Res<WorldVisualAssets>,
    structure_visuals: Res<crate::config::StructureVisualConfigMap>,
    mut pipe_entities: ResMut<PipeEntities>,
) {
    if !structures.is_changed() && !world_load_state.is_changed() && !overlay_mode.is_changed() {
        return;
    }

    let mut entities_to_despawn = Vec::new();
    entities_to_despawn.extend(pipe_entities.pipes.values().copied());
    entities_to_despawn.extend(pipe_entities.bridges.values().copied());
    entities_to_despawn.extend(pipe_entities.pipe_highlights.values().copied());
    entities_to_despawn.extend(pipe_entities.vents.values().copied());
    entities_to_despawn.extend(
        pipe_entities
            .gas_overlays
            .values()
            .flat_map(|pair| pair.iter().copied()),
    );
    entities_to_despawn.extend(
        pipe_entities
            .gas_overlay_borders
            .values()
            .flat_map(|pair| pair.iter().copied()),
    );
    entities_to_despawn.extend(pipe_entities.vent_overlays.values().copied());
    for entity in entities_to_despawn {
        commands.entity(entity).despawn();
    }
    pipe_entities.pipes.clear();
    pipe_entities.bridges.clear();
    pipe_entities.pipe_highlights.clear();
    pipe_entities.vents.clear();
    pipe_entities.gas_overlays.clear();
    pipe_entities.gas_overlay_borders.clear();
    pipe_entities.vent_overlays.clear();

    if !world_load_state.has_world {
        return;
    }

    let structure_draw_ranks = build_structure_draw_ranks(&structures, &structure_visuals);
    let legacy_pipes_overlay = legacy_pipes_overlay_active(*overlay_mode, graph_scene.as_deref());
    for structure in structures.iter() {
        spawn_structure_pipe_visuals(
            &mut commands,
            &visuals,
            &structure_visuals,
            &mut pipe_entities,
            &structures,
            structure,
            *structure_draw_ranks.get(&structure.id).unwrap_or(&0),
            world_load_state.has_world,
            *overlay_mode,
            legacy_pipes_overlay,
        );
    }
}

pub(crate) fn sync_structure_edit_highlight(
    world_load_state: Res<WorldLoadState>,
    structure_edit: Res<StructureEditState>,
    structures: Res<PlacedStructureMap>,
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
    if structures.editable_structure_at(cell.x, cell.y).is_none() {
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
            draw_rect(&mut data, half - arm_half, 0, half + arm_half, half);
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
            draw_rect(
                &mut data,
                half - arm_half,
                half,
                half + arm_half,
                size as i32,
            );
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
