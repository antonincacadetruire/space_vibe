use bevy::prelude::*;
use rand::Rng;

use crate::components::SceneEntity;
use crate::resources::{ActiveScene, MouseLook, PrevCameraPosition, SceneKind, SpawnTransform};
use super::space_scene::spawn_space_scene;
use super::ice_caves::spawn_ice_caves_scene;
use super::desert_planet::spawn_desert_planet_scene;

/// Spawns the scene selected by the `ActiveScene` resource.
/// Runs on OnEnter(Playing), before enter_playing, so SpawnTransform is ready.
pub fn spawn_active_scene_system(
    mut commands: Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images:    ResMut<Assets<Image>>,
    active_scene:  Res<ActiveScene>,
    mut spawn_transform: ResMut<SpawnTransform>,
    mut mouse_look: ResMut<MouseLook>,
    mut prev_cam:   ResMut<PrevCameraPosition>,
) {
    let mut rng = rand::thread_rng();

    let transform = match &active_scene.0 {
        SceneKind::SpaceAsteroids => spawn_space_scene(
            &mut commands, &mut meshes, &mut materials, &mut images, &mut rng,
        ),
        SceneKind::IceCaves => spawn_ice_caves_scene(
            &mut commands, &mut meshes, &mut materials, &mut rng,
        ),
        SceneKind::DesertPlanet => spawn_desert_planet_scene(
            &mut commands, &mut meshes, &mut materials, &mut rng,
        ),
    };

    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    *spawn_transform = SpawnTransform { transform, yaw, pitch };
    mouse_look.yaw   = yaw;
    mouse_look.pitch = pitch;
    prev_cam.0       = transform.translation;
}

/// Despawns all entities tagged SceneEntity (called on OnExit(Playing)).
pub fn despawn_scene_entities(
    mut commands: Commands,
    entities: Query<Entity, With<SceneEntity>>,
) {
    for e in entities.iter() {
        commands.entity(e).despawn_recursive();
    }
}
