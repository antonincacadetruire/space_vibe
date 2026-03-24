use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

use crate::components::{Asteroid, AngularVelocity, Radius, SceneEntity, SkyDome, Velocity};

/// Spawns the ice-cavern-inside-a-gigantic-asteroid scene.
/// Returns the player's start transform.
pub fn spawn_ice_caves_scene(
    commands: &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rng:       &mut impl Rng,
) -> Transform {
    // ── Ambient: cold blue darkness ────────────────────────────────────────────
    commands.insert_resource(AmbientLight {
        color: Color::rgb(0.06, 0.12, 0.22),
        brightness: 0.18,
    });

    // ── Cave shell — huge hollow sphere the player flies inside ────────────────
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 180_000.0,
                sectors: 48,
                stacks: 28,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.10, 0.16, 0.28),
                emissive: Color::rgb(0.01, 0.03, 0.08),
                perceptual_roughness: 1.0,
                metallic: 0.0,
                cull_mode: None,
                unlit: false,
                ..default()
            }),
            ..default()
        },
        SkyDome,
        SceneEntity,
    ));

    // ── Ice-crystal glow veins on the cave walls ────────────────────────────────
    let vein_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(0.5, 0.8, 1.0, 0.6),
        emissive: Color::rgb(0.0, 2.5, 5.0),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        ..default()
    });
    for _ in 0..80 {
        let phi   = rng.gen_range(0.0_f32..TAU);
        let theta = rng.gen_range(0.0_f32..std::f32::consts::PI);
        let r = 170_000.0_f32;
        let pos = Vec3::new(
            r * theta.sin() * phi.cos(),
            r * theta.cos(),
            r * theta.sin() * phi.sin(),
        );
        let length = rng.gen_range(3_000.0_f32..12_000.0);
        let width  = rng.gen_range(400.0_f32..1_200.0);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cylinder {
                    radius: width,
                    height: length,
                    resolution: 6,
                    segments: 1,
                })),
                material: vein_mat.clone(),
                transform: Transform::from_translation(pos)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Massive ice stalactites / stalagmites ──────────────────────────────────
    let ice_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.65, 0.82, 0.95),
        emissive: Color::rgb(0.0, 0.8, 1.5),
        perceptual_roughness: 0.15,
        metallic: 0.6,
        ..default()
    });
    let dark_ice_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.20, 0.30, 0.45),
        emissive: Color::rgb(0.0, 0.1, 0.3),
        perceptual_roughness: 0.6,
        metallic: 0.3,
        ..default()
    });

    for i in 0..60 {
        let angle = (i as f32 / 60.0) * TAU + rng.gen_range(-0.15_f32..0.15);
        let horiz_dist = rng.gen_range(8_000.0_f32..45_000.0);
        let height = rng.gen_range(-60_000.0_f32..60_000.0);
        let column_h = rng.gen_range(6_000.0_f32..28_000.0);
        let column_r = rng.gen_range(800.0_f32..3_500.0);
        let pos = Vec3::new(angle.cos() * horiz_dist, height, angle.sin() * horiz_dist);

        // Main column
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cylinder {
                    radius: column_r,
                    height: column_h,
                    resolution: 8,
                    segments: 1,
                })),
                material: if rng.gen_bool(0.35) { dark_ice_mat.clone() } else { ice_mat.clone() },
                transform: Transform::from_translation(pos),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Floating ice asteroids (obstacles) ──────────────────────────────────────
    let ice_rock_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.55, 0.72, 0.88),
        emissive: Color::rgb(0.0, 0.3, 0.7),
        perceptual_roughness: 0.5,
        metallic: 0.4,
        ..default()
    });
    for _ in 0..160 {
        let pos = Vec3::new(
            rng.gen_range(-60_000.0_f32..60_000.0),
            rng.gen_range(-50_000.0_f32..50_000.0),
            rng.gen_range(-60_000.0_f32..60_000.0),
        );
        let r = rng.gen_range(200.0_f32..2_200.0);
        let vel = Vec3::new(
            rng.gen_range(-12.0_f32..12.0),
            rng.gen_range(-8.0_f32..8.0),
            rng.gen_range(-12.0_f32..12.0),
        );
        let spin = Vec3::new(
            rng.gen_range(-0.15_f32..0.15),
            rng.gen_range(-0.15_f32..0.15),
            rng.gen_range(-0.15_f32..0.15),
        );
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(make_chunky_sphere_mesh(r, rng)),
                material: ice_rock_mat.clone(),
                transform: Transform::from_translation(pos),
                ..default()
            },
            Asteroid,
            Velocity(vel),
            AngularVelocity(spin),
            Radius(r),
            SceneEntity,
        ));
    }

    // ── Embedded blue point lights (glowing ice formations) ────────────────────
    for i in 0..12 {
        let angle = (i as f32 / 12.0) * TAU;
        let dist  = rng.gen_range(20_000.0_f32..60_000.0);
        let y     = rng.gen_range(-40_000.0_f32..40_000.0);
        commands.spawn((
            PointLightBundle {
                point_light: PointLight {
                    intensity: 12_000_000.0,
                    range: 60_000.0,
                    color: Color::rgb(0.3, 0.6, 1.0),
                    shadows_enabled: false,
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    angle.cos() * dist, y, angle.sin() * dist,
                )),
                ..default()
            },
            SceneEntity,
        ));
    }
    // A warm deep-violet "core" light in the center of the cave
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 30_000_000.0,
                range: 120_000.0,
                color: Color::rgb(0.45, 0.2, 0.9),
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, -20_000.0, 0.0)),
            ..default()
        },
        SceneEntity,
    ));

    // ── Player spawn: floating in the middle of the cave ───────────────────────
    let spawn_pos = Vec3::new(0.0, 5_000.0, 50_000.0);
    Transform::from_translation(spawn_pos)
        .looking_at(Vec3::ZERO, Vec3::Y)
}

/// Very simple lumpy sphere mesh for ice chunks.
fn make_chunky_sphere_mesh(radius: f32, rng: &mut impl Rng) -> Mesh {
    use bevy::render::mesh::VertexAttributeValues;
    let mut mesh = Mesh::from(shape::UVSphere {
        radius: 1.0,
        sectors: 10,
        stacks: 6,
    });
    let stretch_x = rng.gen_range(0.5_f32..1.6);
    let stretch_y = rng.gen_range(0.5_f32..1.6);
    let stretch_z = rng.gen_range(0.5_f32..1.6);
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in positions.iter_mut() {
            p[0] *= stretch_x * rng.gen_range(0.8_f32..1.2) * radius;
            p[1] *= stretch_y * rng.gen_range(0.8_f32..1.2) * radius;
            p[2] *= stretch_z * rng.gen_range(0.8_f32..1.2) * radius;
        }
    }
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}

