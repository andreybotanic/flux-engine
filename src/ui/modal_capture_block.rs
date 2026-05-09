fn setup_modal_world_snapshot_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let size = snapshot_target_size(&window);
    let image_handle = images.add(build_snapshot_target_image(size));

    commands.insert_resource(ModalWorldSnapshotTarget {
        image: image_handle.clone(),
        size,
    });
    commands.spawn((
        Camera2d,
        Camera {
            target: RenderTarget::Image(image_handle.into()),
            is_active: false,
            order: -90,
            ..default()
        },
        Projection::default(),
        Transform::default(),
        ModalWorldSnapshotCamera,
    ));
}

fn resize_modal_world_snapshot_target(
    mut resize_events: EventReader<WindowResized>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut target: ResMut<ModalWorldSnapshotTarget>,
    mut capture_camera: Single<&mut Camera, With<ModalWorldSnapshotCamera>>,
    mut snapshot_state: ResMut<ModalWorldSnapshotState>,
) {
    let desired_size = snapshot_target_size(&window);
    if resize_events.is_empty() && target.size == desired_size {
        return;
    }
    resize_events.clear();
    if target.size == desired_size {
        return;
    }

    let image_handle = images.add(build_snapshot_target_image(desired_size));
    target.image = image_handle.clone();
    target.size = desired_size;
    capture_camera.target = RenderTarget::Image(image_handle.into());
    snapshot_state.ready_key = None;
    snapshot_state.pending_request = None;
    snapshot_state.blurred_handle = None;
    snapshot_state.settle_frames_remaining = 0;
    snapshot_state.phase = ModalWorldSnapshotPhase::Idle;
}

fn request_modal_world_snapshot_capture(
    active: Res<ActiveModalBackdropState>,
    target: Res<ModalWorldSnapshotTarget>,
    mut snapshot_state: ResMut<ModalWorldSnapshotState>,
    main_camera: Single<
        (&Camera, &Projection, &Transform),
        (With<MainCamera>, Without<ModalWorldSnapshotCamera>),
    >,
    mut capture_camera: Single<
        (&mut Camera, &mut Projection, &mut Transform),
        (With<ModalWorldSnapshotCamera>, Without<MainCamera>),
    >,
) {
    let desired_request = desired_world_snapshot_request(&active, target.size);
    if !should_request_world_snapshot_capture(
        snapshot_state.phase,
        snapshot_state.ready_key,
        snapshot_state.pending_request,
        desired_request,
    ) {
        return;
    }

    let Some(request) = desired_request else {
        return;
    };

    let (main_camera, main_projection, main_transform) = *main_camera;
    let (capture_camera, capture_projection, capture_transform) = &mut *capture_camera;
    capture_camera.target = RenderTarget::Image(target.image.clone().into());
    capture_camera.viewport = main_camera.viewport.clone();
    capture_camera.is_active = true;
    **capture_projection = main_projection.clone();
    **capture_transform = main_transform.clone();

    snapshot_state.ready_key = None;
    snapshot_state.blurred_handle = None;
    snapshot_state.pending_request = Some(request);
    snapshot_state.settle_frames_remaining = MODAL_SNAPSHOT_SETTLE_FRAMES;
    snapshot_state.phase = ModalWorldSnapshotPhase::Settling;
}

fn advance_modal_world_snapshot_settle_frame(mut snapshot_state: ResMut<ModalWorldSnapshotState>) {
    if snapshot_state.phase != ModalWorldSnapshotPhase::Settling {
        return;
    }
    if snapshot_state.pending_request.is_none() {
        snapshot_state.phase = ModalWorldSnapshotPhase::Idle;
        snapshot_state.settle_frames_remaining = 0;
        return;
    }
    if snapshot_state.settle_frames_remaining > 0 {
        snapshot_state.settle_frames_remaining -= 1;
        return;
    }
    snapshot_state.phase = ModalWorldSnapshotPhase::Armed;
}

