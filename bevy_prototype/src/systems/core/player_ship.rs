use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::components::{MainCamera, PlayerShipModel, SceneEntity};
use crate::resources::{CameraMode, GameState, TimePaused};
use crate::systems::data_loader::{SkinCatalog, SkinDef, SkinPart};
use crate::systems::ui::copilot_chat::LlmChatState;

/// Resource holding the current roll angle of the player ship (radians).
#[derive(Resource, Default)]
pub struct ShipRollState {
    /// Current roll angle in radians — springs toward target and back to 0.
    pub current_roll: f32,
}

/// Spawns the player ship as a child of the main camera.
/// The ship is entirely driven by the active skin's JSON definition
/// (`data/skins/<id>.json`).  No skin-specific code exists here.
pub fn spawn_player_ship_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_q: Query<Entity, With<MainCamera>>,
    ship_skin: Res<crate::resources::ShipSkin>,
    cam_mode: Res<CameraMode>,
    skin_catalog: Res<SkinCatalog>,
) {
    let Ok(camera_entity) = camera_q.get_single() else { return };
    let initial_vis = if *cam_mode == CameraMode::ThirdPerson {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    // Look up the skin definition by id; fall back to a default so the ship
    // always spawns even if the JSON file is missing or the id is unknown.
    let fallback = SkinDef::default();
    let skin_def = skin_catalog.by_id(&ship_skin.0).unwrap_or(&fallback);

    let hull_color = skin_def.primary_color
        .map(|[r, g, b]| Color::rgb(r, g, b))
        .unwrap_or(Color::rgb(0.20, 0.22, 0.26));
    let accent_color = skin_def.secondary_color
        .map(|[r, g, b]| Color::rgb(r, g, b))
        .unwrap_or(Color::rgb(0.55, 0.10, 0.10));
    let glow_color = skin_def.emissive_color
        .map(|[r, g, b]| Color::rgb(r, g, b))
        .unwrap_or(Color::rgb(0.20, 0.55, 1.00));

    let root = commands
        .spawn((
            SpatialBundle {
                visibility: initial_vis,
                transform: Transform::from_xyz(0.0, -1.5, -10.0),
                ..default()
            },
            PlayerShipModel,
            SceneEntity,
        ))
        .id();

    build_ship_from_skin_def(skin_def, root, &mut commands, &mut meshes, &mut materials, hull_color, accent_color, glow_color);
    commands.entity(camera_entity).push_children(&[root]);
}

// ── Unified JSON-driven ship builder ──────────────────────────────────────────

/// Build a procedural 3-D ship into `root` from a `SkinDef`.
/// When `parts` is non-empty each part spawns with its own material derived
/// from per-part color overrides; otherwise a shape preset is used.
fn build_ship_from_skin_def(
    skin: &SkinDef,
    root: Entity,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    hull_color: Color,
    accent_color: Color,
    glow_color: Color,
) {
    // Parts-based composable skin — each part creates its own material.
    if !skin.parts.is_empty() {
        for part in &skin.parts {
            spawn_skin_part(part, root, commands, meshes, materials, hull_color, accent_color, glow_color);
        }
        return;
    }

    // Pre-create shared materials for the legacy preset shapes.
    let hull_mat = materials.add(StandardMaterial {
        base_color: hull_color,
        metallic: 0.70,
        perceptual_roughness: 0.30,
        ..default()
    });
    let accent_mat = materials.add(StandardMaterial {
        base_color: accent_color,
        metallic: 0.60,
        perceptual_roughness: 0.40,
        ..default()
    });
    let glow_mat = materials.add(StandardMaterial {
        base_color: glow_color,
        emissive: glow_color,
        metallic: 0.00,
        perceptual_roughness: 0.80,
        ..default()
    });

    match skin.shape.as_str() {
        "disc" | "ufo" => {
            // Flat disc body + dome on top
            let disc = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cylinder { radius: 1.8, height: 0.35, resolution: 16, segments: 1 })),
                material: hull_mat,
                ..default()
            }).id();
            let dome = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.8, sectors: 12, stacks: 8 })),
                material: accent_mat,
                transform: Transform::from_xyz(0.0, 0.3, 0.0),
                ..default()
            }).id();
            let eng_l = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.22, sectors: 8, stacks: 5 })),
                material: glow_mat.clone(),
                transform: Transform::from_xyz(-0.8, -0.25, 0.0),
                ..default()
            }).id();
            let eng_r = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.22, sectors: 8, stacks: 5 })),
                material: glow_mat,
                transform: Transform::from_xyz(0.8, -0.25, 0.0),
                ..default()
            }).id();
            commands.entity(root).push_children(&[disc, dome, eng_l, eng_r]);
        }

        "diamond" | "prism" => {
            // Angular elongated fighter
            let body = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box { min_x: -0.5, max_x: 0.5, min_y: -0.5, max_y: 0.5, min_z: -4.0, max_z: 2.5 })),
                material: hull_mat,
                ..default()
            }).id();
            let fin_l = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box { min_x: -3.5, max_x: 0.0, min_y: -0.08, max_y: 0.08, min_z: -0.5, max_z: 1.8 })),
                material: accent_mat.clone(),
                ..default()
            }).id();
            let fin_r = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box { min_x: 0.0, max_x: 3.5, min_y: -0.08, max_y: 0.08, min_z: -0.5, max_z: 1.8 })),
                material: accent_mat,
                ..default()
            }).id();
            let nozzle = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.35, sectors: 8, stacks: 5 })),
                material: glow_mat,
                transform: Transform::from_xyz(0.0, 0.0, 2.5),
                ..default()
            }).id();
            commands.entity(root).push_children(&[body, fin_l, fin_r, nozzle]);
        }

        "organic" | "flower" => {
            // Round core with two orbital rings (like a planet/flower)
            let core = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 1.0, sectors: 16, stacks: 10 })),
                material: hull_mat,
                ..default()
            }).id();
            let ring1 = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Torus {
                    radius: 1.6,
                    ring_radius: 0.18,
                    subdivisions_segments: 24,
                    subdivisions_sides: 6,
                })),
                material: accent_mat.clone(),
                ..default()
            }).id();
            let ring2 = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Torus {
                    radius: 1.6,
                    ring_radius: 0.14,
                    subdivisions_segments: 24,
                    subdivisions_sides: 6,
                })),
                material: accent_mat,
                transform: Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ..default()
            }).id();
            let nozzle = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.3, sectors: 8, stacks: 5 })),
                material: glow_mat,
                transform: Transform::from_xyz(0.0, 0.0, 1.1),
                ..default()
            }).id();
            commands.entity(root).push_children(&[core, ring1, ring2, nozzle]);
        }

        "cylinder" | "pod" => {
            // Elongated capsule pod
            let body = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Capsule {
                    radius: 0.5,
                    rings: 4,
                    depth: 4.5,
                    latitudes: 8,
                    longitudes: 8,
                    uv_profile: shape::CapsuleUvProfile::Uniform,
                })),
                material: hull_mat,
                transform: Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ..default()
            }).id();
            let fin_top = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box { min_x: -0.08, max_x: 0.08, min_y: 0.45, max_y: 1.6, min_z: -0.5, max_z: 0.8 })),
                material: accent_mat.clone(),
                ..default()
            }).id();
            let fin_l = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box { min_x: -1.6, max_x: -0.45, min_y: -0.08, max_y: 0.08, min_z: -0.5, max_z: 0.8 })),
                material: accent_mat,
                ..default()
            }).id();
            let nozzle = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.28, sectors: 8, stacks: 5 })),
                material: glow_mat,
                transform: Transform::from_xyz(0.0, 0.0, 2.5),
                ..default()
            }).id();
            commands.entity(root).push_children(&[body, fin_top, fin_l, nozzle]);
        }

        _ => {
            // Default "sphere" — generic round spacecraft
            let sphere = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.9, sectors: 16, stacks: 10 })),
                material: hull_mat,
                ..default()
            }).id();
            let ring = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Torus {
                    radius: 1.5,
                    ring_radius: 0.22,
                    subdivisions_segments: 24,
                    subdivisions_sides: 6,
                })),
                material: accent_mat,
                transform: Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_4)),
                ..default()
            }).id();
            let nozzle_l = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.25, sectors: 8, stacks: 5 })),
                material: glow_mat.clone(),
                transform: Transform::from_xyz(-0.4, 0.0, 0.9),
                ..default()
            }).id();
            let nozzle_r = commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere { radius: 0.25, sectors: 8, stacks: 5 })),
                material: glow_mat,
                transform: Transform::from_xyz(0.4, 0.0, 0.9),
                ..default()
            }).id();
            commands.entity(root).push_children(&[sphere, ring, nozzle_l, nozzle_r]);
        }
    }
}

