use std::{fs, path::Path};

use bevy::{
    prelude::*,
    render::{
        camera::{OrthographicProjection, Projection, RenderTarget, ScalingMode},
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    ui::UiTargetCamera,
};

use crate::{
    editor::StructureEditState,
    input::camera::MainCamera,
    save::{patch_save_preview_meta, SavePreviewCaptureFinished, SavePreviewQueueState, SavePreviewRequest},
};

use super::{world_view::preview_world_extent, world_view::WORLD_PREVIEW_TARGET_SIZE_PX, OverlayMode};

#[derive(Component)]
struct SavePreviewCamera;

#[derive(Resource, Clone)]
struct SavePreviewRenderTarget {
    image: Handle<Image>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SavePreviewPhase {
    #[default]
    Idle,
    Armed,
    Capturing,
}

#[derive(Resource, Default)]
struct SavePreviewRuntimeState {
    active_request: Option<SavePreviewRequest>,
    overlay_before_capture: Option<OverlayMode>,
    selected_cell_before_capture: Option<UVec2>,
    phase: SavePreviewPhase,
}

fn setup_save_preview_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: WORLD_PREVIEW_TARGET_SIZE_PX,
            height: WORLD_PREVIEW_TARGET_SIZE_PX,
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
    let image_handle = images.add(image);

    let preview_extent = preview_world_extent();
    commands.spawn((
        Camera2d,
        Camera {
            target: RenderTarget::Image(image_handle.clone().into()),
            is_active: false,
            order: -100,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: preview_extent.x,
                height: preview_extent.y,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::default(),
        SavePreviewCamera,
    ));

    commands.insert_resource(SavePreviewRenderTarget { image: image_handle });
}

fn attach_ui_target_camera_to_root_nodes(
    mut commands: Commands,
    main_camera: Single<Entity, With<MainCamera>>,
    root_nodes: Query<
        Entity,
        (
            Added<Node>,
            Without<bevy::ecs::hierarchy::ChildOf>,
            Without<UiTargetCamera>,
        ),
    >,
) {
    for entity in &root_nodes {
        commands.entity(entity).insert(UiTargetCamera(*main_camera));
    }
}

fn arm_save_preview_capture(
    mut preview_queue: ResMut<SavePreviewQueueState>,
    mut runtime: ResMut<SavePreviewRuntimeState>,
    mut overlay_mode: ResMut<OverlayMode>,
    mut structure_edit: ResMut<StructureEditState>,
    mut preview_camera: Single<(&mut Camera, &mut Projection, &mut Transform), With<SavePreviewCamera>>,
) {
    if runtime.active_request.is_some() {
        return;
    }
    let Some(request) = preview_queue.pending.take() else {
        return;
    };

    let (camera, projection, transform) = &mut *preview_camera;
    let preview_extent = preview_world_extent();
    **projection = Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: preview_extent.x,
            height: preview_extent.y,
        },
        ..OrthographicProjection::default_2d()
    });
    **transform = Transform::default();
    camera.is_active = true;

    preview_queue.active = true;
    runtime.overlay_before_capture = Some(*overlay_mode);
    runtime.selected_cell_before_capture = structure_edit.selected_cell;
    *overlay_mode = OverlayMode::Main;
    structure_edit.selected_cell = None;
    runtime.phase = SavePreviewPhase::Armed;
    runtime.active_request = Some(request);
}

fn spawn_save_preview_screenshot(
    mut commands: Commands,
    target: Res<SavePreviewRenderTarget>,
    mut runtime: ResMut<SavePreviewRuntimeState>,
) {
    if runtime.phase != SavePreviewPhase::Armed {
        return;
    }
    let Some(request) = runtime.active_request.clone() else {
        runtime.phase = SavePreviewPhase::Idle;
        return;
    };

    commands
        .spawn(Screenshot::image(target.image.clone()))
        .observe(
            move |trigger: Trigger<ScreenshotCaptured>,
                  mut preview_queue: ResMut<SavePreviewQueueState>,
                  mut finish_events: EventWriter<SavePreviewCaptureFinished>| {
                let mut result = write_preview_png(&trigger.event().0, &request.target_path);
                if result.is_ok() && request.patch_meta_on_success {
                    result = patch_save_preview_meta(&request.root, &request.descriptor.id)
                        .map(|_| ())
                        .map_err(|err| err.to_string());
                }
                preview_queue.active = false;
                finish_events.write(SavePreviewCaptureFinished {
                    request: request.clone(),
                    result,
                });
            },
        );
    runtime.phase = SavePreviewPhase::Capturing;
}

fn restore_after_save_preview_capture(
    mut finished: EventReader<SavePreviewCaptureFinished>,
    mut runtime: ResMut<SavePreviewRuntimeState>,
    mut overlay_mode: ResMut<OverlayMode>,
    mut structure_edit: ResMut<StructureEditState>,
    mut preview_camera: Single<&mut Camera, With<SavePreviewCamera>>,
) {
    if finished.is_empty() {
        return;
    }

    for _ in finished.read() {
        if runtime.active_request.is_none() {
            continue;
        }
        if let Some(previous_overlay_mode) = runtime.overlay_before_capture.take() {
            *overlay_mode = previous_overlay_mode;
        }
        structure_edit.selected_cell = runtime.selected_cell_before_capture.take();
        preview_camera.is_active = false;
        runtime.phase = SavePreviewPhase::Idle;
        runtime.active_request = None;
    }
}

fn write_preview_png(captured: &Image, path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "Preview path '{}' has no parent directory",
            path.display()
        ));
    };
    fs::create_dir_all(parent).map_err(|err| {
        format!(
            "Failed to create preview directory '{}': {}",
            parent.display(),
            err
        )
    })?;

    let dynamic = captured
        .clone()
        .try_into_dynamic()
        .map_err(|err| format!("Failed to convert preview image to PNG buffer: {}", err))?;
    dynamic
        .to_rgba8()
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|err| format!("Failed to write preview PNG '{}': {}", path.display(), err))
}

pub(super) struct SavePreviewPlugin;

impl Plugin for SavePreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SavePreviewQueueState>()
            .init_resource::<SavePreviewRuntimeState>()
            .add_event::<SavePreviewCaptureFinished>()
            .add_systems(Startup, setup_save_preview_camera)
            .add_systems(
                Update,
                (
                    attach_ui_target_camera_to_root_nodes,
                    arm_save_preview_capture,
                    spawn_save_preview_screenshot,
                    restore_after_save_preview_capture,
                )
                    .chain(),
            );
    }
}