fn spawn_modal_world_snapshot_screenshot(
    mut commands: Commands,
    target: Res<ModalWorldSnapshotTarget>,
    mut snapshot_state: ResMut<ModalWorldSnapshotState>,
) {
    if snapshot_state.phase != ModalWorldSnapshotPhase::Armed {
        return;
    }
    let Some(request) = snapshot_state.pending_request else {
        snapshot_state.phase = ModalWorldSnapshotPhase::Idle;
        return;
    };

    commands
        .spawn(Screenshot::image(target.image.clone()))
        .observe(
            move |trigger: Trigger<ScreenshotCaptured>,
                  mut images: ResMut<Assets<Image>>,
                  mut finished: EventWriter<ModalWorldSnapshotCaptureFinished>| {
                let sigma = f32::from_bits(request.blur_sigma_bits);
                let result = build_blurred_backdrop_image(&trigger.event().0, sigma)
                    .map(|blurred| images.add(blurred));
                finished.write(ModalWorldSnapshotCaptureFinished {
                    key: request.key,
                    result,
                });
            },
        );
    snapshot_state.phase = ModalWorldSnapshotPhase::Capturing;
}

fn finish_modal_world_snapshot_capture(
    mut finished: EventReader<ModalWorldSnapshotCaptureFinished>,
    mut snapshot_state: ResMut<ModalWorldSnapshotState>,
    mut capture_camera: Single<&mut Camera, With<ModalWorldSnapshotCamera>>,
) {
    if finished.is_empty() {
        return;
    }

    for event in finished.read() {
        capture_camera.is_active = false;
        snapshot_state.settle_frames_remaining = 0;
        snapshot_state.phase = ModalWorldSnapshotPhase::Idle;
        let pending = snapshot_state.pending_request.take();
        if pending.map(|request| request.key) != Some(event.key) {
            continue;
        }
        match &event.result {
            Ok(handle) => {
                snapshot_state.ready_key = Some(event.key);
                snapshot_state.blurred_handle = Some(handle.clone());
            }
            Err(err) => {
                error!("Modal world snapshot blur failed: {err}");
                snapshot_state.ready_key = None;
                snapshot_state.blurred_handle = None;
            }
        }
    }
}

fn desired_world_snapshot_request(
    active: &ActiveModalBackdropState,
    capture_size: UVec2,
) -> Option<WorldSnapshotRequest> {
    let spec = active.active_spec.as_ref()?;
    if !matches!(spec.source, ModalBackdropSource::WorldSnapshot) {
        return None;
    }

    Some(WorldSnapshotRequest {
        key: world_snapshot_key(
            active.active_root,
            active.active_open_generation,
            capture_size,
            spec.blur_sigma,
        ),
        capture_size,
        blur_sigma_bits: spec.blur_sigma.to_bits(),
    })
}

fn should_request_world_snapshot_capture(
    phase: ModalWorldSnapshotPhase,
    ready_key: Option<WorldSnapshotKey>,
    pending_request: Option<WorldSnapshotRequest>,
    desired_request: Option<WorldSnapshotRequest>,
) -> bool {
    let Some(desired_request) = desired_request else {
        return false;
    };
    phase == ModalWorldSnapshotPhase::Idle
        && ready_key != Some(desired_request.key)
        && pending_request != Some(desired_request)
}

fn world_snapshot_key(
    root: Option<Entity>,
    open_generation: u64,
    capture_size: UVec2,
    blur_sigma: f32,
) -> WorldSnapshotKey {
    WorldSnapshotKey {
        root: root.expect("world snapshot key requires an active modal root"),
        open_generation,
        capture_size,
        sigma_bits: blur_sigma.to_bits(),
    }
}

fn snapshot_target_size(window: &Window) -> UVec2 {
    UVec2::new(window.physical_width().max(1), window.physical_height().max(1))
}

fn build_snapshot_target_image(size: UVec2) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC
        | TextureUsages::RENDER_ATTACHMENT;
    image
}

fn build_blurred_backdrop_image(source: &Image, blur_sigma: f32) -> Result<Image, String> {
    let dynamic = source
        .clone()
        .try_into_dynamic()
        .map_err(|err| format!("Failed to convert backdrop image for blur: {err}"))?;
    Ok(Image::from_dynamic(
        dynamic.blur(blur_sigma),
        true,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}
