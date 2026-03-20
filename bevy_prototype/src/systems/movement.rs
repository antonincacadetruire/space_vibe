use bevy::prelude::*;

use crate::components::{Asteroid, MainCamera, Radius, Velocity, AngularVelocity};
use crate::resources::{Throttle, TimePaused, VelocityUpdates, MenuState, Keybindings, PrevCameraPosition};

pub fn player_movement_system(
    time: Res<Time>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut throttle: ResMut<Throttle>,
    mut paused: ResMut<TimePaused>,
    menu: Res<MenuState>,
    keyb: Res<Keybindings>,
    keyboard: Res<Input<KeyCode>>,
) {
    // Toggle pause via keybinding
    if keyboard.just_pressed(keyb.toggle_pause) {
        paused.0 = !paused.0;
    }

    // If the menu is open, prevent player movement
    if menu.open {
        return;
    }

    let Ok(mut transform) = camera_q.get_single_mut() else { return };

    // store previous camera pos at start so other systems can use swept tests
    // (we'll rely on PrevCameraPosition resource updated each frame elsewhere)

    let dt = time.delta_seconds();
    // Support configurable keybindings; keep AZERTY fallback for throttle up
    if keyboard.pressed(keyb.throttle_up) || keyboard.pressed(KeyCode::Z) || keyboard.pressed(KeyCode::Up) {
        throttle.0 += 20_000.0 * dt;
    }
    if keyboard.pressed(keyb.throttle_down) || keyboard.pressed(KeyCode::Down) {
        throttle.0 -= 20_000.0 * dt;
    }
    throttle.0 = throttle.0.clamp(-50_000.0, 50_000.0);

    let forward = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero();
    let vertical_up = keyboard.pressed(keyb.vertical_up);
    let vertical_down = keyboard.pressed(keyb.vertical_down);
    let vertical = match (vertical_up, vertical_down) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    };

    let movement = forward * throttle.0 * dt + Vec3::Y * crate::SHUTTLE_SPEED * vertical * dt;
    transform.translation += movement;
}

pub fn record_camera_position_system(
    camera_q: Query<&Transform, With<MainCamera>>,
    mut prev: ResMut<PrevCameraPosition>,
) {
    if let Ok(transform) = camera_q.get_single() {
        prev.0 = transform.translation;
    }
}

pub fn asteroid_movement_system(
    time: Res<Time>,
    mut commands: Commands,
    mut asteroids: Query<(
        Entity,
        &mut Velocity,
        &Radius,
        &mut Transform,
        Option<&AngularVelocity>,
    ), (With<Asteroid>, Without<MainCamera>)>,
    camera_q: Query<&Transform, With<MainCamera>>,
    updates: Res<VelocityUpdates>,
    paused: Res<TimePaused>,
    prev_cam: Res<PrevCameraPosition>,
) {
    if paused.0 {
        return;
    }
    let Ok(camera_transform) = camera_q.get_single() else { return };

    for (entity, mut vel_comp, _radius, mut transform, ang_opt) in asteroids.iter_mut() {
        if let Some(new_vel) = updates.0.get(&entity) {
            vel_comp.0 = *new_vel;
        }

        transform.translation += vel_comp.0 * time.delta_seconds();
        if let Some(ang) = ang_opt {
            let ang_vec = ang.0;
            let dt = time.delta_seconds();
            if ang_vec.length_squared() > 0.0 {
                let angle = ang_vec.length() * dt;
                let axis = ang_vec.normalize_or_zero();
                transform.rotate(Quat::from_axis_angle(axis, angle));
            }
        }

        if transform.translation.length() > 3_000_000.0 {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        // swept-sphere test: check closest distance from camera movement segment to asteroid center
        let camera_prev = prev_cam.0;
        let cam_start = camera_prev;
        let cam_end = camera_transform.translation;
        let seg = cam_end - cam_start;
        let to_center = transform.translation - cam_start;
        let seg_len_sq = seg.length_squared();
        let t = if seg_len_sq > 0.0 { seg.dot(to_center) / seg_len_sq } else { 0.0 };
        let t_clamped = t.clamp(0.0, 1.0);
        let closest = cam_start + seg * t_clamped;
        let dist = (transform.translation - closest).length();
        let camera_radius = 12.0; // slightly larger to be forgiving
        if dist < camera_radius + _radius.0 {
            info!("Collision with asteroid (camera/player)!");
            commands.entity(entity).despawn_recursive();
        }
    }
}
