use std::collections::HashMap;

use bevy::{
    asset::AssetId,
    prelude::*,
    render::{
        camera::{Projection, RenderTarget},
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    window::{PrimaryWindow, WindowResized},
};

use crate::input::camera::MainCamera;

pub const PANEL_FROSTED_BLUR_SIGMA: f32 = 10.0;
pub const FULLSCREEN_BLUR_SIGMA: f32 = 6.0;
pub const MAIN_MENU_PANEL_TINT: Color = Color::srgba(0.96, 0.96, 0.97, 0.78);
pub const MAIN_MENU_PANEL_OVERLAY_TINT: Color = Color::srgba(0.96, 0.97, 1.0, 0.18);
pub const IN_GAME_MENU_PANEL_TINT: Color = Color::srgba(0.96, 0.96, 0.97, 0.84);
pub const FULLSCREEN_BLUR_OVERLAY_TINT: Color = Color::srgba(0.02, 0.02, 0.03, 0.18);
const MODAL_PANEL_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.42);
const MODAL_PANEL_SHADOW_BLUR_RADIUS: f32 = 28.0;
const MODAL_SNAPSHOT_SETTLE_FRAMES: u8 = 1;

/// Selects how the modal should treat the backdrop behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModalBackdropEffect {
    PanelFrosted,
    FullscreenBlur,
}

/// Describes where the modal backdrop image should come from.
#[derive(Clone, Debug)]
pub enum ModalBackdropSource {
    Asset(Handle<Image>),
    WorldSnapshot,
}

/// Configures the reusable backdrop, shell tint and overlay tint for a modal.
#[derive(Component, Clone, Debug)]
pub struct ModalBackdropSpec {
    pub effect: ModalBackdropEffect,
    pub source: ModalBackdropSource,
    pub blur_sigma: f32,
    pub overlay_tint: Color,
    pub panel_tint: Color,
    pub panel_overlay_tint: Color,
}

impl ModalBackdropSpec {
    /// Builds the spec for a panel-local frosted modal.
    pub fn panel_frosted(source: ModalBackdropSource, panel_tint: Color) -> Self {
        Self {
            effect: ModalBackdropEffect::PanelFrosted,
            source,
            blur_sigma: PANEL_FROSTED_BLUR_SIGMA,
            overlay_tint: Color::NONE,
            panel_tint,
            panel_overlay_tint: Color::NONE,
        }
    }

    /// Builds the spec for a fullscreen blurred modal.
    pub fn fullscreen_blur(source: ModalBackdropSource, panel_tint: Color) -> Self {
        Self {
            effect: ModalBackdropEffect::FullscreenBlur,
            source,
            blur_sigma: FULLSCREEN_BLUR_SIGMA,
            overlay_tint: FULLSCREEN_BLUR_OVERLAY_TINT,
            panel_tint,
            panel_overlay_tint: Color::NONE,
        }
    }

    /// Adds an optional tint layer above the panel blur and below modal content.
    pub fn with_panel_overlay_tint(mut self, panel_overlay_tint: Color) -> Self {
        self.panel_overlay_tint = panel_overlay_tint;
        self
    }
}

/// Marks the root node of a reusable modal.
#[derive(Component)]
pub struct ModalRoot;

/// Marks the clipped shell surface that hosts the modal backdrop and content.
#[derive(Component)]
pub struct ModalPanelSurface;

/// Opts a modal root into the shared backdrop runtime.
#[derive(Component, Default)]
pub struct ModalBackdropController;

#[derive(Component)]
struct ModalFullscreenBackdropLayer;

#[derive(Component)]
struct ModalFullscreenVeilLayer;

#[derive(Component)]
struct ModalPanelBackdropLayer;

#[derive(Component)]
struct ModalPanelOverlayLayer;

#[derive(Component)]
struct ModalWorldSnapshotCamera;