/// Spawns a single geometric part as a child of `root`, creating a
/// per-part material that respects color_rgb / emissive_rgb overrides.
fn spawn_skin_part(
    part: &SkinPart,
    root: Entity,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    hull_color: Color,
    accent_color: Color,
    glow_color: Color,
) {
    let material = build_part_material(part, materials, hull_color, accent_color, glow_color);

    let pos   = part.pos.map(|[x, y, z]| Vec3::new(x, y, z)).unwrap_or(Vec3::ZERO);
    let rot   = part.rot.map(|[rx, ry, rz]| {
        Quat::from_euler(EulerRot::XYZ, rx.to_radians(), ry.to_radians(), rz.to_radians())
    }).unwrap_or(Quat::IDENTITY);
    let scale = part.scale.map(|[x, y, z]| Vec3::new(x, y, z)).unwrap_or(Vec3::ONE);
    let transform = Transform { translation: pos, rotation: rot, scale };

    let mesh = match part.shape.as_str() {
        "sphere" => {
            let r = part.radius.unwrap_or(0.5);
            meshes.add(Mesh::from(shape::UVSphere { radius: r, sectors: 16, stacks: 10 }))
        }
        "icosphere" => {
            let r = part.radius.unwrap_or(0.5);
            meshes.add(
                Mesh::try_from(shape::Icosphere { radius: r, subdivisions: 3 })
                    .unwrap_or_else(|_| Mesh::from(shape::UVSphere { radius: r, sectors: 16, stacks: 10 }))
            )
        }
        "box" => {
            let [sx, sy, sz] = part.size.unwrap_or([1.0, 1.0, 1.0]);
            meshes.add(Mesh::from(shape::Box {
                min_x: -sx * 0.5, max_x: sx * 0.5,
                min_y: -sy * 0.5, max_y: sy * 0.5,
                min_z: -sz * 0.5, max_z: sz * 0.5,
            }))
        }
        "cylinder" => {
            let r = part.radius.unwrap_or(0.5);
            let h = part.height.unwrap_or(1.0);
            meshes.add(Mesh::from(shape::Cylinder { radius: r, height: h, resolution: 16, segments: 1 }))
        }
        "capsule" => {
            let r = part.radius.unwrap_or(0.3);
            let d = part.height.unwrap_or(1.0);
            meshes.add(Mesh::from(shape::Capsule {
                radius: r, rings: 4, depth: d,
                latitudes: 8, longitudes: 8,
                uv_profile: shape::CapsuleUvProfile::Uniform,
            }))
        }
        "torus" => {
            let r  = part.radius.unwrap_or(1.0);
            let rr = part.ring_radius.unwrap_or(0.2);
            meshes.add(Mesh::from(shape::Torus {
                radius: r, ring_radius: rr,
                subdivisions_segments: 24, subdivisions_sides: 6,
            }))
        }
        "cone" => {
            let r = part.radius.unwrap_or(0.5);
            let h = part.height.unwrap_or(1.0);
            meshes.add(create_cone_mesh(r, h))
        }
        _ => meshes.add(Mesh::from(shape::UVSphere { radius: 0.3, sectors: 8, stacks: 6 })),
    };

    let child = commands.spawn(PbrBundle { mesh, material, transform, ..default() }).id();
    commands.entity(root).push_children(&[child]);
}

