use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::components::{MainCamera, Shuttle};
use crate::resources::{ShuttlePosition, Throttle, MouseLook};

const CAMERA_DISTANCE: f32 = 600.0;
const CAMERA_HEIGHT: f32 = 200.0;
const MAX_YAW_RADIANS: f32 = 0.8; // how far camera can yaw from center
const MAX_PITCH_RADIANS: f32 = 0.4;
const THROTTLE_RATE: f32 = 1.0; // units per second
const MAX_THROTTLE: f32 = 4.0;
const MIN_THROTTLE: f32 = 0.0;

pub fn shuttle_control_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_info: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    time: Res<Time>,
    mut shuttle_query: Query<&mut Transform, With<Shuttle>>,
    mut shuttle_pos: ResMut<ShuttlePosition>,
    mut throttle: ResMut<Throttle>,
    keyboard: Res<Input<KeyCode>>,
) {
    let Ok(mut transform) = shuttle_query.get_single_mut() else { return };
    let Ok((cam, cam_global)) = camera_info.get_single() else { return };

    // throttle control (W accelerate, S decelerate)
    let delta = time.delta_seconds();
    if keyboard.pressed(KeyCode::W) {
        throttle.0 += THROTTLE_RATE * delta;
    }
    if keyboard.pressed(KeyCode::S) {
        throttle.0 -= THROTTLE_RATE * delta;
    }
    throttle.0 = throttle.0.clamp(MIN_THROTTLE, MAX_THROTTLE);

    // vertical movement: Space = up, LShift = down (relative world Y)
    let vertical_speed = crate::SHUTTLE_SPEED * 0.5;
    if keyboard.pressed(KeyCode::Space) {
        transform.translation.y += vertical_speed * delta;
        shuttle_pos.0 = transform.translation;
    }
    if keyboard.pressed(KeyCode::C) {
        transform.translation.y -= vertical_speed * delta;
        shuttle_pos.0 = transform.translation;
    }

    // determine desired heading from a ray through the screen center (cursor fixed)
    if let Ok(window) = windows.get_single() {
        let screen_pos = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        if let Some(ray) = cam.viewport_to_world(cam_global, screen_pos) {
            let origin = ray.origin;
            let dir = ray.direction;
            let plane_y = transform.translation.y;
            if dir.y.abs() > 1e-6 {
                let t = (plane_y - origin.y) / dir.y;
                if t > 0.0 {
                    let world_pos = origin + dir * t;
                    let desired = (world_pos - transform.translation).normalize_or_zero();
                    if desired.length_squared() > 0.0 {
                        let target_pos = transform.translation + desired;
                        transform.look_at(target_pos, Vec3::Y);
                        transform.translation += desired * crate::SHUTTLE_SPEED * throttle.0 * delta;
                        shuttle_pos.0 = transform.translation;
                    }
                }
            }
        }
    }
}

pub fn camera_follow_system(
    mut cam_query: Query<&mut Transform, With<MainCamera>>,
    shuttle_pos: Res<ShuttlePosition>,
    mouse_look: Res<MouseLook>,
    time: Res<Time>,
) {
    let Ok(mut cam_transform) = cam_query.get_single_mut() else { return };

    let yaw = mouse_look.yaw;
    let pitch = mouse_look.pitch;
    let rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

    let base_offset = Vec3::new(0.0, CAMERA_HEIGHT, CAMERA_DISTANCE);
    let target = shuttle_pos.0 + rot * base_offset;
    let lerp_t = (time.delta_seconds() * 6.0).min(1.0);
    cam_transform.translation = cam_transform.translation.lerp(target, lerp_t);
    cam_transform.look_at(shuttle_pos.0, Vec3::Y);
}
