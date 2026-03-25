use bevy::prelude::*;

use crate::components::{MainCamera, PlayerShipModel};
use crate::resources::{CameraArmOffset, CameraMode, GameState};

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
/// third-person mode; clears the offset in first-person mode.
pub fn apply_arm_offset_system(
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    cam_mode: Res<CameraMode>,
    mut offset: ResMut<CameraArmOffset>,
) {
    if let Ok(mut t) = cam_q.get_single_mut() {
        if *cam_mode == CameraMode::ThirdPerson {
            // Pull camera 22 units backward along its own local Z axis and 4 up.
            // Using the camera's own rotation ensures the arm follows look direction.
            let back = t.rotation * Vec3::Z;   // local +Z = camera backward
            offset.0 = back * 22.0 + Vec3::Y * 4.0;
        } else {
            offset.0 = Vec3::ZERO;
        }
        t.translation += offset.0;
    }
}
