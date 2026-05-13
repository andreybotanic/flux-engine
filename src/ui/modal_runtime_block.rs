fn update_active_modal_backdrop_state(
    mut tracker: ResMut<ModalOpenTracker>,
    mut active: ResMut<ActiveModalBackdropState>,
    modal_roots: Query<
        (Entity, &ModalBackdropSpec, &Visibility, Option<&GlobalZIndex>),
        (With<ModalRoot>, With<ModalBackdropController>),
    >,
    panel_surfaces: Query<(Entity, &ChildOf), With<ModalPanelSurface>>,
) {
    let next_active = choose_topmost_modal_candidate(modal_roots.iter().map(
        |(entity, _, visibility, global_z)| ModalCandidate {
            entity,
            visible: *visibility != Visibility::Hidden,
            global_z: global_z.map(|value| value.0).unwrap_or(0),
        },
    ));

    if let Some(candidate) = next_active {
        if tracker.previous_active_root != Some(candidate.entity) {
            tracker.open_generation += 1;
            tracker.previous_active_root = Some(candidate.entity);
        }

        let mut active_panel = None;
        for (panel_entity, child_of) in &panel_surfaces {
            if child_of.parent() == candidate.entity {
                active_panel = Some(panel_entity);
                break;
            }
        }

        if let Ok((_, spec, _, _)) = modal_roots.get(candidate.entity) {
            active.active_root = Some(candidate.entity);
            active.active_panel = active_panel;
            active.active_spec = Some(spec.clone());
            active.active_global_z = candidate.global_z;
            active.active_open_generation = tracker.open_generation;
            return;
        }
    }

    tracker.previous_active_root = None;
    active.active_root = None;
    active.active_panel = None;
    active.active_spec = None;
    active.active_global_z = 0;
    active.active_open_generation = 0;
}

fn prepare_asset_backdrop_blur_cache(
    active: Res<ActiveModalBackdropState>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<ModalBackdropAssetCache>,
) {
    let Some(spec) = active.active_spec.as_ref() else {
        return;
    };
    let ModalBackdropSource::Asset(source) = &spec.source else {
        return;
    };

    let key = blurred_asset_key(source, spec.blur_sigma);
    if cache.blurred_by_asset.contains_key(&key) {
        return;
    }

    let Some(source_image) = images.get(source).cloned() else {
        return;
    };
    let blurred_handle = match build_blurred_backdrop_image(&source_image, spec.blur_sigma) {
        Ok(blurred) => images.add(blurred),
        Err(_) => source.clone(),
    };
    cache.blurred_by_asset.insert(key, blurred_handle);
}

