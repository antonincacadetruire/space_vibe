use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct AsteroidSpawnTimer(pub Timer);

#[derive(Resource, Default)]
pub struct ShuttlePosition(pub Vec2);

#[derive(Resource, Default)]
pub struct VelocityUpdates(pub HashMap<Entity, Vec2>);
