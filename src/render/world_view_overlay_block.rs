pub fn update_overlay_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut overlay_mode: ResMut<OverlayMode>,
) {
    if input.just_pressed(KeyCode::F1) {
        *overlay_mode = OverlayMode::Main;
    }
    if input.just_pressed(KeyCode::F2) {
        *overlay_mode = OverlayMode::Gas;
    }
}

/// Runs `apply_overlay_mode` logic.
pub(crate) fn apply_overlay_mode(
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    mut sprite_sets: ParamSet<(
        Query<
            (&mut Sprite, &mut Visibility),
            (
                With<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            (&mut Sprite, &mut Visibility),
            (
                With<BackdropLayer>,
                Without<BoardLayer>,
                Without<WallVisual>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            (&WallVisual, &mut Sprite, &mut Visibility),
            (
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
            ),
        >,
        Query<
            &mut Visibility,
            (
                With<GasOverlaySprite>,
                Without<GasMainOverlaySprite>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
            ),
        >,
        Query<
            &mut Visibility,
            (
                With<GasMainOverlaySprite>,
                Without<GasOverlaySprite>,
                Without<BoardLayer>,
                Without<BackdropLayer>,
                Without<WallVisual>,
            ),
        >,
    )>,
) {
    if !overlay_mode.is_changed() && !world_load_state.is_changed() {
        return;
    }

    let show_world = world_load_state.has_world;
    let (board_color, backdrop_color, gas_visibility, gas_main_visibility) = match *overlay_mode {
        OverlayMode::Main => (
            BOARD_MAIN_COLOR,
            BACKDROP_MAIN_COLOR,
            Visibility::Hidden,
            Visibility::Visible,
        ),
        OverlayMode::Gas => (
            BOARD_GAS_COLOR,
            BACKDROP_GAS_COLOR,
            Visibility::Visible,
            Visibility::Hidden,
        ),
    };

    for (mut board, mut visibility) in &mut sprite_sets.p0() {
        board.color = board_color;
        *visibility = if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (mut backdrop, mut visibility) in &mut sprite_sets.p1() {
        backdrop.color = backdrop_color;
        *visibility = if show_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut sprite_sets.p3() {
        *visibility = if show_world {
            gas_visibility
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut sprite_sets.p4() {
        *visibility = if show_world {
            gas_main_visibility
        } else {
            Visibility::Hidden
        };
    }
    for (wall_visual, mut sprite, mut visibility) in &mut sprite_sets.p2() {
        sprite.color = match *overlay_mode {
            OverlayMode::Main => wall_visual.main_tint,
            OverlayMode::Gas => wall_visual.gas_tint,
        };
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
    mut gizmos: Gizmos,
) {
    if !world_load_state.has_world {
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

