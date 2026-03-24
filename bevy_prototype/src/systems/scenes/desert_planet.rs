use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::{PI, TAU};

use crate::components::{Asteroid, AngularVelocity, Radius, SceneEntity, SkyDome, Velocity};

/// Spawns the desert planet surface scene.
/// Returns the player's start transform.
pub fn spawn_desert_planet_scene(
    commands: &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rng:       &mut impl Rng,
) -> Transform {
    // ── Harsh sun illumination ─────────────────────────────────────────────────
    commands.insert_resource(AmbientLight {
        color: Color::rgb(0.28, 0.20, 0.10),
        brightness: 0.65,
    });

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 110_000.0,
                color: Color::rgb(1.0, 0.86, 0.56),
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_rotation(
                Quat::from_euler(EulerRot::XYZ, -0.45, 0.8, 0.0)
            ),
            ..default()
        },
        SceneEntity,
    ));

    // ── Sky dome — warm orange horizon to deep red-brown zenith ────────────────
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 600_000.0,
                sectors: 48,
                stacks: 28,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.55, 0.22, 0.06),
                emissive: Color::rgb(0.15, 0.05, 0.01),
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            ..default()
        },
        SkyDome,
        SceneEntity,
    ));

    // Sun disk glare in the sky
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 18_000.0,
                sectors: 16,
                stacks: 8,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(1.0, 0.92, 0.5),
                emissive: Color::rgb(4.0, 2.8, 0.8),
                unlit: true,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(-180_000.0, 140_000.0, -280_000.0)),
            ..default()
        },
        SceneEntity,
    ));

    // ── Ground — enormous flat disc (extends far in all directions) ────────────
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.68, 0.48, 0.28),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 1_400_000.0,
                subdivisions: 0,
            })),
            material: ground_mat.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, -3_500.0, 0.0)),
            ..default()
        },
        SceneEntity,
    ));

    // ── Sand dune mounds ───────────────────────────────────────────────────────
    let dune_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.78, 0.58, 0.32),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    for _ in 0..120 {
        let x = rng.gen_range(-200_000.0_f32..200_000.0);
        let z = rng.gen_range(-200_000.0_f32..200_000.0);
        let rx = rng.gen_range(4_000.0_f32..28_000.0);
        let ry = rng.gen_range(400.0_f32..2_200.0);
        let rz = rng.gen_range(4_000.0_f32..28_000.0);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0,
                    sectors: 12,
                    stacks: 6,
                })),
                material: dune_mat.clone(),
                transform: Transform::from_translation(Vec3::new(x, -3_500.0 + ry * 0.3, z))
                    .with_scale(Vec3::new(rx, ry, rz)),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Mountain range on the horizon ──────────────────────────────────────────
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.38, 0.25, 0.15),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    let rock_dark_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.22, 0.14, 0.09),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    for i in 0..48 {
        let base_angle = (i as f32 / 48.0) * TAU + rng.gen_range(-0.04_f32..0.04);
        let dist = rng.gen_range(180_000.0_f32..280_000.0);
        let height = rng.gen_range(18_000.0_f32..55_000.0);
        let base_r = rng.gen_range(14_000.0_f32..40_000.0);
        let mountain_pos = Vec3::new(
            base_angle.cos() * dist,
            -3_500.0 + height * 0.4,
            base_angle.sin() * dist,
        );

        // Main peak cone
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0,
                    sectors: 10,
                    stacks: 6,
                })),
                material: if rng.gen_bool(0.4) { rock_dark_mat.clone() } else { rock_mat.clone() },
                transform: Transform::from_translation(mountain_pos)
                    .with_scale(Vec3::new(base_r, height, base_r)),
                ..default()
            },
            SceneEntity,
        ));
        // Secondary shoulder
        if rng.gen_bool(0.5) {
            let shoulder_off = Vec3::new(
                rng.gen_range(-8_000.0_f32..8_000.0),
                0.0,
                rng.gen_range(-8_000.0_f32..8_000.0),
            );
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 1.0,
                        sectors: 8,
                        stacks: 4,
                    })),
                    material: rock_mat.clone(),
                    transform: Transform::from_translation(mountain_pos + shoulder_off)
                        .with_scale(Vec3::new(
                            base_r * 0.55,
                            height * rng.gen_range(0.45_f32..0.75),
                            base_r * 0.55,
                        )),
                    ..default()
                },
                SceneEntity,
            ));
        }
    }

    // ── Desert boulders / floating rock debris ─────────────────────────────────
    let boulder_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.50, 0.35, 0.20),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    for _ in 0..200 {
        let x = rng.gen_range(-80_000.0_f32..80_000.0);
        let y = rng.gen_range(-2_000.0_f32..15_000.0);
        let z = rng.gen_range(-80_000.0_f32..80_000.0);
        let r = rng.gen_range(120.0_f32..1_800.0);
        let vel = Vec3::new(
            rng.gen_range(-5.0_f32..5.0),
            rng.gen_range(-2.0_f32..2.0),
            rng.gen_range(-5.0_f32..5.0),
        );
        let spin = Vec3::new(
            rng.gen_range(-0.08_f32..0.08),
            rng.gen_range(-0.08_f32..0.08),
            rng.gen_range(-0.08_f32..0.08),
        );
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(make_boulder_mesh(r, rng)),
                material: boulder_mat.clone(),
                transform: Transform::from_translation(Vec3::new(x, y, z)),
                ..default()
            },
            Asteroid,
            Velocity(vel),
            AngularVelocity(spin),
            Radius(r),
            SceneEntity,
        ));
    }

    // ── Heat haze point lights near the ground ─────────────────────────────────
    for i in 0..8 {
        let angle = (i as f32 / 8.0) * TAU;
        let dist  = rng.gen_range(5_000.0_f32..30_000.0);
        commands.spawn((
            PointLightBundle {
                point_light: PointLight {
                    intensity: 8_000_000.0,
                    range: 40_000.0,
                    color: Color::rgb(1.0, 0.50, 0.10),
                    shadows_enabled: false,
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    angle.cos() * dist, -2_000.0, angle.sin() * dist,
                )),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Player spawn above the desert surface ─────────────────────────────────
    Transform::from_translation(Vec3::new(0.0, 8_000.0, 30_000.0))
        .looking_at(Vec3::new(0.0, 8_000.0, 0.0), Vec3::Y)
}

fn make_boulder_mesh(radius: f32, rng: &mut impl Rng) -> Mesh {
    use bevy::render::mesh::VertexAttributeValues;
    let mut mesh = Mesh::from(shape::UVSphere {
        radius: 1.0,
        sectors: 8,
        stacks: 4,
    });
    let sx = rng.gen_range(0.6_f32..1.4);
    let sy = rng.gen_range(0.5_f32..1.1);
    let sz = rng.gen_range(0.6_f32..1.4);
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in positions.iter_mut() {
            p[0] *= sx * rng.gen_range(0.85_f32..1.15) * radius;
            p[1] *= sy * rng.gen_range(0.85_f32..1.15) * radius;
            p[2] *= sz * rng.gen_range(0.85_f32..1.15) * radius;
        }
    }
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}

