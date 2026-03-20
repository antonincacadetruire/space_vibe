use bevy::prelude::*;

use crate::components::{Asteroid, MainCamera, Radius, Velocity};
use crate::resources::{Throttle, TimePaused, VelocityUpdates};

pub fn player_movement_system(
    time: Res<Time>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut throttle: ResMut<Throttle>,
    mut paused: ResMut<TimePaused>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        paused.0 = !paused.0;
    }

    // Do not block player movement when time is paused — only asteroids/spawner stop.
    let Ok(mut transform) = camera_q.get_single_mut() else { return };

    let dt = time.delta_seconds();
    // Support common keyboard layouts (QWERTY and AZERTY) and arrows
    if keyboard.pressed(KeyCode::W) || keyboard.pressed(KeyCode::Z) || keyboard.pressed(KeyCode::Up) {
        throttle.0 += 1000.0 * dt;
    }
    if keyboard.pressed(KeyCode::S) || keyboard.pressed(KeyCode::Down) {
        throttle.0 -= 1000.0 * dt;
    }
    throttle.0 = throttle.0.clamp(0.0, 1000.0);

    let forward = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero();
    let vertical_up = keyboard.pressed(KeyCode::E);
    let vertical_down = keyboard.pressed(KeyCode::Q);
    let vertical = match (vertical_up, vertical_down) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    };

    let movement = forward * throttle.0 * dt + Vec3::Y * crate::SHUTTLE_SPEED * vertical * dt;
    transform.translation += movement;
}

pub fn asteroid_movement_system(
    time: Res<Time>,
    mut commands: Commands,
    mut asteroids: Query<(Entity, &mut Velocity, &Radius, &mut Transform), (With<Asteroid>, Without<MainCamera>)>,
    camera_q: Query<&Transform, With<MainCamera>>,
    updates: Res<VelocityUpdates>,
    paused: Res<TimePaused>,
) {
    if paused.0 {
        return;
    }
    let Ok(camera_transform) = camera_q.get_single() else { return };

    for (entity, mut vel_comp, _radius, mut transform) in asteroids.iter_mut() {
        if let Some(new_vel) = updates.0.get(&entity) {
            vel_comp.0 = *new_vel;
        }

        transform.translation += vel_comp.0 * time.delta_seconds();

        if transform.translation.y < -1500.0
            || transform.translation.y > 2500.0
            || transform.translation.x.abs() > 5000.0
            || transform.translation.z.abs() > 5000.0
        {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        let dist = (transform.translation - camera_transform.translation).length();
        let camera_radius = 15.0;
        let asteroid_radius = 16.0;
        if dist < camera_radius + asteroid_radius {
            info!("Collision with asteroid (camera/player)!");
            commands.entity(entity).despawn_recursive();
        }
    }
}
