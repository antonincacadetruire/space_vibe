use bevy::prelude::*;

use crate::components::{Asteroid, BeltAsteroid, MainCamera, Radius, Velocity, AngularVelocity};
use crate::resources::{DeathCause, DesertTerrainData, Throttle, SpeedMode, TimePaused, VelocityUpdates, MenuState, Keybindings, PrevCameraPosition, GameState, GameTimer, FreeLook, ZoneBoundary};
use crate::systems::ui::copilot_chat::LlmChatState;

/// Preset speed values (units/s) for the E/A tap system.
const PRESET_SPEEDS: [f32; 3] = [5_000.0, 10_000.0, 15_000.0];
/// Max speed when holding Z/S (units/s).
const MANUAL_MAX_SPEED: f32 = 15_000.0;
/// Time (seconds) to accelerate from 0 to MANUAL_MAX_SPEED while holding Z/S.
const MANUAL_ACCEL_TIME: f32 = 1.5;
/// Time (seconds) to coast from full speed to 0 after releasing Z/S.
const COAST_DURATION: f32 = 3.5;

pub fn player_movement_system(
    time: Res<Time>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut throttle: ResMut<Throttle>,
    mut speed_mode: ResMut<SpeedMode>,
    mut paused: ResMut<TimePaused>,
    menu: Res<MenuState>,
    keyb: Res<Keybindings>,
    keyboard: Res<Input<KeyCode>>,
    free_look: Res<FreeLook>,
    boundary: Res<ZoneBoundary>,
    chat: Res<LlmChatState>,
) {
    // Toggle pause via keybinding
    if keyboard.just_pressed(keyb.toggle_pause) {
        paused.0 = !paused.0;
    }

    // If the menu or chat is open, prevent player movement
    if menu.open || chat.open {
        return;
    }

    let Ok(mut transform) = camera_q.get_single_mut() else { return };

    let dt = time.delta_seconds();

    // ── E / A preset taps ────────────────────────────────────────────────────
    if keyboard.just_pressed(KeyCode::E) || keyboard.just_pressed(keyb.vertical_up) {
        speed_mode.manual_active = false;
        speed_mode.preset_step = (speed_mode.preset_step + 1).clamp(-3, 3);
        if speed_mode.preset_step == 0 { speed_mode.preset_step = 1; }
    }
    if keyboard.just_pressed(KeyCode::A) || keyboard.just_pressed(keyb.vertical_down) {
        speed_mode.manual_active = false;
        speed_mode.preset_step = (speed_mode.preset_step - 1).clamp(-3, 3);
        if speed_mode.preset_step == 0 { speed_mode.preset_step = -1; }
    }

    // ── Z / S manual acceleration ────────────────────────────────────────────
    let z_held = keyboard.pressed(keyb.throttle_up) || keyboard.pressed(KeyCode::Z) || keyboard.pressed(KeyCode::Up);
    let s_held = keyboard.pressed(keyb.throttle_down) || keyboard.pressed(KeyCode::Down);

    if z_held || s_held {
        // Override preset mode
        speed_mode.manual_active = true;
        speed_mode.preset_step = 0;

        let accel = MANUAL_MAX_SPEED / MANUAL_ACCEL_TIME;
        if z_held {
            speed_mode.manual_target = MANUAL_MAX_SPEED;
            throttle.0 = (throttle.0 + accel * dt).min(MANUAL_MAX_SPEED);
        }
        if s_held {
            speed_mode.manual_target = -MANUAL_MAX_SPEED;
            throttle.0 = (throttle.0 - accel * dt).max(-MANUAL_MAX_SPEED);
        }
    } else if speed_mode.manual_active {
        // Z/S released → coast to zero
        let decel = throttle.0.abs() / COAST_DURATION.max(0.01);
        let decel = decel.max(MANUAL_MAX_SPEED / COAST_DURATION); // minimum decel
        if throttle.0 > 0.0 {
            throttle.0 = (throttle.0 - decel * dt).max(0.0);
        } else if throttle.0 < 0.0 {
            throttle.0 = (throttle.0 + decel * dt).min(0.0);
        }
        if throttle.0.abs() < 10.0 {
            throttle.0 = 0.0;
            speed_mode.manual_active = false;
        }
    } else if speed_mode.preset_step != 0 {
        // Smoothly approach preset speed
        let idx = (speed_mode.preset_step.unsigned_abs() as usize).clamp(1, 3) - 1;
        let target = PRESET_SPEEDS[idx] * speed_mode.preset_step.signum() as f32;
        let approach_rate = MANUAL_MAX_SPEED / MANUAL_ACCEL_TIME;
        if (target - throttle.0).abs() < approach_rate * dt {
            throttle.0 = target;
        } else if target > throttle.0 {
            throttle.0 += approach_rate * dt;
        } else {
            throttle.0 -= approach_rate * dt;
        }
    }

    // When free-look is active use saved travel direction, not current camera rotation
    let travel_rotation = if free_look.active {
        Quat::from_euler(EulerRot::YXZ, free_look.travel_yaw, free_look.travel_pitch, 0.0)
    } else {
        transform.rotation
    };
    let forward = travel_rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero();

    let movement = forward * throttle.0 * dt;
    transform.translation += movement;

    // Boundary: push the player back inward but preserve speed (feels natural).
    let dist = transform.translation.length();
    if dist > boundary.0 {
        // Clamp position to sphere surface
        transform.translation = transform.translation / dist * boundary.0;
        // Reflect the throttle so the player bounces; also bleed 30 % energy.
        throttle.0 = -(throttle.0 * 0.7);
    }
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
    ), (With<Asteroid>, Without<MainCamera>, Without<BeltAsteroid>)>,
    camera_q: Query<&Transform, With<MainCamera>>,
    updates: Res<VelocityUpdates>,
    paused: Res<TimePaused>,
    mut death_cause: ResMut<DeathCause>,
    prev_cam: Res<PrevCameraPosition>,
    game_timer: Res<GameTimer>,
    mut next_state: ResMut<NextState<GameState>>,
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
            info!("Collision with asteroid (camera/player)! Score: {:.1}s", game_timer.0);
            commands.entity(entity).despawn_recursive();
            *death_cause = DeathCause::Asteroid;
            next_state.set(GameState::Dead);
        }
    }
}

