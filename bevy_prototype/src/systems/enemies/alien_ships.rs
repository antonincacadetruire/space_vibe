use bevy::prelude::*;
use rand::Rng;

use crate::components::{AlienHealthPip, AlienShip, MainCamera, Missile, SpawnPortal};
use crate::resources::{AlienSpawnTimer, GameTimer, TimePaused};
use crate::systems::data_loader::EnemyCatalog;

// ── Spawner ───────────────────────────────────────────────────────────────────
pub fn alien_ship_spawner_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    game_timer: Res<GameTimer>,
    mut spawn_timer: ResMut<AlienSpawnTimer>,
    paused: Res<TimePaused>,
    camera_q: Query<&Transform, With<MainCamera>>,
    aliens: Query<(), With<AlienShip>>,
    enemy_catalog: Res<EnemyCatalog>,
) {
    let def = enemy_catalog.active();
    if paused.0 || game_timer.0 < def.first_spawn_time { return; }
    if aliens.iter().count() >= def.max_count { return; }

    spawn_timer.0.tick(time.delta());
    if !spawn_timer.0.just_finished() { return; }

    let Ok(cam) = camera_q.get_single() else { return };
    let mut rng = rand::thread_rng();

    let spawn_dist = rng.gen_range(def.spawn_dist_min..def.spawn_dist_max);
    let angle_h = rng.gen_range(0.0_f32..std::f32::consts::TAU);
    let angle_v = rng.gen_range(-0.3_f32..0.3);
    let dir = Vec3::new(
        angle_h.cos() * angle_v.cos(),
        angle_v.sin(),
        angle_h.sin() * angle_v.cos(),
    );
    let spawn_pos = cam.translation + dir * spawn_dist;

    let speed = rng.gen_range(def.speed_min..def.speed_max);
    let shoot_interval = rng.gen_range(def.shoot_interval_min..def.shoot_interval_max);

    // ── Materials (colours come from the active EnemyDef) ─────────────────────
    let [hr, hg, hb] = def.hull_color;
    let [her, heg, heb] = def.hull_emissive;
    let [rr, rg, rb] = def.rim_color;
    let [rer, reg, reb] = def.rim_emissive;
    let [dr, dg, db] = def.dome_color;
    let [der, deg, deb] = def.dome_emissive;

    let hull_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(hr, hg, hb),
        emissive: Color::rgb(her, heg, heb),
        perceptual_roughness: 0.25,
        metallic: 0.95,
        ..default()
    });
    let rim_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(rr, rg, rb),
        emissive: Color::rgb(rer, reg, reb),
        unlit: true,
        ..default()
    });
    let dome_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(dr, dg, db, 0.80),
        emissive: Color::rgb(der, deg, deb),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.08,
        ..default()
    });
    let pod_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.08, 0.04, 0.16),
        metallic: 0.9,
        perceptual_roughness: 0.3,
        ..default()
    });
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(0.8, 0.0, 1.0, 0.9),
        emissive: Color::rgb(5.0, 0.0, 8.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    let antenna_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.12, 0.06, 0.22),
        emissive: Color::rgb(1.0, 0.0, 2.0),
        metallic: 0.85,
        perceptual_roughness: 0.2,
        ..default()
    });

    // ── Meshes ────────────────────────────────────────────────────────────────
    // Central hull disc
    let hull_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 900.0,
        height: 280.0,
        resolution: 36,
        segments: 1,
    }));
    // Glowing outer rim disc
    let rim_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 1300.0,
        height: 70.0,
        resolution: 48,
        segments: 1,
    }));
    // Dome on top
    let dome_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 425.0,
        sectors: 20,
        stacks: 10,
    }));
    // Engine pods (6× around the underside)
    let pod_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 80.0,
        height: 325.0,
        resolution: 10,
        segments: 1,
    }));
    // Engine glows
    let glow_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 90.0,
        sectors: 8,
        stacks: 4,
    }));
    // Antenna spires (3× on top)
    let antenna_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 25.0,
        height: 450.0,
        resolution: 8,
        segments: 1,
    }));
    let antenna_tip_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 50.0,
        sectors: 8,
        stacks: 4,
    }));

    // ── Health-pip assets (defined before spawn so handles are cloneable into closure) ──
    let pip_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 100.0,
        sectors: 10,
        stacks: 5,
    }));
    let pip_active_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.0, 1.0, 0.2),
        emissive: Color::rgb(0.0, 5.0, 1.0),
        unlit: true,
        ..default()
    });
    let pip_inactive_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.06, 0.06, 0.06),
        perceptual_roughness: 1.0,
        ..default()
    });

    // ── Spawn hierarchy ──────────────────────────────────────────────────────
    commands
        .spawn((
            PbrBundle {
                mesh: hull_mesh,
                material: hull_mat,
                transform: Transform::from_translation(spawn_pos),
                ..default()
            },
            AlienShip { speed, shoot_timer: shoot_interval, shoot_interval, health: def.health },
        ))
        .with_children(|ship| {
            // Outer glowing rim
            ship.spawn(PbrBundle {
                mesh: rim_mesh,
                material: rim_mat,
                transform: Transform::from_translation(Vec3::ZERO),
                ..default()
            });
            // Dome
            ship.spawn(PbrBundle {
                mesh: dome_mesh,
                material: dome_mat,
                transform: Transform::from_translation(Vec3::new(0.0, 160.0, 0.0)),
                ..default()
            });
            // 6 engine pods + glows
            for i in 0..6 {
                let angle = std::f32::consts::TAU * i as f32 / 6.0;
                let pod_x = angle.cos() * 650.0;
                let pod_z = angle.sin() * 650.0;
                ship.spawn(PbrBundle {
                    mesh: pod_mesh.clone(),
                    material: pod_mat.clone(),
                    transform: Transform::from_translation(Vec3::new(pod_x, -225.0, pod_z)),
                    ..default()
                });
                ship.spawn(PbrBundle {
                    mesh: glow_mesh.clone(),
                    material: glow_mat.clone(),
                    transform: Transform::from_translation(Vec3::new(pod_x, -410.0, pod_z)),
                    ..default()
                });
            }
            // 3 antenna spires on top
            for i in 0..3 {
                let angle = std::f32::consts::TAU * i as f32 / 3.0 + std::f32::consts::FRAC_PI_6;
                let ant_x = angle.cos() * 400.0;
                let ant_z = angle.sin() * 400.0;
                ship.spawn(PbrBundle {
                    mesh: antenna_mesh.clone(),
                    material: antenna_mat.clone(),
                    transform: Transform::from_translation(Vec3::new(ant_x, 400.0, ant_z)),
                    ..default()
                });
                ship.spawn(PbrBundle {
                    mesh: antenna_tip_mesh.clone(),
                    material: glow_mat.clone(),
                    transform: Transform::from_translation(Vec3::new(ant_x, 650.0, ant_z)),
                    ..default()
                });
            }
            // Central beacon light — makes the ship visible from far away
            ship.spawn(PointLightBundle {
                point_light: PointLight {
                    intensity: 80_000_000.0,
                    range: 30_000.0,
                    color: Color::rgb(0.55, 0.0, 1.0),
                    shadows_enabled: false,
                    ..default()
                },
                transform: Transform::from_translation(Vec3::ZERO),
                ..default()
            });
            // Health pips — 3 glowing orbs above the saucer
            for i in 0..3usize {
                let x_off = (i as f32 - 1.0) * 280.0;
                ship.spawn((
                    PbrBundle {
                        mesh: pip_mesh.clone(),
                        material: pip_active_mat.clone(),
                        transform: Transform::from_translation(Vec3::new(x_off, 1_100.0, 0.0)),
                        ..default()
                    },
                    AlienHealthPip {
                        index: i,
                        mat_active: pip_active_mat.clone(),
                        mat_inactive: pip_inactive_mat.clone(),
                    },
                ));
            }
        });

    // ── Portal spawn effect ─────────────────────────────────────────────────────────
    // Three concentric rings open outward in sequence.
    let portal_params: [(f32, f32, f32, f32); 3] = [
        (1_400.0, 20.0, 1.0, 0.0),
        (850.0,   24.0, 1.6, 0.20),
        (420.0,   30.0, 2.2, 0.40),
    ];
    for &(radius, height, brightness, delay) in portal_params.iter() {
        let portal_mesh = meshes.add(Mesh::from(shape::Cylinder {
            radius,
            height,
            resolution: 24,
            segments: 1,
        }));
        let portal_mat = materials.add(StandardMaterial {
            base_color: Color::rgba(0.15, 0.0, 0.6, 0.8),
            emissive: Color::rgb(0.4 * brightness, 0.0, 2.2 * brightness),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        });
        commands.spawn((
            PbrBundle {
                mesh: portal_mesh,
                material: portal_mat,
                transform: Transform::from_translation(spawn_pos)
                    .with_scale(Vec3::new(0.001, 1.0, 0.001)),
                ..default()
            },
            SpawnPortal { timer: -delay, max_time: 2.5 },
        ));
    }
}

