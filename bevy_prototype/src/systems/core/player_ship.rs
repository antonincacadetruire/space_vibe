use bevy::prelude::*;

use crate::components::{MainCamera, PlayerShipModel, SceneEntity};

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
    cam_mode: Res<crate::resources::CameraMode>,
) {
    let Ok(camera_entity) = camera_q.get_single() else { return };
    // Ship starts visible in ThirdPerson mode, hidden in FirstPerson.
    let initial_vis = if *cam_mode == crate::resources::CameraMode::ThirdPerson {
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
    }

    // Attach ship tree to the camera so it moves/rotates with it.
    commands.entity(camera_entity).push_children(&[root]);
}
