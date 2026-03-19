mod components;
mod resources;
mod setup;
mod systems;

use bevy::prelude::*;
use components::*;
use resources::*;
use systems::shuttle::{shuttle_control_system, camera_follow_system};
use systems::ui::ui_update_system;
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
        .insert_resource(Throttle(1.0))
        .insert_resource(VelocityUpdates::default())
        .add_systems(
            Update,
            (
                shuttle_control_system,
                camera_follow_system.after(shuttle_control_system),
                ui_update_system.after(camera_follow_system),
                asteroid_collision_system.after(ui_update_system),
                asteroid_movement_system.after(asteroid_collision_system),
                asteroid_spawner_system.after(asteroid_movement_system),
            ),
        )
        .run();
}
