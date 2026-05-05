use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

use crate::ui::panels::PanelManager;
use crate::world::grid::{world_dimensions, CAMERA_MARGIN};

#[derive(Component)]
/// Stores `MainCamera` state.
pub struct MainCamera;

/// Runs `spawn_main_camera` logic.
pub fn spawn_main_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

/// Runs `camera_pan_zoom` logic.
pub fn camera_pan_zoom(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&mut Projection, &mut Transform), With<MainCamera>>,
    panels: Option<Res<PanelManager>>,
) {
    let (mut projection, mut transform) = camera_query.into_inner();
    let cursor = window.cursor_position();
    let blocked_by_panel = cursor
        .and_then(|cursor| {
            panels
                .as_ref()
                .map(|manager| manager.is_cursor_over_any_panel(cursor))
        })
        .unwrap_or(false);

    let mut zoom_delta: f32 = 0.0;
    for event in mouse_wheel.read() {
        if !blocked_by_panel {
            zoom_delta += event.y;
        }
    }

    let mut scale = 1.0;
    if let Projection::Orthographic(orthographic) = &mut *projection {
        if zoom_delta.abs() > f32::EPSILON {
            let prev_scale = orthographic.scale;
            let next_scale = (orthographic.scale * 0.9_f32.powf(zoom_delta)).clamp(0.12, 8.0);
            if let Some(cursor_pos) = window.cursor_position() {
                anchor_zoom_to_cursor(
                    &mut transform.translation,
                    cursor_pos,
                    window.resolution.size(),
                    prev_scale,
                    next_scale,
                );
            }
            orthographic.scale = next_scale;
        }
        scale = orthographic.scale;
    }

    if mouse_buttons.pressed(MouseButton::Middle) && !blocked_by_panel {
        let mut delta = Vec2::ZERO;
        for event in mouse_motion.read() {
            delta += event.delta;
        }

        transform.translation.x -= delta.x * scale;
        transform.translation.y += delta.y * scale;
    } else {
        mouse_motion.clear();
    }

    clamp_camera_to_world(&mut transform, scale, window.resolution.size());
}

fn anchor_zoom_to_cursor(
    translation: &mut Vec3,
    cursor: Vec2,
    window_size: Vec2,
    old_scale: f32,
    new_scale: f32,
) {
    let delta_scale = old_scale - new_scale;
    let half = window_size * 0.5;
    let offset_x = cursor.x - half.x;
    let offset_y = half.y - cursor.y;
    translation.x += offset_x * delta_scale;
    translation.y += offset_y * delta_scale;
}

fn clamp_camera_to_world(transform: &mut Transform, scale: f32, window_size: Vec2) {
    let world_size = world_dimensions();
    let half_window = window_size * 0.5 * scale;
    let half_world = world_size * 0.5;

    let limit_x = (half_world.x + CAMERA_MARGIN - half_window.x).max(0.0);
    let limit_y = (half_world.y + CAMERA_MARGIN - half_window.y).max(0.0);

    transform.translation.x = transform.translation.x.clamp(-limit_x, limit_x);
    transform.translation.y = transform.translation.y.clamp(-limit_y, limit_y);
}

#[cfg(test)]
mod tests {
    use super::anchor_zoom_to_cursor;
    use bevy::prelude::*;

    #[test]
    fn zoom_anchor_keeps_world_point_under_cursor() {
        let mut translation = Vec3::new(0.0, 0.0, 0.0);
        let cursor = Vec2::new(1200.0, 700.0);
        let window = Vec2::new(1600.0, 900.0);
        let old_scale = 1.5;
        let new_scale = 0.75;

        let half = window * 0.5;
        let offset = Vec2::new(cursor.x - half.x, half.y - cursor.y);
        let world_before = translation.truncate() + offset * old_scale;
        anchor_zoom_to_cursor(&mut translation, cursor, window, old_scale, new_scale);
        let world_after = translation.truncate() + offset * new_scale;

        assert!(
            world_before.abs_diff_eq(world_after, 1e-4),
            "World point under cursor should stay fixed after zoom"
        );
    }
}
