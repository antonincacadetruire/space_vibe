use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

use crate::components::{MainCamera, PlayerShipModel, SceneEntity};
use crate::resources::{CameraMode, GameState, TimePaused};
use crate::systems::data_loader::SkinCatalog;
use crate::systems::ui::copilot_chat::LlmChatState;

/// Resource holding the current roll angle of the player ship (radians).
#[derive(Resource, Default)]
pub struct ShipRollState {
    /// Current roll angle in radians — springs toward target and back to 0.
    pub current_roll: f32,
}

/// Spawns a procedural war-plane model as a child of the main camera.
/// The ship sits slightly in front of and below the camera so the player can
/// always see it — giving a third-person-behind-the-cockpit perspective.
///
/// The model uses only Bevy primitive shapes (no external assets) so it works
/// out of the box.  In the future, swap `build_war_plane` for other builders
/// to implement different `ShipSkin` options.
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
    // Ship starts visible in ThirdPerson mode, hidden in FirstPerson.
    let initial_vis = if *cam_mode == CameraMode::ThirdPerson {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    // ── Materials ─────────────────────────────────────────────────────────────
    let hull = materials.add(StandardMaterial {
        base_color: Color::rgb(0.20, 0.22, 0.26),
        metallic: 0.85,
        perceptual_roughness: 0.25,
        reflectance: 0.6,
        ..default()
    });

    let accent = materials.add(StandardMaterial {
        base_color: Color::rgb(0.55, 0.10, 0.10),
        metallic: 0.7,
        perceptual_roughness: 0.35,
        ..default()
    });

    let cockpit = materials.add(StandardMaterial {
        base_color: Color::rgba(0.08, 0.55, 0.90, 0.75),
        emissive: Color::rgb(0.0, 0.30, 0.65),
        metallic: 0.2,
        perceptual_roughness: 0.05,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let engine_glow = materials.add(StandardMaterial {
        base_color: Color::rgb(0.20, 0.55, 1.00),
        emissive: Color::rgb(0.40, 1.20, 2.50),
        metallic: 0.0,
        perceptual_roughness: 0.8,
        ..default()
    });

    // ── Mesh helpers ──────────────────────────────────────────────────────────
    // The ship coordinate system: nose at -Z (forward), tail at +Z (toward cam).
    // Ship root is at (0, -1.5, -10) in camera-local space, putting the tail
    // ~7 units ahead of the camera so the player always sees the whole aircraft.

    // ── Custom (AI-generated) skins — build from SkinDef shape/color fields ─────
    if let crate::resources::ShipSkin::Custom(ref skin_id) = *ship_skin {
        let skin_def = skin_catalog.by_id(skin_id);
        let shape_name = skin_def.map(|s| s.shape.as_str()).unwrap_or("sphere");
        let hull_color = skin_def
            .and_then(|s| s.primary_color)
            .map(|[r, g, b]| Color::rgb(r, g, b))
            .unwrap_or(Color::rgb(0.30, 0.50, 0.80));
        let accent_color = skin_def
            .and_then(|s| s.secondary_color)
            .map(|[r, g, b]| Color::rgb(r, g, b))
            .unwrap_or(Color::rgb(0.80, 0.30, 0.20));
        let glow_color = skin_def
            .and_then(|s| s.emissive_color)
            .map(|[r, g, b]| Color::rgb(r, g, b))
            .unwrap_or(Color::rgb(0.40, 1.00, 2.00));

        let custom_hull = materials.add(StandardMaterial {
            base_color: hull_color,
            metallic: 0.70,
            perceptual_roughness: 0.30,
            ..default()
        });
        let custom_accent = materials.add(StandardMaterial {
            base_color: accent_color,
            metallic: 0.60,
            perceptual_roughness: 0.40,
            ..default()
        });
        let custom_glow = materials.add(StandardMaterial {
            base_color: glow_color,
            emissive: glow_color,
            metallic: 0.00,
            perceptual_roughness: 0.80,
            ..default()
        });

        let custom_root = commands
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

        build_custom_ship(shape_name, custom_root, &mut commands, &mut meshes, custom_hull, custom_accent, custom_glow);
        commands.entity(camera_entity).push_children(&[custom_root]);
        return;
    }

    // Currently only one skin available; keep structure for future variants
    let fuselage_mesh = match *ship_skin {
        crate::resources::ShipSkin::WarPlane => meshes.add(Mesh::from(shape::Box {
            min_x: -0.50, max_x: 0.50,
            min_y: -0.32, max_y: 0.32,
            min_z: -5.00, max_z: 3.00,
        })),
        crate::resources::ShipSkin::Banana => meshes.add(Mesh::from(shape::Capsule {
            radius: 0.35,
            rings: 4,
            depth: 4.0,
            latitudes: 8,
            longitudes: 8,
            uv_profile: shape::CapsuleUvProfile::Uniform,
        })),
        crate::resources::ShipSkin::Mosquito => meshes.add(Mesh::from(shape::Box {
            min_x: -0.28, max_x: 0.28,
            min_y: -0.28, max_y: 0.28,
            min_z: -3.80, max_z: 2.50,
        })),
        crate::resources::ShipSkin::Custom(_) => unreachable!(),
    };

    // ── Hierarchy ─────────────────────────────────────────────────────────────
    // Root sits in camera-local space.  Receiving SceneEntity means it is
    // cleaned up automatically by despawn_scene_entities on OnExit(Playing).
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

    let fuselage = commands.spawn(PbrBundle { mesh: fuselage_mesh, material: hull.clone(), ..default() }).id();

    match *ship_skin {
        crate::resources::ShipSkin::WarPlane => {
            // Main delta wings (slightly swept back)
            let wings_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -5.50, max_x: 5.50,
                min_y: -0.10, max_y: 0.10,
                min_z: -0.50, max_z: 2.00,
            }));
            // Vertical tail fin
            let vtail_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -0.10, max_x: 0.10,
                min_y:  0.30, max_y: 2.00,
                min_z:  1.60, max_z: 3.00,
            }));
            // Horizontal stabiliser
            let htail_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -2.50, max_x: 2.50,
                min_y: -0.10, max_y: 0.10,
                min_z:  2.00, max_z: 3.00,
            }));
            // Nose cone
            let nose_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -0.42, max_x: 0.42,
                min_y: -0.28, max_y: 0.28,
                min_z: -5.10, max_z: -3.80,
            }));
            // Cockpit canopy
            let canopy_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -0.34, max_x: 0.34,
                min_y:  0.28, max_y: 0.75,
                min_z: -3.60, max_z: -1.20,
            }));
            // Dual engine nozzles
            let nozzle_mesh = meshes.add(Mesh::from(shape::UVSphere { radius: 0.38, sectors: 8, stacks: 5 }));

            let wings  = commands.spawn(PbrBundle { mesh: wings_mesh,  material: hull.clone(), ..default() }).id();
            let vtail  = commands.spawn(PbrBundle { mesh: vtail_mesh,  material: hull.clone(), ..default() }).id();
            let htail  = commands.spawn(PbrBundle { mesh: htail_mesh,  material: hull.clone(), ..default() }).id();
            let nose   = commands.spawn(PbrBundle { mesh: nose_mesh,   material: accent.clone(), ..default() }).id();
            let canopy = commands.spawn(PbrBundle { mesh: canopy_mesh, material: cockpit, ..default() }).id();
            let nozzle_l = commands.spawn(PbrBundle {
                mesh: nozzle_mesh.clone(),
                material: engine_glow.clone(),
                transform: Transform::from_xyz(-0.30, -0.10, 3.25),
                ..default()
            }).id();
            let nozzle_r = commands.spawn(PbrBundle {
                mesh: nozzle_mesh,
                material: engine_glow,
                transform: Transform::from_xyz(0.30, -0.10, 3.25),
                ..default()
            }).id();
            commands.entity(root).push_children(&[fuselage, wings, vtail, htail, nose, canopy, nozzle_l, nozzle_r]);
        }

        crate::resources::ShipSkin::Banana => {
            // Two tiny spheres as "the tips" of the banana
            let tip_mesh = meshes.add(Mesh::from(shape::UVSphere { radius: 0.22, sectors: 8, stacks: 6 }));
            let tip_front = commands.spawn(PbrBundle {
                mesh: tip_mesh.clone(),
                material: accent.clone(),
                transform: Transform::from_xyz(0.0, 0.25, -2.3),
                ..default()
            }).id();
            let tip_back = commands.spawn(PbrBundle {
                mesh: tip_mesh,
                material: accent,
                transform: Transform::from_xyz(0.0, -0.25, 2.3),
                ..default()
            }).id();
            // Engine glow at tail
            let nozzle_mesh = meshes.add(Mesh::from(shape::UVSphere { radius: 0.28, sectors: 8, stacks: 5 }));
            let nozzle = commands.spawn(PbrBundle {
                mesh: nozzle_mesh,
                material: engine_glow,
                transform: Transform::from_xyz(0.0, -0.10, 2.8),
                ..default()
            }).id();
            commands.entity(root).push_children(&[fuselage, tip_front, tip_back, nozzle]);
        }

        crate::resources::ShipSkin::Mosquito => {
            // Thin needle-like nose spike
            let spike_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -0.06, max_x: 0.06,
                min_y: -0.06, max_y: 0.06,
                min_z: -6.50, max_z: -3.80,
            }));
            // Wide transparent wings (dragonfly-style)
            let wing_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -7.0,  max_x: 7.0,
                min_y: -0.05, max_y: 0.05,
                min_z: -1.50, max_z: 0.50,
            }));
            // Abdomen (rear body extension)
            let abdomen_mesh = meshes.add(Mesh::from(shape::Box {
                min_x: -0.16, max_x: 0.16,
                min_y: -0.16, max_y: 0.16,
                min_z:  2.50, max_z: 6.00,
            }));
            let spike   = commands.spawn(PbrBundle { mesh: spike_mesh,   material: accent.clone(), ..default() }).id();
            let wings   = commands.spawn(PbrBundle { mesh: wing_mesh,    material: cockpit, ..default() }).id();
            let abdomen = commands.spawn(PbrBundle { mesh: abdomen_mesh, material: hull.clone(), ..default() }).id();
            let nozzle_mesh = meshes.add(Mesh::from(shape::UVSphere { radius: 0.20, sectors: 8, stacks: 5 }));
            let nozzle = commands.spawn(PbrBundle {
                mesh: nozzle_mesh,
                material: engine_glow,
                transform: Transform::from_xyz(0.0, 0.0, 2.5),
                ..default()
            }).id();
            commands.entity(root).push_children(&[fuselage, spike, wings, abdomen, nozzle]);
        }

        crate::resources::ShipSkin::Custom(_) => unreachable!(),
    }

    // Attach ship tree to the camera so it moves/rotates with it.
    commands.entity(camera_entity).push_children(&[root]);
}

// ── Custom ship builder ────────────────────────────────────────────────────────

/// Build a procedural ship shape into `root` based on the skin's `shape` field.
fn build_custom_ship(
    shape: &str,
    root: Entity,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    hull_mat: Handle<StandardMaterial>,
    accent_mat: Handle<StandardMaterial>,
    glow_mat: Handle<StandardMaterial>,
) {
    match shape {
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
