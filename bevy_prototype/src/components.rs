use bevy::prelude::*;

#[derive(Component)]
pub struct Shuttle;

#[derive(Component)]
pub struct Asteroid;

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Radius(pub f32);
