#[test]
fn modal_cover_layout_keeps_matching_aspect_ratio() {
    let layout = cover_layout(
        Extent3d {
            width: 1600,
            height: 900,
            depth_or_array_layers: 1,
        },
        Vec2::new(1600.0, 900.0),
    );

    assert_eq!(layout.left, 0.0);
    assert_eq!(layout.top, 0.0);
    assert_eq!(layout.width, 1600.0);
    assert_eq!(layout.height, 900.0);
    assert_eq!(layout.scale, 1.0);
}

#[test]
fn modal_cover_layout_crops_wider_image_without_distortion() {
    let layout = cover_layout(
        Extent3d {
            width: 2048,
            height: 1024,
            depth_or_array_layers: 1,
        },
        Vec2::new(1600.0, 900.0),
    );

    assert!(layout.width >= 1600.0);
    assert_eq!(layout.height, 900.0);
    assert!(layout.left < 0.0);
    let aspect = layout.width / layout.height;
    assert!((aspect - 2.0).abs() < 0.0001);
}

#[test]
fn modal_cover_layout_crops_taller_image_without_distortion() {
    let layout = cover_layout(
        Extent3d {
            width: 900,
            height: 1600,
            depth_or_array_layers: 1,
        },
        Vec2::new(1600.0, 900.0),
    );

    assert_eq!(layout.width, 1600.0);
    assert!(layout.height >= 900.0);
    assert!(layout.top < 0.0);
    let aspect = layout.width / layout.height;
    assert!((aspect - (900.0 / 1600.0)).abs() < 0.0001);
}

#[test]
fn modal_topmost_visible_root_uses_highest_global_z() {
    let low = ModalCandidate {
        entity: Entity::from_raw(1),
        visible: true,
        global_z: 10,
    };
    let hidden = ModalCandidate {
        entity: Entity::from_raw(2),
        visible: false,
        global_z: 900,
    };
    let high = ModalCandidate {
        entity: Entity::from_raw(3),
        visible: true,
        global_z: 250,
    };

    let picked = choose_topmost_modal_candidate([low, hidden, high]).expect("visible modal");
    assert_eq!(picked.entity, high.entity);
}

#[test]
fn modal_panel_frosted_policy_uses_only_local_blur() {
    let policy = backdrop_layer_policy(ModalBackdropEffect::PanelFrosted);

    assert!(policy.show_fullscreen_backdrop);
    assert!(!policy.use_blurred_fullscreen);
    assert!(!policy.show_fullscreen_veil);
    assert!(policy.show_panel_backdrop);
}

#[test]
fn modal_fullscreen_blur_policy_uses_only_screen_blur() {
    let policy = backdrop_layer_policy(ModalBackdropEffect::FullscreenBlur);

    assert!(policy.show_fullscreen_backdrop);
    assert!(policy.use_blurred_fullscreen);
    assert!(policy.show_fullscreen_veil);
    assert!(!policy.show_panel_backdrop);
}

#[test]
fn modal_backdrop_spec_defaults_to_no_panel_overlay_tint() {
    let spec = ModalBackdropSpec::panel_frosted(
        ModalBackdropSource::WorldSnapshot,
        MAIN_MENU_PANEL_TINT,
    );

    assert_eq!(spec.panel_overlay_tint, Color::NONE);
}

#[test]
fn modal_backdrop_spec_accepts_custom_panel_overlay_tint() {
    let tint = Color::srgba(0.96, 0.97, 1.0, 0.18);
    let spec = ModalBackdropSpec::panel_frosted(
        ModalBackdropSource::WorldSnapshot,
        MAIN_MENU_PANEL_TINT,
    )
    .with_panel_overlay_tint(tint);

    assert_eq!(spec.panel_overlay_tint, tint);
}

#[test]
fn modal_snapshot_capture_is_requested_only_once_for_same_open_generation() {
    let request = WorldSnapshotRequest {
        key: WorldSnapshotKey {
            root: Entity::from_raw(1),
            open_generation: 1,
            capture_size: UVec2::new(1600, 900),
            sigma_bits: FULLSCREEN_BLUR_SIGMA.to_bits(),
        },
        capture_size: UVec2::new(1600, 900),
        blur_sigma_bits: FULLSCREEN_BLUR_SIGMA.to_bits(),
    };

    assert!(should_request_world_snapshot_capture(
        ModalWorldSnapshotPhase::Idle,
        None,
        None,
        Some(request),
    ));
    assert!(!should_request_world_snapshot_capture(
        ModalWorldSnapshotPhase::Idle,
        Some(request.key),
        None,
        Some(request),
    ));
    assert!(!should_request_world_snapshot_capture(
        ModalWorldSnapshotPhase::Capturing,
        None,
        Some(request),
        Some(request),
    ));
}

#[test]
fn modal_snapshot_capture_refreshes_for_new_open_generation_or_resize() {
    let base = WorldSnapshotRequest {
        key: WorldSnapshotKey {
            root: Entity::from_raw(7),
            open_generation: 2,
            capture_size: UVec2::new(1600, 900),
            sigma_bits: FULLSCREEN_BLUR_SIGMA.to_bits(),
        },
        capture_size: UVec2::new(1600, 900),
        blur_sigma_bits: FULLSCREEN_BLUR_SIGMA.to_bits(),
    };
    let reopened = WorldSnapshotRequest {
        key: WorldSnapshotKey {
            open_generation: 3,
            ..base.key
        },
        ..base
    };
    let resized = WorldSnapshotRequest {
        key: WorldSnapshotKey {
            capture_size: UVec2::new(1920, 1080),
            ..base.key
        },
        capture_size: UVec2::new(1920, 1080),
        ..base
    };

    assert!(should_request_world_snapshot_capture(
        ModalWorldSnapshotPhase::Idle,
        Some(base.key),
        None,
        Some(reopened),
    ));
    assert!(should_request_world_snapshot_capture(
        ModalWorldSnapshotPhase::Idle,
        Some(base.key),
        None,
        Some(resized),
    ));
}

#[test]
fn modal_blur_rejects_mipmapped_source_images() {
    let mut image = Image::new_fill(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = 2;

    let error = build_blurred_backdrop_image(&image, 1.0).expect_err("mipmapped image must fail");
    assert!(error.contains("mipmapped textures are not supported"));
}
