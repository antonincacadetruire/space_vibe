use bevy::prelude::*;
use rand::Rng;

use crate::components::*;
use crate::resources::{AsteroidSpawnTimer, TimePaused};

pub fn asteroid_spawner_system(
    time: Res<Time>,
    mut timer: ResMut<AsteroidSpawnTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    paused: Res<TimePaused>,
) {
    if paused.0 {
        return;
    }

    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        let mut rng = rand::thread_rng();
        spawn_asteroid(&mut commands, &mut rng, &mut meshes, &mut materials);
    }
}

pub fn spawn_asteroid(
    commands: &mut Commands,
    rng: &mut impl Rng,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let x = rng.gen_range(-1200.0..1200.0);
    let z = rng.gen_range(-1200.0..1200.0);
    let y = rng.gen_range(200.0..1600.0);
    let radius = rng.gen_range(8.0..40.0);
    let vx = rng.gen_range(-30.0..30.0);
    let vz = rng.gen_range(-30.0..30.0);
    let vy = rng.gen_range(-80.0..-30.0);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere { radius, sectors: 16, stacks: 16 })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.5, 0.45, 0.4),
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(x, y, z)),
            ..default()
        },
        Asteroid,
        Velocity(Vec3::new(vx, vy, vz)),
        Radius(radius),
    ));
}