#[derive(Clone, Copy, Debug, PartialEq)]
struct CoverLayout {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModalCandidate {
    entity: Entity,
    visible: bool,
    global_z: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BackdropLayerPolicy {
    show_fullscreen_backdrop: bool,
    use_blurred_fullscreen: bool,
    show_fullscreen_veil: bool,
    show_panel_backdrop: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BlurredAssetKey {
    asset_id: AssetId<Image>,
    sigma_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WorldSnapshotKey {
    root: Entity,
    open_generation: u64,
    capture_size: UVec2,
    sigma_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorldSnapshotRequest {
    key: WorldSnapshotKey,
    capture_size: UVec2,
    blur_sigma_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum ModalWorldSnapshotPhase {
    #[default]
    Idle,
    Settling,
    Armed,
    Capturing,
}

#[derive(Clone, Debug)]
struct ResolvedBackdropImages {
    sharp: Option<Handle<Image>>,
    blurred: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct ModalBackdropAssetCache {
    blurred_by_asset: HashMap<BlurredAssetKey, Handle<Image>>,
}

#[derive(Resource, Default)]
struct ModalOpenTracker {
    previous_active_root: Option<Entity>,
    open_generation: u64,
}

#[derive(Resource, Default)]
struct ActiveModalBackdropState {
    active_root: Option<Entity>,
    active_panel: Option<Entity>,
    active_spec: Option<ModalBackdropSpec>,
    active_global_z: i32,
    active_open_generation: u64,
}

#[derive(Resource, Clone)]
struct ModalWorldSnapshotTarget {
    image: Handle<Image>,
    size: UVec2,
}

#[derive(Resource, Default)]
struct ModalWorldSnapshotState {
    ready_key: Option<WorldSnapshotKey>,
    pending_request: Option<WorldSnapshotRequest>,
    blurred_handle: Option<Handle<Image>>,
    settle_frames_remaining: u8,
    phase: ModalWorldSnapshotPhase,
}

#[derive(Event, Clone, Debug)]
struct ModalWorldSnapshotCaptureFinished {
    key: WorldSnapshotKey,
    result: Result<Handle<Image>, String>,
}

/// Returns the shared outer shadow for reusable modal panel shells.
pub fn modal_panel_box_shadow() -> BoxShadow {
    BoxShadow::new(
        MODAL_PANEL_SHADOW_COLOR,
        Val::Px(0.0),
        Val::Px(0.0),
        Val::Px(0.0),
        Val::Px(MODAL_PANEL_SHADOW_BLUR_RADIUS),
    )
}

/// Spawns the fullscreen backdrop chrome for a modal root.
pub fn spawn_modal_backdrop_chrome(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        Visibility::Hidden,
        ModalFullscreenBackdropLayer,
    ));
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
        ModalFullscreenVeilLayer,
    ));
}

/// Spawns the clipped panel chrome layers for a modal panel surface.
pub fn spawn_modal_panel_backdrop(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        Visibility::Hidden,
        ModalPanelBackdropLayer,
    ));
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
        ModalPanelOverlayLayer,
    ));
}

/// Wires the reusable modal backdrop runtime into the UI app.
pub struct ModalPlugin;

impl Plugin for ModalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModalBackdropAssetCache>()
            .init_resource::<ModalOpenTracker>()
            .init_resource::<ActiveModalBackdropState>()
            .init_resource::<ModalWorldSnapshotState>()
            .add_event::<ModalWorldSnapshotCaptureFinished>()
            .add_systems(Startup, setup_modal_world_snapshot_camera)
            .add_systems(
                Update,
                (
                    update_active_modal_backdrop_state,
                    prepare_asset_backdrop_blur_cache,
                    resize_modal_world_snapshot_target,
                    request_modal_world_snapshot_capture,
                    advance_modal_world_snapshot_settle_frame,
                    spawn_modal_world_snapshot_screenshot,
                    finish_modal_world_snapshot_capture,
                    sync_modal_backdrop_visuals,
                )
                    .chain(),
            );
    }
}

include!("modal_runtime_block.rs");
include!("modal_capture_block.rs");

#[cfg(test)]
include!("modal_tests_block.rs");