fn sync_modal_backdrop_visuals(
    active: Res<ActiveModalBackdropState>,
    target: Res<ModalWorldSnapshotTarget>,
    snapshot_state: Res<ModalWorldSnapshotState>,
    cache: Res<ModalBackdropAssetCache>,
    images: Res<Assets<Image>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut fullscreen_layers: Query<
        (Entity, &ChildOf, &mut ImageNode, &mut Node, &mut Visibility),
        (
            With<ModalFullscreenBackdropLayer>,
            Without<ModalFullscreenVeilLayer>,
            Without<ModalPanelBackdropLayer>,
            Without<ModalPanelSurface>,
        ),
    >,
    mut fullscreen_veils: Query<
        (Entity, &ChildOf, &mut BackgroundColor, &mut Visibility),
        (
            With<ModalFullscreenVeilLayer>,
            Without<ModalFullscreenBackdropLayer>,
            Without<ModalPanelBackdropLayer>,
            Without<ModalPanelSurface>,
        ),
    >,
    mut panel_surfaces: Query<
        (
            Entity,
            &ComputedNode,
            &GlobalTransform,
            &mut BackgroundColor,
        ),
        (
            With<ModalPanelSurface>,
            Without<ModalFullscreenBackdropLayer>,
            Without<ModalFullscreenVeilLayer>,
            Without<ModalPanelBackdropLayer>,
        ),
    >,
    mut panel_backdrops: Query<
        (Entity, &ChildOf, &mut ImageNode, &mut Node, &mut Visibility),
        (
            With<ModalPanelBackdropLayer>,
            Without<ModalFullscreenBackdropLayer>,
            Without<ModalFullscreenVeilLayer>,
            Without<ModalPanelOverlayLayer>,
            Without<ModalPanelSurface>,
        ),
    >,
    mut panel_overlays: Query<
        (Entity, &ChildOf, &mut BackgroundColor, &mut Visibility),
        (
            With<ModalPanelOverlayLayer>,
            Without<ModalFullscreenBackdropLayer>,
            Without<ModalFullscreenVeilLayer>,
            Without<ModalPanelBackdropLayer>,
            Without<ModalPanelSurface>,
        ),
    >,
) {
    let mut active_fullscreen = None;
    for (entity, child_of, _, _, mut visibility) in &mut fullscreen_layers {
        *visibility = Visibility::Hidden;
        if Some(child_of.parent()) == active.active_root {
            active_fullscreen = Some(entity);
        }
    }

    let mut active_veil = None;
    for (entity, child_of, _, mut visibility) in &mut fullscreen_veils {
        *visibility = Visibility::Hidden;
        if Some(child_of.parent()) == active.active_root {
            active_veil = Some(entity);
        }
    }

    let mut active_panel_backdrop = None;
    for (entity, child_of, _, _, mut visibility) in &mut panel_backdrops {
        *visibility = Visibility::Hidden;
        if Some(child_of.parent()) == active.active_panel {
            active_panel_backdrop = Some(entity);
        }
    }

    let mut active_panel_overlay = None;
    for (entity, child_of, _, mut visibility) in &mut panel_overlays {
        *visibility = Visibility::Hidden;
        if Some(child_of.parent()) == active.active_panel {
            active_panel_overlay = Some(entity);
        }
    }

    let (Some(active_root), Some(active_panel), Some(spec)) = (
        active.active_root,
        active.active_panel,
        active.active_spec.as_ref(),
    ) else {
        return;
    };

    let Ok((_, panel_computed, panel_transform, mut panel_background)) =
        panel_surfaces.get_mut(active_panel)
    else {
        return;
    };
    panel_background.0 = spec.panel_tint;

    let resolved = resolve_backdrop_images(&active, spec, &cache, &snapshot_state, &target);
    let layer_policy = backdrop_layer_policy(spec.effect);
    let viewport_size = window.resolution.size();

    if let Some(entity) = active_fullscreen {
        if let Some(handle) = if layer_policy.use_blurred_fullscreen {
            resolved.blurred.clone()
        } else {
            resolved.sharp.clone()
        } {
            if let Some(layout) = cover_layout_for_handle(&images, &handle, viewport_size) {
                if let Ok((_, child_of, mut image, mut node, mut visibility)) =
                    fullscreen_layers.get_mut(entity)
                {
                    if child_of.parent() == active_root && layer_policy.show_fullscreen_backdrop {
                        image.image = handle;
                        image.color = Color::WHITE;
                        image.rect = None;
                        apply_cover_layout_to_node(&mut node, layout);
                        *visibility = Visibility::Visible;
                    }
                }
            }
        }
    }

    if let Some(entity) = active_veil {
        if let Ok((_, child_of, mut background, mut visibility)) = fullscreen_veils.get_mut(entity)
        {
            if child_of.parent() == active_root
                && layer_policy.show_fullscreen_veil
                && spec.overlay_tint.to_srgba().alpha > 0.0
            {
                background.0 = spec.overlay_tint;
                *visibility = Visibility::Visible;
            }
        }
    }

    if let Some(entity) = active_panel_backdrop {
        if let Some(handle) = resolved.blurred {
            if let Some(layout) = cover_layout_for_handle(&images, &handle, viewport_size) {
                let panel_top_left = logical_node_top_left(panel_computed, panel_transform);
                if let Ok((_, child_of, mut image, mut node, mut visibility)) =
                    panel_backdrops.get_mut(entity)
                {
                    if child_of.parent() == active_panel && layer_policy.show_panel_backdrop {
                        image.image = handle;
                        image.color = Color::WHITE;
                        image.rect = None;
                        node.left = Val::Px(layout.left - panel_top_left.x);
                        node.top = Val::Px(layout.top - panel_top_left.y);
                        node.width = Val::Px(layout.width);
                        node.height = Val::Px(layout.height);
                        *visibility = Visibility::Visible;
                    }
                }
            }
        }
    }

    if let Some(entity) = active_panel_overlay {
        if let Ok((_, child_of, mut background, mut visibility)) = panel_overlays.get_mut(entity) {
            if child_of.parent() == active_panel && spec.panel_overlay_tint.to_srgba().alpha > 0.0 {
                background.0 = spec.panel_overlay_tint;
                *visibility = Visibility::Visible;
            }
        }
    }
}

