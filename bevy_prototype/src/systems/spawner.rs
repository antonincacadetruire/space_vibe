use bevy::prelude::*;
use rand::Rng;

use crate::components::*;
use crate::resources::AsteroidSpawnTimer;

pub fn asteroid_spawner_system(
    time: Res<Time>,
    mut timer: ResMut<AsteroidSpawnTimer>,
    mut commands: Commands,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        let mut rng = rand::thread_rng();
        spawn_asteroid(&mut commands, &mut rng);
    }
}

pub fn spawn_asteroid(commands: &mut Commands, rng: &mut impl Rng) {
    let x = rng.gen_range(-400.0..400.0);
    let y = rng.gen_range(300.0..700.0);
    let radius = rng.gen_range(8.0..40.0);
    let vx = rng.gen_range(-30.0..30.0);
    let vy = rng.gen_range(-80.0..-30.0);

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.5, 0.45, 0.4),
                custom_size: Some(Vec2::new(radius * 2.0, radius * 2.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(x, y, 0.0)),
            ..default()
        },
        Asteroid,
        Velocity(Vec2::new(vx, vy)),
        Radius(radius),
    ));
}
