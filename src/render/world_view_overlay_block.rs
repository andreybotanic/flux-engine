pub fn update_overlay_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut overlay_mode: ResMut<OverlayMode>,
    main_menu: Option<Res<crate::editor::MainMenuState>>,
) {
    if main_menu.as_ref().map(|menu| menu.open).unwrap_or(false) {
        return;
    }
    if input.just_pressed(KeyCode::F1) {
        *overlay_mode = OverlayMode::Main;
    }
    if input.just_pressed(KeyCode::F2) {
        *overlay_mode = OverlayMode::Gas;
    }
    if input.just_pressed(KeyCode::F3) {
        *overlay_mode = OverlayMode::Pipes;
    }
}

/// Runs `apply_overlay_mode` logic.
pub(crate) fn apply_overlay_mode(
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    mut visuals: ParamSet<(
        Query<
            '_,
            '_,
            (&mut Sprite, &mut Visibility),
            (
                With<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            '_,
            '_,
            (&mut Sprite, &mut Visibility),
            (
                With<BackdropLayer>,
                Without<BoardLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            '_,
            '_,
            (&WallVisual, &mut Sprite, &mut Visibility),
            (
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            '_,
            '_,
            (&OuterBorderVisual, &mut Sprite, &mut Visibility),
            (
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            '_,
            '_,
            (&PipeWorldVisual, &mut Sprite, &mut Visibility, &mut Transform),
            (
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            '_,
            '_,
            (&mut Visibility, &mut Transform),
            (
                With<PipeHighlightOverlayVisual>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            '_,
            '_,
            (&mut Visibility, &mut Sprite),
            (
                With<VentWorldVisual>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<WorldFadeMaskLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
    )>,
) {
    let show_world = world_load_state.has_world;
    let (board_color, backdrop_color) = match *overlay_mode {
        OverlayMode::Main => (
            BOARD_MAIN_COLOR,
            BACKDROP_MAIN_COLOR,
        ),
        OverlayMode::Gas => (
            BOARD_GAS_COLOR,
            BACKDROP_GAS_COLOR,
        ),
        OverlayMode::Pipes => (
            BOARD_PIPE_COLOR,
            BACKDROP_PIPE_COLOR,
        ),
    };

    for (mut board, mut visibility) in &mut visuals.p0() {
        board.color = board_color;
        *visibility = world_layer_visibility(show_world);
    }
    for (mut backdrop, mut visibility) in &mut visuals.p1() {
        backdrop.color = backdrop_color;
        *visibility = world_layer_visibility(show_world);
    }
    for (wall_visual, mut sprite, mut visibility) in &mut visuals.p2() {
        sprite.color = match *overlay_mode {
            OverlayMode::Main => wall_visual.main_tint,
            OverlayMode::Gas => wall_visual.gas_tint,
            OverlayMode::Pipes => wall_visual.main_tint.with_alpha(0.18),
        };
        *visibility = world_layer_visibility(show_world);
    }
    for (outer_border_visual, mut sprite, mut visibility) in &mut visuals.p3() {
        sprite.color = match *overlay_mode {
            OverlayMode::Main => outer_border_visual.main_tint,
            OverlayMode::Gas => outer_border_visual.gas_tint,
            OverlayMode::Pipes => outer_border_visual.main_tint.with_alpha(0.12),
        };
        *visibility = world_layer_visibility(show_world);
    }
    for (pipe_visual, mut sprite, mut visibility, mut transform) in &mut visuals.p4() {
        sprite.color = pipe_sprite_tint(*overlay_mode, pipe_visual);
        transform.translation.z = pipe_visual.appearance_z;
        *visibility = world_layer_visibility(show_world);
    }
    for (mut visibility, mut transform) in &mut visuals.p5() {
        transform.translation.z = pipe_highlight_z();
        *visibility = pipe_highlight_visibility(show_world, *overlay_mode);
    }
    for (mut visibility, mut sprite) in &mut visuals.p6() {
        *visibility = vent_world_visibility(show_world, *overlay_mode);
        sprite.color = Color::WHITE;
    }
}

/// Runs `apply_overlay_visibility_mode` logic.
pub(crate) fn apply_overlay_visibility_mode(
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    mut visuals: ParamSet<(
        Query<
            '_,
            '_,
            &mut Visibility,
            (
                With<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
            ),
        >,
        Query<
            '_,
            '_,
            &mut Visibility,
            (
                With<GasMainOverlaySprite>,
                Without<GasOverlaySprite>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<WorldFadeMaskLayer>,
            ),
        >,
        Query<
            '_,
            '_,
            &mut Visibility,
            (
                With<WorldFadeMaskLayer>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<OuterBorderVisual>,
                Without<PipeWorldVisual>,
                Without<VentWorldVisual>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
    )>,
) {
    let show_world = world_load_state.has_world;
    let (gas_visibility, gas_main_mode_visibility) = match *overlay_mode {
        OverlayMode::Main => (Visibility::Hidden, Visibility::Visible),
        OverlayMode::Gas => (Visibility::Visible, Visibility::Hidden),
        OverlayMode::Pipes => (Visibility::Hidden, Visibility::Hidden),
    };

    for mut visibility in &mut visuals.p0() {
        *visibility = if show_world {
            gas_visibility
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut visuals.p1() {
        *visibility = if show_world {
            gas_main_mode_visibility
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut visuals.p2() {
        *visibility = if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Runs `draw_cursor_grid_overlay` logic.
pub fn draw_cursor_grid_overlay(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    world_load_state: Res<WorldLoadState>,
    panels: Option<Res<PanelManager>>,
    main_menu: Option<Res<crate::editor::MainMenuState>>,
    mut gizmos: Gizmos,
) {
    if !world_load_state.has_world {
        return;
    }
    if main_menu.as_ref().map(|menu| menu.open).unwrap_or(false) {
        return;
    }

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    if panels
        .as_ref()
        .map(|panel_manager| panel_manager.is_cursor_over_any_panel(cursor_pos))
        .unwrap_or(false)
    {
        return;
    }

    let (camera, camera_transform) = *camera_query;
    let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };
    let Some(cursor_cell) = world_to_cell(cursor_world_pos) else {
        return;
    };

    for dy in -CURSOR_GRID_RADIUS_CELLS..=CURSOR_GRID_RADIUS_CELLS {
        for dx in -CURSOR_GRID_RADIUS_CELLS..=CURSOR_GRID_RADIUS_CELLS {
            let x = cursor_cell.x as i32 + dx;
            let y = cursor_cell.y as i32 + dy;
            if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
                continue;
            }

            let center = cell_center(x as u32, y as u32);
            let cell_distance = (center - cursor_world_pos).length() / CELL_SIZE;
            let fade = grid_fade(cell_distance, CURSOR_GRID_FADE_RADIUS);
            if fade <= 0.01 {
                continue;
            }

            let color = GRID_LINE_COLOR.with_alpha(CURSOR_GRID_MAX_ALPHA * fade);
            gizmos.rect_2d(
                Isometry2d::from_translation(center),
                Vec2::splat(CELL_SIZE),
                color,
            );
        }
    }
}

/// Runs `sync_gas_display_texture` logic.
pub(crate) fn sync_gas_display_texture(
    step: Res<SimulationStep>,
    gas: Res<GasField>,
    gas_registry: Res<GasRegistry>,
    visual_settings: Res<GasVisualSettings>,
    main_view_settings: Res<GasMainViewVisualConfig>,
    simulation_images: Res<GasSimulationImages>,
    mut images: ResMut<Assets<Image>>,
    mut gas_query: Query<&mut Sprite, (With<GasOverlaySprite>, Without<GasMainOverlaySprite>)>,
    mut gas_main_query: Query<&mut Sprite, (With<GasMainOverlaySprite>, Without<GasOverlaySprite>)>,
) {
    if !step.is_changed()
        && !gas.is_changed()
        && !gas_registry.is_changed()
        && !visual_settings.is_changed()
        && !main_view_settings.is_changed()
    {
        return;
    }

    for image_handle in [
        &simulation_images.texture_f2_a,
        &simulation_images.texture_f2_b,
    ] {
        let Some(image) = images.get_mut(image_handle) else {
            continue;
        };

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let texture_y = WORLD_HEIGHT - 1 - y;
                let color = if is_boundary(x, y) {
                    Color::linear_rgba(0.0, 1.0, 0.0, 1.0)
                } else {
                    // Keep overlay semantics consistent with HUD "particles" values.
                    // Tiny float residuals from solver should not light up cells as non-zero gas.
                    let particles = gas.total_amount_rounded(x, y) as f32;
                    let storage_linear =
                        (particles / HYDROGEN_GPU_STORAGE_MAX_PARTICLES as f32).clamp(0.0, 1.0);
                    let visual = gas_visual_intensity(
                        particles,
                        visual_settings.gamma,
                        visual_settings.max_particles_for_max_color as f32,
                        1.0,
                        0.05,
                    );
                    Color::linear_rgba(visual, 0.0, storage_linear, 1.0)
                };

                let _ = image.set_color_at(x, texture_y, color);
            }
        }
    }

    let texture = if step.0 % 2 == 0 {
        simulation_images.texture_f2_a.clone()
    } else {
        simulation_images.texture_f2_b.clone()
    };

    for mut sprite in &mut gas_query {
        sprite.image = texture.clone();
    }

    for image_handle in [
        &simulation_images.texture_f1_a,
        &simulation_images.texture_f1_b,
    ] {
        let Some(image) = images.get_mut(image_handle) else {
            continue;
        };
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let texture_y = WORLD_HEIGHT - 1 - y;
                let color = if is_boundary(x, y) {
                    Color::linear_rgba(0.0, 0.0, 0.0, 0.0)
                } else {
                    let total = gas.total_amount(x, y).max(0.0);
                    if total <= 1e-6 {
                        Color::linear_rgba(0.0, 0.0, 0.0, 0.0)
                    } else {
                        let mut weighted_rgb = Vec3::ZERO;
                        for gas_index in 0..gas.gas_count() {
                            let amount = gas.amount(x, y, gas_index).max(0.0);
                            if amount <= 1e-6 {
                                continue;
                            }
                            if let Some(gas_def) = gas_registry.get(gas_index) {
                                let rgb = Vec3::from_array(gas_def.color);
                                weighted_rgb += rgb * amount;
                            }
                        }
                        let mix_rgb = if weighted_rgb.length_squared() <= f32::EPSILON {
                            Vec3::ZERO
                        } else {
                            weighted_rgb / total.max(1e-6)
                        };
                        let visual = gas_visual_intensity(
                            total,
                            visual_settings.gamma,
                            main_view_settings.max_particles_for_max_intensity,
                            main_view_settings.min_particles,
                            main_view_settings.min_intensity,
                        );
                        let rgb = (mix_rgb * visual).clamp(Vec3::ZERO, Vec3::ONE);
                        let alpha = (visual * main_view_settings.alpha).clamp(0.0, 1.0);
                        Color::linear_rgba(rgb.x, rgb.y, rgb.z, alpha)
                    }
                };
                let _ = image.set_color_at(x, texture_y, color);
            }
        }
    }

    let f1_texture = if step.0 % 2 == 0 {
        simulation_images.texture_f1_a.clone()
    } else {
        simulation_images.texture_f1_b.clone()
    };
    for mut sprite in &mut gas_main_query {
        sprite.image = f1_texture.clone();
    }
}

fn grid_fade(distance_cells: f32, fade_radius_cells: f32) -> f32 {
    if fade_radius_cells <= f32::EPSILON || distance_cells >= fade_radius_cells {
        return 0.0;
    }
    if distance_cells <= 0.0 {
        return 1.0;
    }

    let t = (distance_cells / fade_radius_cells).clamp(0.0, 1.0);
    let smooth = 1.0 - t * t;
    smooth * smooth
}

pub(crate) fn sync_pipe_overlay_visuals(
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    structures: Res<PlacedStructureMap>,
    pipe_gas: Res<PipeGasField>,
    flow_state: Res<PipeFlowVisualState>,
    gas_registry: Res<GasRegistry>,
    visual_settings: Res<GasVisualSettings>,
    main_view_settings: Res<GasMainViewVisualConfig>,
    pipe_entities: Res<PipeEntities>,
    mut visuals: ParamSet<(
        Query<
            '_,
            '_,
            (&mut Sprite, &mut Transform, &mut Visibility),
            (
                With<PipeGasOverlayVisual>,
                Without<PipeGasOverlayBorderVisual>,
                Without<PipeVentOverlayVisual>,
            ),
        >,
        Query<
            '_,
            '_,
            (&mut Sprite, &mut Transform, &mut Visibility),
            (
                With<PipeGasOverlayBorderVisual>,
                Without<PipeGasOverlayVisual>,
                Without<PipeVentOverlayVisual>,
            ),
        >,
        Query<
            '_,
            '_,
            &mut Visibility,
            (
                With<PipeVentOverlayVisual>,
                Without<PipeGasOverlayVisual>,
                Without<PipeGasOverlayBorderVisual>,
            ),
        >,
    )>,
) {
    let show_pipe_overlay = world_load_state.has_world && *overlay_mode == OverlayMode::Pipes;

    for ((x, y), overlay_entities) in &pipe_entities.gas_overlays {
        let Some(border_entities) = pipe_entities.gas_overlay_borders.get(&(*x, *y)) else {
            continue;
        };
        let display_blocks = if show_pipe_overlay {
            pipe_cell_display_blocks_with_transfers(
                &structures,
                &pipe_gas,
                &flow_state,
                *x,
                *y,
                false,
            )
        } else {
            Vec::new()
        };

        for slot in 0..overlay_entities.len() {
            let overlay_entity = overlay_entities[slot];
            let border_entity = border_entities[slot];
            let block = display_blocks
                .get(slot)
                .filter(|block| pipe_overlay_block_visible(block.total_particles));
            let total_slots = display_blocks.len().min(overlay_entities.len());
            let cell_center = crate::world::grid::cell_center(*x, *y);

            if let Some(block) = block {
                let total_f = block.total_particles as f32;
                let mut weighted_rgb = Vec3::ZERO;
                for gas_index in 0..block.species_counts.len() {
                    let amount = block.species_counts[gas_index] as f32;
                    if amount <= 0.0 {
                        continue;
                    }
                    if let Some(gas_def) = gas_registry.get(gas_index) {
                        weighted_rgb += Vec3::from_array(gas_def.color) * amount;
                    }
                }
                let mix_rgb = if weighted_rgb.length_squared() <= f32::EPSILON {
                    Vec3::ZERO
                } else {
                    weighted_rgb / total_f.max(1.0)
                };
                let visual = gas_visual_intensity(
                    total_f,
                    visual_settings.gamma,
                    main_view_settings.max_particles_for_max_intensity,
                    1.0,
                    0.12,
                );
                let rgb = (mix_rgb * visual).clamp(Vec3::ZERO, Vec3::ONE);
                let outer_size = pipe_overlay_slot_size(block.total_particles, total_slots);
                let inner_size = pipe_square_inner_size(outer_size);
                let offset = pipe_overlay_block_offset(
                    slot,
                    total_slots,
                    block.kind,
                    &structures,
                    UVec2::new(*x, *y),
                );
                {
                    let mut gas_query = visuals.p0();
                    let Ok((mut sprite, mut transform, mut visibility)) =
                        gas_query.get_mut(overlay_entity)
                    else {
                        continue;
                    };
                    sprite.custom_size = Some(Vec2::splat(inner_size));
                    sprite.color = Color::linear_rgba(
                        rgb.x,
                        rgb.y,
                        rgb.z,
                        (0.42 + visual * 0.5).clamp(0.0, 1.0),
                    );
                    transform.translation = (cell_center + offset).extend(1.05);
                    *visibility = Visibility::Visible;
                }
                {
                    let mut border_query = visuals.p1();
                    let Ok((mut border_sprite, mut transform, mut border_visibility)) =
                        border_query.get_mut(border_entity)
                    else {
                        continue;
                    };
                    border_sprite.custom_size = Some(Vec2::splat(outer_size));
                    border_sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.96);
                    transform.translation = (cell_center + offset).extend(1.04);
                    *border_visibility = Visibility::Visible;
                }
            } else {
                if let Ok((_, _, mut visibility)) = visuals.p0().get_mut(overlay_entity) {
                    *visibility = Visibility::Hidden;
                }
                if let Ok((_, _, mut border_visibility)) = visuals.p1().get_mut(border_entity) {
                    *border_visibility = Visibility::Hidden;
                }
            }
        }
    }

    for ((x, y), entity) in &pipe_entities.vent_overlays {
        let mut vent_query = visuals.p2();
        let Ok(mut visibility) = vent_query.get_mut(*entity) else {
            continue;
        };
        let _ = (x, y);
        *visibility = if show_pipe_overlay {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn pipe_overlay_block_visible(total_particles: u32) -> bool {
    total_particles > 0
}

fn pipe_overlay_block_offset(
    slot: usize,
    total_slots: usize,
    kind: crate::simulation::pipes::PipeContainerKind,
    structures: &PlacedStructureMap,
    cell: UVec2,
) -> Vec2 {
    let Some(rotation) = bridge_rotation_for_center_cell(structures, cell) else {
        return pipe_overlay_slot_offset(slot, total_slots);
    };
    let bridge_offset = bridge_bend_direction(rotation) * (CELL_SIZE * 0.18);
    match kind {
        crate::simulation::pipes::PipeContainerKind::BridgePipe => bridge_offset,
        crate::simulation::pipes::PipeContainerKind::Pipe if total_slots > 1 => -bridge_offset,
        crate::simulation::pipes::PipeContainerKind::Pipe => Vec2::ZERO,
    }
}

fn bridge_rotation_for_center_cell(
    structures: &PlacedStructureMap,
    cell: UVec2,
) -> Option<StructureRotation> {
    structures.iter().find_map(|structure| {
        (structure.kind == StructureKind::GasPipeBridge
            && crate::world::structures::bridge_center_cell(structure.origin, structure.rotation)
                == Some(cell))
            .then_some(structure.rotation)
    })
}

pub(crate) fn sync_pipe_flow_packets(
    mut commands: Commands,
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    structures: Res<PlacedStructureMap>,
    flow_state: Res<PipeFlowVisualState>,
    control: Res<crate::simulation::SimulationControl>,
    gas_registry: Res<GasRegistry>,
    time: Res<Time>,
    mut pipe_entities: ResMut<PipeEntities>,
) {
    for entity in pipe_entities.flow_packets.drain(..) {
        commands.entity(entity).despawn();
    }

    if !pipe_flow_packets_enabled(
        world_load_state.has_world,
        *overlay_mode,
        control.paused,
        structures.is_changed(),
    ) {
        return;
    }

    let progress = (time.elapsed_secs() * 2.0).fract();
    for transfer in &flow_state.transfers {
        if transfer.total_amount == 0 {
            continue;
        }
        let position = flow_packet_position(transfer, progress);
        let total = transfer.total_amount as f32;
        let mut weighted_rgb = Vec3::ZERO;
        for (gas_index, amount) in transfer.gas_counts.iter().copied().enumerate() {
            if amount == 0 {
                continue;
            }
            if let Some(gas_def) = gas_registry.get(gas_index) {
                weighted_rgb += Vec3::from_array(gas_def.color) * amount as f32;
            }
        }
        let mix_rgb = if weighted_rgb.length_squared() <= f32::EPSILON {
            Vec3::splat(0.9)
        } else {
            weighted_rgb / total.max(1.0)
        };
        let packet_visual = pipe_flow_packet_visual(transfer.total_amount);
        let rgb = (mix_rgb * packet_visual.intensity).clamp(Vec3::ZERO, Vec3::ONE);
        let outer_size = pipe_flow_square_size(transfer.total_amount);
        let inner_size = pipe_square_inner_size(outer_size);
        let border_entity = commands
            .spawn((
                Sprite::from_color(
                    Color::srgba(1.0, 1.0, 1.0, 1.0),
                    Vec2::splat(outer_size),
                ),
                Transform::from_translation(position.extend(1.22)),
                PipeFlowPacketVisual,
            ))
            .id();
        let entity = commands
            .spawn((
                Sprite::from_color(
                    Color::linear_rgba(rgb.x, rgb.y, rgb.z, packet_visual.fill_alpha),
                    Vec2::splat(inner_size),
                ),
                Transform::from_translation(position.extend(1.23)),
                PipeFlowPacketVisual,
            ))
            .id();
        pipe_entities.flow_packets.push(border_entity);
        pipe_entities.flow_packets.push(entity);
    }
}

fn pipe_flow_packets_enabled(
    has_world: bool,
    overlay_mode: OverlayMode,
    paused: bool,
    structures_changed: bool,
) -> bool {
    has_world
        && overlay_mode == OverlayMode::Pipes
        && !paused
        && !structures_changed
}

const BRIDGE_PACKET_ARC_OFFSET_CELLS: f32 = 0.45;

fn flow_packet_position(
    transfer: &crate::simulation::pipes::PipeTransferRecord,
    t: f32,
) -> Vec2 {
    bridge_packet_position(transfer, t).unwrap_or_else(|| {
        straight_packet_position(
            cell_center(transfer.from.x, transfer.from.y),
            cell_center(transfer.to.x, transfer.to.y),
            t,
        )
    })
}

fn straight_packet_position(from: Vec2, to: Vec2, t: f32) -> Vec2 {
    from.lerp(to, t)
}

fn bridge_packet_position(
    transfer: &crate::simulation::pipes::PipeTransferRecord,
    t: f32,
) -> Option<Vec2> {
    let crate::simulation::pipes::PipeTransferVisualPath::BridgeArc {
        bridge_origin,
        bridge_rotation,
    } = transfer.visual_path
    else {
        return None;
    };
    let Some(center_cell) =
        crate::world::structures::bridge_center_cell(bridge_origin, bridge_rotation)
    else {
        return None;
    };
    let [first_port, second_port] = bridge_port_world_cells(bridge_origin, bridge_rotation);
    let curve_t =
        bridge_curve_progress_for_transfer(transfer, center_cell, first_port, second_port, t)?;
    Some(quadratic_bezier_point(
        cell_center(first_port.x, first_port.y),
        bridge_arc_control_point(center_cell, bridge_rotation),
        cell_center(second_port.x, second_port.y),
        curve_t,
    ))
}

fn bridge_curve_progress_for_transfer(
    transfer: &crate::simulation::pipes::PipeTransferRecord,
    center_cell: UVec2,
    first_port: UVec2,
    second_port: UVec2,
    t: f32,
) -> Option<f32> {
    if transfer.from == first_port && transfer.to == center_cell {
        return Some(t * 0.5);
    }
    if transfer.from == center_cell && transfer.to == first_port {
        return Some((1.0 - t) * 0.5);
    }
    if transfer.from == center_cell && transfer.to == second_port {
        return Some(0.5 + t * 0.5);
    }
    if transfer.from == second_port && transfer.to == center_cell {
        return Some(1.0 - t * 0.5);
    }
    None
}

fn quadratic_bezier_point(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let one_minus_t = 1.0 - t;
    p0 * (one_minus_t * one_minus_t) + p1 * (2.0 * one_minus_t * t) + p2 * (t * t)
}

fn bridge_arc_control_point(center_cell: UVec2, rotation: StructureRotation) -> Vec2 {
    let bend_offset = BRIDGE_PACKET_ARC_OFFSET_CELLS * CELL_SIZE;
    cell_center(center_cell.x, center_cell.y) + bridge_bend_direction(rotation) * bend_offset
}

fn bridge_bend_direction(rotation: StructureRotation) -> Vec2 {
    match rotation {
        StructureRotation::Deg0 => Vec2::Y,
        StructureRotation::Deg90 => Vec2::NEG_X,
        StructureRotation::Deg180 => Vec2::NEG_Y,
        StructureRotation::Deg270 => Vec2::X,
    }
}

fn bridge_port_world_cells(origin: UVec2, rotation: StructureRotation) -> [UVec2; 2] {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            [origin, UVec2::new(origin.x + 2, origin.y)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            [origin, UVec2::new(origin.x, origin.y + 2)]
        }
    }
}

fn pipe_gas_square_size(total_particles: u32) -> f32 {
    scaled_pipe_square_size(total_particles, 0.22, 0.63)
}

fn pipe_flow_square_size(moved_particles: u32) -> f32 {
    scaled_pipe_square_size(moved_particles, 0.21, 0.63)
}

struct PipeFlowPacketVisualParams {
    intensity: f32,
    fill_alpha: f32,
}

fn pipe_flow_packet_visual(moved_particles: u32) -> PipeFlowPacketVisualParams {
    let ratio = (moved_particles.min(1_000) as f32 / 1_000.0).clamp(0.0, 1.0);
    let eased = ratio.sqrt();
    PipeFlowPacketVisualParams {
        intensity: (0.08 + eased * 0.92).clamp(0.0, 1.0),
        fill_alpha: (0.12 + eased * 0.80).clamp(0.0, 0.92),
    }
}

fn scaled_pipe_square_size(particles: u32, min_fraction: f32, max_fraction: f32) -> f32 {
    if particles == 0 {
        return 0.0;
    }
    let fill_ratio = (particles.min(1_000) as f32 / 1_000.0).clamp(0.0, 1.0);
    CELL_SIZE * (min_fraction + (max_fraction - min_fraction) * fill_ratio)
}

fn pipe_square_inner_size(outer_size: f32) -> f32 {
    const PIPE_SQUARE_BORDER_THICKNESS: f32 = CELL_SIZE / 64.0;
    (outer_size - PIPE_SQUARE_BORDER_THICKNESS * 2.0).max(0.0)
}

