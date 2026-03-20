use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use crate::components::MainCamera;
use crate::resources::{MouseLook, MenuState};

use std::f32::consts::PI;

const BASE_MOUSE_SENSITIVITY: f32 = 0.0025;

pub fn mouse_look_system(
    mut motion_evr: EventReader<MouseMotion>,
    mut mouse_look: ResMut<MouseLook>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    menu: Res<MenuState>,
) {
    // disable mouse look while menu is open
    if menu.open {
        return;
    }

    let sensitivity = mouse_look.sensitivity * BASE_MOUSE_SENSITIVITY;
    for ev in motion_evr.iter() {
        // invert horizontal sign so moving mouse right rotates view to the right
        mouse_look.yaw -= ev.delta.x * sensitivity;
        mouse_look.pitch += -ev.delta.y * sensitivity;
    }

    // Allow full looping on both axes by wrapping angles into (-PI, PI]
    mouse_look.yaw = (mouse_look.yaw + PI).rem_euclid(2.0 * PI) - PI;
    mouse_look.pitch = (mouse_look.pitch + PI).rem_euclid(2.0 * PI) - PI;

    if let Ok(mut transform) = camera_q.get_single_mut() {
        transform.rotation = Quat::from_euler(EulerRot::YXZ, mouse_look.yaw, mouse_look.pitch, 0.0);
    }
}
