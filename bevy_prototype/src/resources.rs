use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct AsteroidSpawnTimer(pub Timer);

#[derive(Resource, Default)]
pub struct MouseLook {
	pub yaw: f32,
	pub pitch: f32,
}

#[derive(Resource, Default)]
pub struct TimePaused(pub bool);

#[derive(Resource, Default)]
pub struct VelocityUpdates(pub HashMap<Entity, Vec3>);

#[derive(Resource)]
pub struct Throttle(pub f32);
