use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use crate::components::MainCamera;
use crate::resources::{MouseLook, MenuState, FreeLook};

use std::f32::consts::PI;

const BASE_MOUSE_SENSITIVITY: f32 = 0.0025;

pub fn mouse_look_system(
    mut motion_evr: EventReader<MouseMotion>,
    mut mouse_look: ResMut<MouseLook>,
    mut free_look: ResMut<FreeLook>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    menu: Res<MenuState>,
    keyboard: Res<Input<KeyCode>>,
) {
    // disable mouse look while menu is open
    if menu.open {
        return;
    }

    let holding_c = keyboard.pressed(KeyCode::C);

    // On C pressed: save current travel direction
    if keyboard.just_pressed(KeyCode::C) {
        free_look.travel_yaw   = mouse_look.yaw;
        free_look.travel_pitch = mouse_look.pitch;
        free_look.active = true;
    }
    if keyboard.just_released(KeyCode::C) {
        // Restore travel direction so movement continues the same way
        mouse_look.yaw   = free_look.travel_yaw;
        mouse_look.pitch = free_look.travel_pitch;
        free_look.active = false;
    }

    let sensitivity = mouse_look.sensitivity * BASE_MOUSE_SENSITIVITY;
    for ev in motion_evr.iter() {
        mouse_look.yaw   -= ev.delta.x * sensitivity;
        mouse_look.pitch += -ev.delta.y * sensitivity;
    }

    // Wrap angles into (-PI, PI]
    mouse_look.yaw   = (mouse_look.yaw   + PI).rem_euclid(2.0 * PI) - PI;
    mouse_look.pitch = (mouse_look.pitch + PI).rem_euclid(2.0 * PI) - PI;

    if let Ok(mut transform) = camera_q.get_single_mut() {
        transform.rotation = Quat::from_euler(EulerRot::YXZ, mouse_look.yaw, mouse_look.pitch, 0.0);
    }

    let _ = holding_c; // used via keyboard.pressed above
}