fn choose_topmost_modal_candidate(
    candidates: impl IntoIterator<Item = ModalCandidate>,
) -> Option<ModalCandidate> {
    candidates
        .into_iter()
        .filter(|candidate| candidate.visible)
        .max_by_key(|candidate| candidate.global_z)
}

fn backdrop_layer_policy(effect: ModalBackdropEffect) -> BackdropLayerPolicy {
    match effect {
        ModalBackdropEffect::PanelFrosted => BackdropLayerPolicy {
            show_fullscreen_backdrop: true,
            use_blurred_fullscreen: false,
            show_fullscreen_veil: false,
            show_panel_backdrop: true,
        },
        ModalBackdropEffect::FullscreenBlur => BackdropLayerPolicy {
            show_fullscreen_backdrop: true,
            use_blurred_fullscreen: true,
            show_fullscreen_veil: true,
            show_panel_backdrop: false,
        },
    }
}

fn resolve_backdrop_images(
    active: &ActiveModalBackdropState,
    spec: &ModalBackdropSpec,
    cache: &ModalBackdropAssetCache,
    snapshot_state: &ModalWorldSnapshotState,
    target: &ModalWorldSnapshotTarget,
) -> ResolvedBackdropImages {
    match &spec.source {
        ModalBackdropSource::Asset(source) => ResolvedBackdropImages {
            sharp: Some(source.clone()),
            blurred: cache
                .blurred_by_asset
                .get(&blurred_asset_key(source, spec.blur_sigma))
                .cloned(),
        },
        ModalBackdropSource::WorldSnapshot => {
            let expected_key =
                world_snapshot_key(active.active_root, active.active_open_generation, target.size, spec.blur_sigma);
            if snapshot_state.ready_key == Some(expected_key) {
                ResolvedBackdropImages {
                    sharp: Some(target.image.clone()),
                    blurred: snapshot_state.blurred_handle.clone(),
                }
            } else {
                ResolvedBackdropImages {
                    sharp: None,
                    blurred: None,
                }
            }
        }
    }
}

fn blurred_asset_key(source: &Handle<Image>, blur_sigma: f32) -> BlurredAssetKey {
    BlurredAssetKey {
        asset_id: source.id(),
        sigma_bits: blur_sigma.to_bits(),
    }
}

fn cover_layout_for_handle(
    images: &Assets<Image>,
    handle: &Handle<Image>,
    viewport_size: Vec2,
) -> Option<CoverLayout> {
    let image = images.get(handle)?;
    Some(cover_layout(image.texture_descriptor.size, viewport_size))
}

fn cover_layout(image_size: Extent3d, viewport_size: Vec2) -> CoverLayout {
    let source_width = image_size.width.max(1) as f32;
    let source_height = image_size.height.max(1) as f32;
    let scale = (viewport_size.x / source_width)
        .max(viewport_size.y / source_height)
        .max(0.0001);
    let width = source_width * scale;
    let height = source_height * scale;

    CoverLayout {
        left: (viewport_size.x - width) * 0.5,
        top: (viewport_size.y - height) * 0.5,
        width,
        height,
        scale,
    }
}

fn apply_cover_layout_to_node(node: &mut Node, layout: CoverLayout) {
    node.left = Val::Px(layout.left);
    node.top = Val::Px(layout.top);
    node.width = Val::Px(layout.width);
    node.height = Val::Px(layout.height);
}

fn logical_node_top_left(computed: &ComputedNode, transform: &GlobalTransform) -> Vec2 {
    let center = transform.translation().truncate() * computed.inverse_scale_factor();
    center - ((computed.size() * computed.inverse_scale_factor()) * 0.5)
}
