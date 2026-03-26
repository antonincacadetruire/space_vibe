use bevy::prelude::*;

use crate::components::{MainCamera, PlayerShipModel};
use crate::resources::{CameraArmOffset, CameraMode, FreeLook, GameState};

/// Toggle CameraMode between first- and third-person with F5.
pub fn camera_toggle_system(
    keyboard: Res<Input<KeyCode>>,
    mut cam_mode: ResMut<CameraMode>,
    mut ship_q: Query<&mut Visibility, With<PlayerShipModel>>,
    state: Res<State<GameState>>,
) {
    if *state.get() != GameState::Playing { return; }

    if keyboard.just_pressed(KeyCode::F5) {
        *cam_mode = match *cam_mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
        };

        let want_visible = matches!(*cam_mode, CameraMode::ThirdPerson);
        for mut vis in ship_q.iter_mut() {
            *vis = if want_visible { Visibility::Visible } else { Visibility::Hidden };
        }
    }
}

/// **Must run BEFORE `player_movement_system` every frame.**
/// Removes the spring-arm offset applied in the previous frame so that the
/// movement system operates at the logical cockpit position.
pub fn undo_arm_offset_system(
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    offset: Res<CameraArmOffset>,
) {
    if let Ok(mut t) = cam_q.get_single_mut() {
        t.translation -= offset.0;
    }
}

/// **Must run AFTER `player_movement_system` every frame.**
/// Re-applies a spring-arm offset so the camera renders from behind the ship in
/// third-person mode. When the player is in the orbit free-look (C held), the
/// arm uses the orbital yaw/pitch so the camera circles the ship without
/// conflicting with the movement system.
pub fn apply_arm_offset_system(
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    cam_mode: Res<CameraMode>,
    mut offset: ResMut<CameraArmOffset>,
    free_look: Res<FreeLook>,
) {
    if let Ok(mut t) = cam_q.get_single_mut() {
        if *cam_mode == CameraMode::ThirdPerson {
            // In orbit mode: pull camera back along the ORBIT direction so it
            // circles around the ship without any translation conflict.
            let arm_rot = if free_look.active {
                Quat::from_euler(EulerRot::YXZ, free_look.orbit_yaw, free_look.orbit_pitch, 0.0)
            } else {
                t.rotation
            };
            let back = arm_rot * Vec3::Z;   // local +Z of orbit rotation = "back from ship"
            offset.0 = back * 16.0 + Vec3::Y * 9.0;
        } else {
            offset.0 = Vec3::ZERO;
        }
        t.translation += offset.0;
    }
}