/// Kills the player if they fly into desert terrain (floor or mountain peaks).
/// Only active when the desert map is loaded (resource present).
pub fn desert_terrain_death_system(
    terrain: Option<Res<DesertTerrainData>>,
    camera_q: Query<&Transform, With<crate::components::MainCamera>>,
    paused: Res<TimePaused>,
    mut death_cause: ResMut<DeathCause>,
    mut next_state: ResMut<NextState<GameState>>,
    game_timer: Res<GameTimer>,
) {
    if paused.0 { return; }
    let Some(terrain) = terrain else { return };
    let Ok(cam) = camera_q.get_single() else { return };
    let pos = cam.translation;

    // Floor death
    if pos.y < terrain.floor_y {
        info!("Player hit the desert floor! Score: {:.1}s", game_timer.0);
        *death_cause = DeathCause::Terrain;
        next_state.set(GameState::Dead);
        return;
    }

    // Mountain / spire / dune death — check against stored kill ellipsoids
    for &(center, hr, vr) in &terrain.kill_zones {
        let d = pos - center;
        let normalized = (d.x * d.x + d.z * d.z) / (hr * hr) + (d.y * d.y) / (vr * vr);
        if normalized < 1.0 {
            info!("Player flew into terrain obstacle! Score: {:.1}s", game_timer.0);
            *death_cause = DeathCause::Terrain;
            next_state.set(GameState::Dead);
            return;
        }
    }
}
