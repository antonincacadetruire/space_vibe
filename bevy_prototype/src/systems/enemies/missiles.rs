use bevy::prelude::*;
use rand::Rng;

use crate::components::{Explosion, MainCamera, Missile};
use crate::resources::{DeathCause, GameState, GameTimer, MissileSpawnTimer, TimePaused};

// ── Difficulty: missiles start appearing after this many seconds ─────────────
const FIRST_MISSILE_DELAY: f32 = 10.0;
/// Spawn interval shrinks as time progresses, bottoms out at 4 s.
fn spawn_interval(elapsed: f32) -> f32 {
    (18.0 - elapsed * 0.04).clamp(4.0, 18.0)
}

// ── Spawner ───────────────────────────────────────────────────────────────────
pub fn missile_spawner_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    game_timer: Res<GameTimer>,
    mut spawn_timer: ResMut<MissileSpawnTimer>,
    paused: Res<TimePaused>,
    camera_q: Query<&Transform, With<MainCamera>>,
) {
    if paused.0 { return; }
    if game_timer.0 < FIRST_MISSILE_DELAY { return; }

    let desired_interval = spawn_interval(game_timer.0);
    spawn_timer.0.set_duration(std::time::Duration::from_secs_f32(desired_interval));
    spawn_timer.0.tick(time.delta());
    if !spawn_timer.0.just_finished() { return; }

    let Ok(cam) = camera_q.get_single() else { return };
    let mut rng = rand::thread_rng();

    // Spawn behind and to the side of the player
    let offset = cam.rotation.mul_vec3(Vec3::Z) * rng.gen_range(1_200.0..2_400.0)
        + Vec3::new(
            rng.gen_range(-400.0..400.0),
            rng.gen_range(-200.0..200.0),
            rng.gen_range(-400.0..400.0),
        );
    let spawn_pos = cam.translation + offset;

    let speed = rng.gen_range(18_000.0_f32..26_000.0);
    let turn_rate = rng.gen_range(1.2_f32..2.2);

    // Torpedo body: elongated cylinder
    let body_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 16.0,
        height: 130.0,
        resolution: 10,
        segments: 1,
    }));

    // Neon red emissive material
    let body_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.9, 0.1, 0.05),
        emissive: Color::rgb(3.0, 0.2, 0.05),
        perceptual_roughness: 0.4,
        metallic: 0.7,
        ..default()
    });

    // Engine glow disc at tail
    let glow_mesh = meshes.add(Mesh::from(shape::UVSphere { radius: 26.0, sectors: 8, stacks: 4 }));
    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(1.0, 0.4, 0.0, 0.9),
        emissive: Color::rgb(6.0, 1.5, 0.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });

    // Initial direction toward player
    let toward_player = (cam.translation - spawn_pos).normalize_or_zero();
    let missile_rot = Quat::from_rotation_arc(Vec3::Y, toward_player);

    commands
        .spawn((
            PbrBundle {
                mesh: body_mesh,
                material: body_mat,
                transform: Transform::from_translation(spawn_pos)
                    .with_rotation(missile_rot),
                ..default()
            },
            Missile { speed, turn_rate, lifetime: 14.0 },
        ))
        .with_children(|p| {
            // engine glow at tail
            p.spawn(PbrBundle {
                mesh: glow_mesh,
                material: glow_mat,
                transform: Transform::from_translation(Vec3::new(0.0, -72.0, 0.0)),
                ..default()
            });
        });
}

// ── Guidance & movement ───────────────────────────────────────────────────────
pub fn missile_movement_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut missiles: Query<(Entity, &mut Transform, &mut Missile)>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<Missile>)>,
    game_timer: Res<GameTimer>,
    mut death_cause: ResMut<DeathCause>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if paused.0 { return; }
    let Ok(cam) = camera_q.get_single() else { return };
    let dt = time.delta_seconds();

    for (entity, mut transform, mut missile) in missiles.iter_mut() {
        missile.lifetime -= dt;
        if missile.lifetime <= 0.0 {
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 1.0,
                        sectors: 10,
                        stacks: 5,
                    })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(1.0, 0.4, 0.0),
                        emissive: Color::rgb(8.0, 2.0, 0.0),
                        alpha_mode: AlphaMode::Add,
                        unlit: true,
                        ..default()
                    }),
                    transform: Transform::from_translation(transform.translation),
                    ..default()
                },
                Explosion { timer: 0.0, max_time: 0.8, max_scale: 500.0 },
            ));
            commands.entity(entity).despawn_recursive();
            continue;
        }

        // Proportional-navigation: steer toward player
        let to_target = (cam.translation - transform.translation).normalize_or_zero();
        // Current heading is +Y for the cylinder
        let current_forward = transform.rotation.mul_vec3(Vec3::Y).normalize_or_zero();

        // Slerp toward target direction
        let target_rot = Quat::from_rotation_arc(current_forward, to_target);
        let max_angle = missile.turn_rate * dt;
        let angle = target_rot.to_axis_angle().1; // always positive
        let t = if angle < max_angle { 1.0 } else { max_angle / angle };
        transform.rotation = transform.rotation.slerp(
            Quat::from_rotation_arc(Vec3::Y, to_target) * Quat::IDENTITY,
            t,
        );

        // Move along heading
        let heading = transform.rotation.mul_vec3(Vec3::Y).normalize_or_zero();
        transform.translation += heading * missile.speed * dt;

        // Collision check vs player
        let dist = transform.translation.distance(cam.translation);
        if dist < 40.0 {
            info!("Missile hit! Score: {:.1}s", game_timer.0);
            commands.entity(entity).despawn_recursive();
            *death_cause = DeathCause::Missile;
            next_state.set(GameState::Dead);
        }
    }
}

// ── Despawn all missiles on state change ─────────────────────────────────────
pub fn despawn_missiles(mut commands: Commands, q: Query<Entity, With<Missile>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}
