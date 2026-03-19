use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::components::Shuttle;
use crate::resources::ShuttlePosition;

pub fn shuttle_steer_and_move_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Shuttle>>,
    mut shuttle_pos: ResMut<ShuttlePosition>,
) {
    let Ok(mut transform) = query.get_single_mut() else { return };

    let window: &Window = match windows.get_single() {
        Ok(w) => w,
        Err(_) => return,
    };

    let (camera, camera_transform) = match cameras.iter().next() {
        Some(pair) => pair,
        None => return,
    };

    if let Some(screen_pos) = window.cursor_position() {
        if let Some(ray) = camera.viewport_to_world(camera_transform, screen_pos) {
            let world_pos = ray.origin.truncate();
            let shuttle_pos_local = transform.translation.truncate();
            let mut dir = world_pos - shuttle_pos_local;
            if dir.length_squared() > 0.0 {
                dir = dir.normalize();
                let angle = dir.y.atan2(dir.x);
                transform.rotation = Quat::from_rotation_z(angle);
                transform.translation += dir.extend(0.0) * super::super::SHUTTLE_SPEED * time.delta_seconds();
                shuttle_pos.0 = transform.translation.truncate();
            }
        }
    }
}
