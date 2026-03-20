mod components;
mod resources;
mod setup;
mod systems;

use bevy::prelude::*;
use resources::*;
use systems::movement::record_camera_position_system;
use systems::mouse::mouse_look_system;
use systems::exit::toggle_menu_system;
use systems::fullscreen::toggle_fullscreen_system;
use systems::ui::ui_update_system;
use systems::ui::cursor_follow_system;
use systems::collision::asteroid_collision_system;
use systems::movement::{asteroid_movement_system, player_movement_system};
use systems::space_scene::{follow_sky_dome_system, update_ring_lod_system, update_ring_orbit_system};
use systems::menu::{button_appearance_system, menu_ui_system, menu_button_system, sensitivity_button_system, sensitivity_text_system, key_capture_system};

const SHUTTLE_SPEED: f32 = 20_000.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup::setup)
        .insert_resource(MouseLook { yaw: 0.0, pitch: 0.0, sensitivity: 1.0 })
        .insert_resource(TimePaused(false))
        .insert_resource(MenuState::default())
        .insert_resource(Keybindings::default())
        .insert_resource(RebindState::default())
        .insert_resource(Throttle(0.0))
        .insert_resource(PrevCameraPosition::default())
        .insert_resource(VelocityUpdates::default())
        .insert_resource(RingLodUpdateTimer(Timer::from_seconds(0.2, TimerMode::Repeating)))
        .add_systems(
            Update,
            (
                mouse_look_system,
                toggle_menu_system,
                toggle_fullscreen_system,
                player_movement_system.after(mouse_look_system),
                record_camera_position_system.after(player_movement_system),
                ui_update_system.after(player_movement_system),
                cursor_follow_system.after(ui_update_system),
                menu_ui_system.after(toggle_menu_system),
                button_appearance_system.after(menu_ui_system),
                menu_button_system.after(button_appearance_system),
                sensitivity_button_system.after(menu_button_system),
                sensitivity_text_system.after(sensitivity_button_system),
                key_capture_system.after(menu_button_system),
                asteroid_collision_system.after(player_movement_system),
                asteroid_movement_system.after(asteroid_collision_system),
                follow_sky_dome_system.after(asteroid_movement_system),
                update_ring_orbit_system.after(asteroid_movement_system),
                update_ring_lod_system.after(update_ring_orbit_system),
            ),
        )
        .run();
}
