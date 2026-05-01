use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

use crate::world::grid::{world_dimensions, CAMERA_MARGIN};

#[derive(Component)]
pub struct MainCamera;

pub fn spawn_main_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

pub fn camera_pan_zoom(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&mut Projection, &mut Transform), With<MainCamera>>,
) {
    let (mut projection, mut transform) = camera_query.into_inner();

    let mut zoom_delta: f32 = 0.0;
    for event in mouse_wheel.read() {
        zoom_delta += event.y;
    }

    let mut scale = 1.0;
    if let Projection::Orthographic(orthographic) = &mut *projection {
        if zoom_delta.abs() > f32::EPSILON {
            orthographic.scale = (orthographic.scale * 0.9_f32.powf(zoom_delta)).clamp(0.35, 6.0);
        }
        scale = orthographic.scale;
    }

    if mouse_buttons.pressed(MouseButton::Middle) {
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

fn clamp_camera_to_world(transform: &mut Transform, scale: f32, window_size: Vec2) {
    let world_size = world_dimensions();
    let half_window = window_size * 0.5 * scale;
    let half_world = world_size * 0.5;

    let limit_x = (half_world.x + CAMERA_MARGIN - half_window.x).max(0.0);
    let limit_y = (half_world.y + CAMERA_MARGIN - half_window.y).max(0.0);

    transform.translation.x = transform.translation.x.clamp(-limit_x, limit_x);
    transform.translation.y = transform.translation.y.clamp(-limit_y, limit_y);
}