/// Builds a `StandardMaterial` for a single skin part, honouring per-part
/// color_rgb / emissive_rgb / metallic / roughness overrides before falling
/// back to the named colour slot defaults.
fn build_part_material(
    part: &SkinPart,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    hull_color: Color,
    accent_color: Color,
    glow_color: Color,
) -> Handle<StandardMaterial> {
    let (slot_base, slot_emissive, slot_metallic, slot_roughness): (Color, Color, f32, f32) =
        match part.color.as_str() {
            "accent" => (accent_color, Color::BLACK, 0.60, 0.40),
            "glow"   => (glow_color,  glow_color,   0.00, 0.80),
            _        => (hull_color,  Color::BLACK,  0.70, 0.30),
        };

    let base_color = part.color_rgb
        .map(|[r, g, b]| Color::rgb(r, g, b))
        .unwrap_or(slot_base);

    let emissive = part.emissive_rgb
        .map(|[r, g, b]| Color::rgb(r, g, b))
        .unwrap_or(slot_emissive);

    materials.add(StandardMaterial {
        base_color,
        emissive,
        metallic:             part.metallic.unwrap_or(slot_metallic),
        perceptual_roughness: part.roughness.unwrap_or(slot_roughness),
        ..default()
    })
}

/// Builds a smooth cone mesh with apex at +Y and base at −Y.
/// To point the apex forward (−Z) in ship space, use `rot: [-90, 0, 0]`.
fn create_cone_mesh(radius: f32, height: f32) -> Mesh {
    const RES: u32 = 16;
    let half_h    = height * 0.5;
    let slope_len = (radius * radius + height * height).sqrt();
    let cos_s     = height / slope_len;  // outward-normal Y component
    let sin_s     = radius / slope_len;  // outward-normal radial component

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut uvs:       Vec<[f32; 2]> = Vec::new();
    let mut indices:   Vec<u32>      = Vec::new();

    // Side triangles — separate vertices per triangle for sharp normals
    for i in 0..RES {
        let a0 = (i as f32 / RES as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / RES as f32) * std::f32::consts::TAU;
        let am = (a0 + a1) * 0.5;
        let base = positions.len() as u32;

        // Apex
        positions.push([0.0, half_h, 0.0]);
        normals.push([am.cos() * cos_s, sin_s, am.sin() * cos_s]);
        uvs.push([0.5, 0.0]);

        // Left base vertex
        positions.push([a0.cos() * radius, -half_h, a0.sin() * radius]);
        normals.push([a0.cos() * cos_s, sin_s, a0.sin() * cos_s]);
        uvs.push([i as f32 / RES as f32, 1.0]);

        // Right base vertex
        positions.push([a1.cos() * radius, -half_h, a1.sin() * radius]);
        normals.push([a1.cos() * cos_s, sin_s, a1.sin() * cos_s]);
        uvs.push([(i + 1) as f32 / RES as f32, 1.0]);

        indices.extend([base, base + 1, base + 2]);
    }

    // Bottom cap
    let cap_center = positions.len() as u32;
    positions.push([0.0, -half_h, 0.0]);
    normals.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    let cap_ring = positions.len() as u32;
    for i in 0..RES {
        let a = (i as f32 / RES as f32) * std::f32::consts::TAU;
        positions.push([a.cos() * radius, -half_h, a.sin() * radius]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5 + a.cos() * 0.5, 0.5 + a.sin() * 0.5]);
    }
    for i in 0..RES {
        let next = (i + 1) % RES;
        indices.extend([cap_center, cap_ring + next, cap_ring + i]);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh
}

