mod components;
mod resources;
mod setup;
mod systems;

use bevy::prelude::*;
use components::*;
use resources::*;
use systems::shuttle::shuttle_steer_and_move_system;
use systems::collision::asteroid_collision_system;
use systems::movement::asteroid_movement_system;
use systems::spawner::asteroid_spawner_system;

const SHUTTLE_SPEED: f32 = 200.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup::setup)
        .insert_resource(AsteroidSpawnTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
        .insert_resource(ShuttlePosition::default())
        .insert_resource(VelocityUpdates::default())
        .add_systems(
            Update,
            (
                shuttle_steer_and_move_system,
                asteroid_collision_system.after(shuttle_steer_and_move_system),
                asteroid_movement_system.after(asteroid_collision_system),
                asteroid_spawner_system.after(asteroid_movement_system),
            ),
        )
        .run();
}
