use bevy::prelude::*;
use rand::Rng;

use crate::components::*;
use crate::systems::spawner::spawn_asteroid;

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.2, 0.8, 1.0),
                custom_size: Some(Vec2::new(30.0, 20.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, -50.0, 0.0)),
            ..default()
        },
        Shuttle,
    ));

    let mut rng = rand::thread_rng();
    for _ in 0..6 {
        spawn_asteroid(&mut commands, &mut rng);
    }
}