// ── Ship bank / roll animation ─────────────────────────────────────────────────

/// Applies a banking roll to the `PlayerShipModel` proportional to yaw input,
/// giving a realistic in-flight inclination in third-person view.
pub fn ship_bank_system(
    mut mouse_motion: EventReader<MouseMotion>,
    mut roll: ResMut<ShipRollState>,
    mut ship_q: Query<&mut Transform, With<PlayerShipModel>>,
    time: Res<Time>,
    chat: Res<LlmChatState>,
    paused: Res<TimePaused>,
    cam_mode: Res<CameraMode>,
    state: Res<State<GameState>>,
    free_look: Res<crate::resources::FreeLook>,
) {
    if paused.0 || chat.open || *state.get() != GameState::Playing { return; }
    if *cam_mode != CameraMode::ThirdPerson { return; }
    // In orbit mode the ship position/rotation is handled by orbit_ship_align_system.
    if free_look.active { return; }

    let dt = time.delta_seconds();

    // Sum all mouse X deltas this frame to get yaw input magnitude
    let total_dx: f32 = mouse_motion.iter().map(|e| e.delta.x).sum();

    // Target roll proportional to yaw input, capped at ~40 degrees
    let target_roll = (-total_dx * 0.025).clamp(-0.70, 0.70);

    // Spring-damper: smoothly approach target
    let spring = 9.0;
    roll.current_roll += (target_roll - roll.current_roll) * (spring * dt).min(1.0);

    // Fade back to 0 when no input
    if total_dx.abs() < 0.5 {
        let fade = 3.5;
        roll.current_roll *= 1.0 - (fade * dt).min(1.0);
    }

    for mut transform in &mut ship_q {
        transform.rotation = Quat::from_rotation_z(-roll.current_roll);
    }
}
