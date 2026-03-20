use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use crate::resources::MouseLook;

const MAX_YAW_RADIANS: f32 = 0.8;
const MAX_PITCH_RADIANS: f32 = 0.4;

pub fn mouse_look_system(
    mut motion_evr: EventReader<MouseMotion>,
    mut mouse_look: ResMut<MouseLook>,
) {
    let sensitivity = 0.0025_f32;
    for ev in motion_evr.iter() {
        mouse_look.yaw += ev.delta.x * sensitivity;
        mouse_look.pitch += -ev.delta.y * sensitivity;
    }
    mouse_look.pitch = mouse_look.pitch.clamp(-MAX_PITCH_RADIANS, MAX_PITCH_RADIANS);
    mouse_look.yaw = mouse_look.yaw.clamp(-MAX_YAW_RADIANS, MAX_YAW_RADIANS);
}
