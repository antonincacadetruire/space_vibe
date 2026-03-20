mod components;
mod resources;
mod setup;
mod systems;

use bevy::prelude::*;
use resources::*;
use systems::mouse::mouse_look_system;
use systems::exit::exit_on_escape_system;
use systems::fullscreen::toggle_fullscreen_system;
use systems::ui::ui_update_system;
use systems::ui::cursor_follow_system;
use systems::collision::asteroid_collision_system;
use systems::movement::{asteroid_movement_system, player_movement_system};
use systems::spawner::asteroid_spawner_system;

const SHUTTLE_SPEED: f32 = 200.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup::setup)
        .insert_resource(AsteroidSpawnTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
        .insert_resource(MouseLook::default())
        .insert_resource(TimePaused(false))
        .insert_resource(Throttle(0.0))
        .insert_resource(VelocityUpdates::default())
        .add_systems(
            Update,
            (
                mouse_look_system,
                exit_on_escape_system,
                toggle_fullscreen_system,
                player_movement_system.after(mouse_look_system),
                ui_update_system.after(player_movement_system),
                cursor_follow_system.after(ui_update_system),
                asteroid_collision_system.after(player_movement_system),
                asteroid_movement_system.after(asteroid_collision_system),
                asteroid_spawner_system.after(asteroid_movement_system),
            ),
        )
        .run();
}
