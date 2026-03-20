use bevy::prelude::*;

#[derive(Component)]
pub struct Shuttle;

#[derive(Component)]
pub struct Asteroid;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Radius(pub f32);

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct SpeedUi;
#[derive(Component)]
pub struct CompassDial;

#[derive(Component)]
pub struct CompassNeedle;

#[derive(Component)]
pub struct CursorCross;
