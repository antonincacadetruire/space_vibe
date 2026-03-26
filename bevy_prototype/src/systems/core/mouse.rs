use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use crate::components::MainCamera;
use crate::resources::{MouseLook, MenuState, FreeLook, CameraMode};
use crate::systems::ui::copilot_chat::LlmChatState;

use std::f32::consts::PI;

const BASE_MOUSE_SENSITIVITY: f32 = 0.0025;

pub fn mouse_look_system(
    mut motion_evr: EventReader<MouseMotion>,
    mut mouse_look: ResMut<MouseLook>,
    mut free_look: ResMut<FreeLook>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    menu: Res<MenuState>,
    keyboard: Res<Input<KeyCode>>,
    chat: Res<LlmChatState>,
    cam_mode: Res<CameraMode>,
) {
    // disable mouse look while menu or chat is open
    if menu.open || chat.open {
        return;
    }

    let in_third_person = *cam_mode == CameraMode::ThirdPerson;

    // On C pressed: save travel direction
    if keyboard.just_pressed(KeyCode::C) {
        free_look.travel_yaw   = mouse_look.yaw;
        free_look.travel_pitch = mouse_look.pitch;
        free_look.active = true;
        if in_third_person {
            // orbit starts aligned with current view
            free_look.orbit_yaw   = mouse_look.yaw;
            free_look.orbit_pitch = mouse_look.pitch;
        }
    }
    if keyboard.just_released(KeyCode::C) {
        // Restore travel direction on release
        mouse_look.yaw   = free_look.travel_yaw;
        mouse_look.pitch = free_look.travel_pitch;
        free_look.active = false;
        free_look.orbit_center = None;
    }

    let sensitivity = mouse_look.sensitivity * BASE_MOUSE_SENSITIVITY;
    let mut total_dx = 0.0_f32;
    let mut total_dy = 0.0_f32;
    for ev in motion_evr.iter() {
        total_dx += ev.delta.x;
        total_dy += ev.delta.y;
    }

    if let Ok(mut transform) = camera_q.get_single_mut() {
        if in_third_person && free_look.active {
            // ── Third-person orbit ────────────────────────────────────────────
            // Only update the orbit angles + camera ROTATION here.
            // apply_arm_offset_system uses orbit_yaw/pitch to place the camera
            // behind the ship from the correct orbital angle, so we must NOT
            // touch camera TRANSLATION here — that would fight the arm system.
            free_look.orbit_yaw   -= total_dx * sensitivity;
            free_look.orbit_pitch -= total_dy * sensitivity;
            free_look.orbit_pitch = free_look.orbit_pitch.clamp(-PI * 0.45, PI * 0.45);

            // Wrap yaw
            free_look.orbit_yaw = (free_look.orbit_yaw + PI).rem_euclid(2.0 * PI) - PI;

            // Camera is placed at ship_pos + orbit_rot * Z * 16 (by apply_arm_offset).
            // It must look TOWARD the ship, i.e. forward = orbit_rot * (-Z).
            // That is exactly what rotation = orbit_rot gives us.
            let orbit_rot = Quat::from_euler(
                EulerRot::YXZ,
                free_look.orbit_yaw,
                free_look.orbit_pitch,
                0.0,
            );
            transform.rotation = orbit_rot;
        } else {
            // ── Normal look / first-person free-look ──────────────────────────
            mouse_look.yaw   -= total_dx * sensitivity;
            mouse_look.pitch -= total_dy * sensitivity;

            // Wrap angles into (-PI, PI]
            mouse_look.yaw   = (mouse_look.yaw   + PI).rem_euclid(2.0 * PI) - PI;
            mouse_look.pitch = (mouse_look.pitch + PI).rem_euclid(2.0 * PI) - PI;

            transform.rotation = Quat::from_euler(EulerRot::YXZ, mouse_look.yaw, mouse_look.pitch, 0.0);
        }
    }
}