// ── Movement: slowly track the player ─────────────────────────────────────────
pub fn alien_ship_movement_system(
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut aliens: Query<(&mut Transform, &AlienShip)>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<AlienShip>)>,
) {
    if paused.0 { return; }
    let Ok(cam) = camera_q.get_single() else { return };
    let dt = time.delta_seconds();

    for (mut transform, ship) in aliens.iter_mut() {
        let to_player = (cam.translation - transform.translation).normalize_or_zero();

        // Slowly tilt/face toward player keeping disc roughly level
        let target_look = Transform::from_translation(transform.translation)
            .looking_at(cam.translation, Vec3::Y);
        let lean_t = (0.8 * dt).clamp(0.0, 1.0);
        transform.rotation = transform.rotation.slerp(target_look.rotation, lean_t);

        // Gentle roll for that eerie alien feel
        transform.rotate_local_y(0.25 * dt);

        // Move toward player
        transform.translation += to_player * ship.speed * dt;
    }
}

// ── Shooting: fired missiles are identical to player-tracking missiles ────────
pub fn alien_ship_shoot_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut aliens: Query<(&Transform, &mut AlienShip)>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<AlienShip>)>,
) {
    if paused.0 { return; }
    let Ok(cam) = camera_q.get_single() else { return };
    let dt = time.delta_seconds();

    for (transform, mut ship) in aliens.iter_mut() {
        ship.shoot_timer -= dt;
        if ship.shoot_timer > 0.0 { continue; }
        ship.shoot_timer = ship.shoot_interval;

        let toward_player = (cam.translation - transform.translation).normalize_or_zero();
        let missile_rot = Quat::from_rotation_arc(Vec3::Y, toward_player);
        // Fire from slightly in front of the ship
        let spawn_pos = transform.translation + toward_player * 320.0;

        let body_mesh = meshes.add(Mesh::from(shape::Cylinder {
            radius: 7.0,
            height: 60.0,
            resolution: 10,
            segments: 1,
        }));
        let body_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(0.25, 0.0, 0.75),
            emissive: Color::rgb(2.0, 0.0, 5.0),
            perceptual_roughness: 0.4,
            metallic: 0.7,
            ..default()
        });
        let glow_mesh = meshes.add(Mesh::from(shape::UVSphere {
            radius: 11.0,
            sectors: 8,
            stacks: 4,
        }));
        let glow_mat = materials.add(StandardMaterial {
            base_color: Color::rgba(0.6, 0.0, 1.0, 0.9),
            emissive: Color::rgb(4.0, 0.0, 8.0),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        });

        commands
            .spawn((
                PbrBundle {
                    mesh: body_mesh,
                    material: body_mat,
                    transform: Transform::from_translation(spawn_pos).with_rotation(missile_rot),
                    ..default()
                },
                Missile { speed: 20_000.0, turn_rate: 1.4, lifetime: 15.0 },
            ))
            .with_children(|p| {
                p.spawn(PbrBundle {
                    mesh: glow_mesh,
                    material: glow_mat,
                    transform: Transform::from_translation(Vec3::new(0.0, -32.0, 0.0)),
                    ..default()
                });
            });
    }
}

// ── Cleanup on state exit ─────────────────────────────────────────────────────
pub fn despawn_alien_ships(
    mut commands: Commands,
    aliens: Query<Entity, With<AlienShip>>,
    portals: Query<Entity, With<SpawnPortal>>,
) {
    for e in aliens.iter() { commands.entity(e).despawn_recursive(); }
    for e in portals.iter() { commands.entity(e).despawn_recursive(); }
}
