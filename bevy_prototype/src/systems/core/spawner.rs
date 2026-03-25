#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use rand::Rng;

use crate::components::*;
use crate::resources::{AsteroidSpawnTimer, TimePaused};

pub fn build_asteroid_mesh(radius: f32, rng: &mut impl Rng) -> Mesh {
    let mut mesh = Mesh::from(shape::UVSphere {
        radius: 1.0,
        sectors: 12,
        stacks: 8,
    });

    let stretch_x = rng.gen_range(0.5..1.6);
    let stretch_y = rng.gen_range(0.5..1.6);
    let stretch_z = rng.gen_range(0.5..1.6);
    let noise_a = rng.gen_range(0.9..2.4);
    let noise_b = rng.gen_range(0.9..2.4);
    let noise_c = rng.gen_range(0.9..2.4);
    let noise_phase = rng.gen_range(0.0..std::f32::consts::TAU);

    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for position in positions.iter_mut() {
            let mut point = Vec3::new(position[0], position[1], position[2]);
            let direction = point.normalize_or_zero();
            let surface_noise = (direction.x * noise_a + noise_phase).sin() * 0.36
                + (direction.y * noise_b - noise_phase * 0.6).cos() * 0.28
                + (direction.z * noise_c + noise_phase * 1.3).sin() * 0.22;
            let crater = if direction.y > 0.25 { -0.22 * rng.gen_range(0.6..1.0) } else { 0.0 };
            let irregularity = (1.0 + surface_noise + crater + rng.gen_range(-0.12..0.12)).clamp(0.55, 1.55);

            point.x *= stretch_x * irregularity;
            point.y *= stretch_y * irregularity;
            point.z *= stretch_z * irregularity;
            point *= radius;

            *position = [point.x, point.y, point.z];
        }
    }

    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}

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
) -> Vec3 {
    let orbit_radius = rng.gen_range(400.0..820.0);
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let y = rng.gen_range(-18.0..18.0);
    let radius = rng.gen_range(6.0..18.0);

    let x = angle.cos() * orbit_radius;
    let z = angle.sin() * orbit_radius;
    let position = Vec3::new(x, y, z);

    let ang_vel = Vec3::new(
        rng.gen_range(-0.55..0.55),
        rng.gen_range(-0.55..0.55),
        rng.gen_range(-0.55..0.55),
    );

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(build_asteroid_mesh(radius, rng)),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.34, 0.32, 0.30),
                perceptual_roughness: 1.0,
                metallic: 0.0,
                reflectance: 0.04,
                ..default()
            }),
            transform: Transform::from_translation(position),
            ..default()
        },
        Asteroid,
        Velocity(Vec3::ZERO),
        Radius(radius),
        AngularVelocity(ang_vel),
    ));

    position
}
