use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

use crate::components::{Asteroid, AngularVelocity, Radius, SceneEntity, SkyDome, Velocity};
use crate::resources::DesertTerrainData;

const FLOOR_Y: f32 = -3_500.0;
/// Kill threshold: camera centre must dip below this Y to trigger terrain death.
const FLOOR_KILL_Y: f32 = FLOOR_Y + 300.0;

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

    // ── Sky dome — deep crimson above, molten-orange horizon ──────────────────
    // Outer dome (seen from inside — renders the distant horizon colour).
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 600_000.0,
                sectors: 48,
                stacks: 28,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.60, 0.18, 0.04),
                emissive: Color::rgb(0.18, 0.05, 0.01),
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            ..default()
        },
        SkyDome,
        SceneEntity,
    ));
    // Inner mid-sky sphere — darker reddish band above the horizon.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 500_000.0,
                sectors: 32,
                stacks: 20,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.22, 0.06, 0.02),
                emissive: Color::rgb(0.06, 0.01, 0.00),
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            ..default()
        },
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

    // ── Ground — enormous flat disc ────────────────────────────────────────────
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.72, 0.50, 0.28),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    // Shadow-tone under-layer slightly lower, gives depth impression.
    let ground_dark_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.45, 0.28, 0.12),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 1_400_000.0, subdivisions: 0 })),
            material: ground_mat.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, FLOOR_Y, 0.0)),
            ..default()
        },
        SceneEntity,
    ));
    // Dark crack patterns across the ground (overlapping smaller planes).
    for _ in 0..60 {
        let x = rng.gen_range(-300_000.0_f32..300_000.0);
        let z = rng.gen_range(-300_000.0_f32..300_000.0);
        let size = rng.gen_range(8_000.0_f32..60_000.0);
        let rot_y = rng.gen_range(0.0_f32..TAU);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Plane { size: 1.0, subdivisions: 0 })),
                material: ground_dark_mat.clone(),
                transform: Transform::from_translation(Vec3::new(x, FLOOR_Y + 1.0, z))
                    .with_rotation(Quat::from_rotation_y(rot_y))
                    .with_scale(Vec3::new(size, 1.0, size * rng.gen_range(0.2_f32..1.0))),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Sand dune mounds ───────────────────────────────────────────────────────
    let mut kill_zones: Vec<(Vec3, f32, f32)> = Vec::new();

    let dune_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.78, 0.58, 0.32),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    for _ in 0..150 {
        let x = rng.gen_range(-200_000.0_f32..200_000.0);
        let z = rng.gen_range(-200_000.0_f32..200_000.0);
        let rx = rng.gen_range(4_000.0_f32..28_000.0);
        let ry = rng.gen_range(400.0_f32..2_200.0);
        let rz = rng.gen_range(4_000.0_f32..28_000.0);
        let dune_pos = Vec3::new(x, FLOOR_Y + ry * 0.3, z);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0,
                    sectors: 12,
                    stacks: 6,
                })),
                material: dune_mat.clone(),
                transform: Transform::from_translation(dune_pos)
                    .with_scale(Vec3::new(rx, ry, rz)),
                ..default()
            },
            SceneEntity,
        ));
        // Dunes are flat mounds — add ellipsoid kill zone
        kill_zones.push((dune_pos, (rx + rz) * 0.35, ry * 0.7));
    }

    // ── Mountain range on the horizon ──────────────────────────────────────────
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.32, 0.18, 0.09),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    let rock_dark_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.16, 0.09, 0.05),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });
    // Shadow-face: a darker tone used for the "leeward" side of peaks.
    let rock_shadow_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.10, 0.06, 0.03),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        ..default()
    });

    for i in 0..56 {
        let base_angle = (i as f32 / 56.0) * TAU + rng.gen_range(-0.05_f32..0.05);
        let dist      = rng.gen_range(160_000.0_f32..300_000.0);
        let height    = rng.gen_range(22_000.0_f32..80_000.0);
        let base_r    = rng.gen_range(18_000.0_f32..50_000.0);
        let mountain_pos = Vec3::new(
            base_angle.cos() * dist,
            FLOOR_Y + height * 0.4,
            base_angle.sin() * dist,
        );

        // Store kill ellipsoid — horizontal radius slightly smaller than visual,
        // vertical radius matches the full height.
        kill_zones.push((mountain_pos, base_r * 0.80, height * 0.80));

        let mat = if rng.gen_bool(0.35) { rock_dark_mat.clone() } else { rock_mat.clone() };

        // Main peak
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0,
                    sectors: 12,
                    stacks: 7,
                })),
                material: mat.clone(),
                transform: Transform::from_translation(mountain_pos)
                    .with_scale(Vec3::new(base_r, height, base_r)),
                ..default()
            },
            SceneEntity,
        ));
        // Shadow-face offset (dark sub-peak slightly behind gives depth).
        let shadow_off = Vec3::new(
            rng.gen_range(-4_000.0_f32..4_000.0),
            -height * 0.12,
            rng.gen_range(-4_000.0_f32..4_000.0),
        );
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0, sectors: 10, stacks: 5,
                })),
                material: rock_shadow_mat.clone(),
                transform: Transform::from_translation(mountain_pos + shadow_off)
                    .with_scale(Vec3::new(base_r * 0.88, height * 0.95, base_r * 0.88)),
                ..default()
            },
            SceneEntity,
        ));
        // Secondary shoulder
        if rng.gen_bool(0.6) {
            let shoulder_off = Vec3::new(
                rng.gen_range(-12_000.0_f32..12_000.0),
                0.0,
                rng.gen_range(-12_000.0_f32..12_000.0),
            );
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 1.0, sectors: 8, stacks: 4,
                    })),
                    material: if rng.gen_bool(0.5) { rock_dark_mat.clone() } else { rock_mat.clone() },
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

    // ── Distant rock spires / monoliths (inside playable area) ────────────────
    let spire_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.26, 0.14, 0.07),
        emissive: Color::rgb(0.04, 0.01, 0.00),
        perceptual_roughness: 0.9,
        ..default()
    });
    for _ in 0..30 {
        let angle = rng.gen_range(0.0_f32..TAU);
        let dist  = rng.gen_range(40_000.0_f32..130_000.0);
        let h     = rng.gen_range(4_000.0_f32..18_000.0);
        let r     = rng.gen_range(600.0_f32..3_000.0);
        let pos   = Vec3::new(angle.cos() * dist, FLOOR_Y + h * 0.5, angle.sin() * dist);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 1.0, sectors: 8, stacks: 5,
                })),
                material: spire_mat.clone(),
                transform: Transform::from_translation(pos)
                    .with_scale(Vec3::new(r, h, r)),
                ..default()
            },
            SceneEntity,
        ));
        // Add smaller spires as kill spheres too (they're smaller but still dangerous).
        kill_zones.push((pos, r * 0.9, h * 0.9));
    }

    // ── Atmospheric dust haze ring (semi-transparent sphere near ground) ───────
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 280_000.0,
                sectors: 24,
                stacks: 12,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(0.65, 0.32, 0.10, 0.18),
                emissive: Color::rgb(0.08, 0.03, 0.00),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, FLOOR_Y + 8_000.0, 0.0)),
            ..default()
        },
        SceneEntity,
    ));

    // Insert terrain data so the death system can use it.
    commands.insert_resource(DesertTerrainData {
        floor_y: FLOOR_KILL_Y,
        kill_zones,
    });

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
    Transform::from_translation(Vec3::new(0.0, 8_000.0, 60_000.0))
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

