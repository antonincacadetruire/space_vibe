use bevy::prelude::*;

use crate::components::{AlienHealthPip, AlienShip, Explosion, Laser, MainCamera, SpawnPortal};
use crate::resources::TimePaused;

// ── Player shoots a laser bolt on left-click ──────────────────────────────────
pub fn shoot_laser_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<Input<MouseButton>>,
    paused: Res<TimePaused>,
    camera_q: Query<&Transform, With<MainCamera>>,
) {
    if paused.0 || !mouse.just_pressed(MouseButton::Left) { return; }
    let Ok(cam) = camera_q.get_single() else { return };
    let forward = (cam.rotation * Vec3::NEG_Z).normalize_or_zero();
    let rot = Quat::from_rotation_arc(Vec3::Y, forward);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cylinder {
                radius: 9.0,
                height: 320.0,
                resolution: 8,
                segments: 1,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.0, 0.9, 1.0),
                emissive: Color::rgb(0.0, 7.0, 12.0),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                ..default()
            }),
            transform: Transform::from_translation(cam.translation + forward * 120.0)
                .with_rotation(rot),
            ..default()
        },
        Laser { speed: 70_000.0, lifetime: 2.0 },
    ));
}

// ── Move laser bolts and resolve UFO hits ─────────────────────────────────────
pub fn laser_movement_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut lasers: Query<(Entity, &mut Transform, &mut Laser)>,
    mut aliens: Query<(Entity, &Transform, &mut AlienShip), Without<Laser>>,
) {
    if paused.0 { return; }
    let dt = time.delta_seconds();

    // Collect hits and expired in one pass (avoids nested mut-borrow conflicts).
    let mut hits: Vec<(Entity, Entity, Vec3)> = Vec::new();
    let mut expired: Vec<Entity> = Vec::new();

    for (laser_entity, mut transform, mut laser) in lasers.iter_mut() {
        let fwd = transform.rotation * Vec3::Y;
        transform.translation += fwd * laser.speed * dt;
        laser.lifetime -= dt;

        if laser.lifetime <= 0.0 {
            expired.push(laser_entity);
            continue;
        }

        for (alien_entity, alien_transform, _) in aliens.iter() {
            if transform.translation.distance(alien_transform.translation) < 1_200.0 {
                hits.push((laser_entity, alien_entity, transform.translation));
                break; // one hit per laser per frame
            }
        }
    }

    for e in expired { commands.entity(e).despawn_recursive(); }

    // Process hits after the iteration is over.
    let mut processed: std::collections::HashSet<Entity> = Default::default();
    for (laser_entity, alien_entity, hit_pos) in hits {
        commands.entity(laser_entity).despawn_recursive();

        // Small cyan impact flash
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0,
                    sectors: 6,
                    stacks: 3,
                })),
                material: materials.add(StandardMaterial {
                    emissive: Color::rgb(0.0, 8.0, 12.0),
                    alpha_mode: AlphaMode::Add,
                    unlit: true,
                    ..default()
                }),
                transform: Transform::from_translation(hit_pos),
                ..default()
            },
            Explosion { timer: 0.0, max_time: 0.22, max_scale: 350.0 },
        ));

        // Only one damage instance per alien per frame
        if processed.contains(&alien_entity) { continue; }
        processed.insert(alien_entity);

        let mut killed = false;
        let mut alien_pos = Vec3::ZERO;
        if let Ok((_, alien_transform, mut ship)) = aliens.get_mut(alien_entity) {
            ship.health -= 1;
            alien_pos = alien_transform.translation;
            if ship.health <= 0 { killed = true; }
        }

        if killed {
            // Big orange kill explosion
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 1.0,
                        sectors: 12,
                        stacks: 6,
                    })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(1.0, 0.4, 0.0),
                        emissive: Color::rgb(10.0, 3.0, 0.0),
                        alpha_mode: AlphaMode::Add,
                        unlit: true,
                        ..default()
                    }),
                    transform: Transform::from_translation(alien_pos),
                    ..default()
                },
                Explosion { timer: 0.0, max_time: 1.4, max_scale: 3_500.0 },
            ));
            commands.entity(alien_entity).despawn_recursive();
        }
    }
}

// ── Animate portal rings (appear when an alien spawns) ────────────────────────
pub fn portal_animation_system(
    mut commands: Commands,
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut portals: Query<(Entity, &mut Transform, &mut SpawnPortal)>,
) {
    if paused.0 { return; }
    let dt = time.delta_seconds();
    for (entity, mut transform, mut portal) in portals.iter_mut() {
        portal.timer += dt;
        if portal.timer < 0.0 { continue; } // still in pre-delay

        let t = (portal.timer / portal.max_time).clamp(0.0, 1.0);
        let scale = if t < 0.55 {
            t / 0.55           // expand
        } else if t < 0.78 {
            1.0                // hold
        } else {
            1.0 - (t - 0.78) / 0.22  // collapse
        };
        let s = scale.max(0.001);
        transform.scale = Vec3::new(s, 1.0, s);
        transform.rotate_local_y(3.0 * dt);

        if portal.timer >= portal.max_time {
            commands.entity(entity).despawn_recursive();
        }
    }
}

// ── Animate explosion spheres ─────────────────────────────────────────────────
pub fn explosion_animation_system(
    mut commands: Commands,
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut explosions: Query<(Entity, &mut Transform, &mut Explosion)>,
) {
    if paused.0 { return; }
    let dt = time.delta_seconds();
    for (entity, mut transform, mut expl) in explosions.iter_mut() {
        expl.timer += dt;
        let t = (expl.timer / expl.max_time).clamp(0.0, 1.0);
        // Fast expand, then gentle shrink
        let scale = if t < 0.40 {
            (t / 0.40) * expl.max_scale
        } else {
            expl.max_scale * (1.0 - (t - 0.40) / 0.60 * 0.60)
        };
        transform.scale = Vec3::splat(scale.max(0.01));
        if expl.timer >= expl.max_time {
            commands.entity(entity).despawn_recursive();
        }
    }
}

// ── Update health pip materials from parent ship health ───────────────────────
pub fn health_pip_update_system(
    alien_q: Query<(&AlienShip, &Children)>,
    mut pip_q: Query<(&AlienHealthPip, &mut Handle<StandardMaterial>)>,
) {
    for (ship, children) in alien_q.iter() {
        for &child in children.iter() {
            if let Ok((pip, mut mat)) = pip_q.get_mut(child) {
                *mat = if (pip.index as i32) < ship.health {
                    pip.mat_active.clone()
                } else {
                    pip.mat_inactive.clone()
                };
            }
        }
    }
}

// ── Cleanup combat effects on state exit ─────────────────────────────────────
pub fn despawn_effects(
    mut commands: Commands,
    lasers: Query<Entity, With<Laser>>,
    explosions: Query<Entity, With<Explosion>>,
) {
    for e in lasers.iter() { commands.entity(e).despawn_recursive(); }
    for e in explosions.iter() { commands.entity(e).despawn_recursive(); }
}
