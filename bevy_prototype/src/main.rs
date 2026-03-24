mod components;
mod resources;
mod setup;
mod systems;

use bevy::prelude::*;
use resources::*;
use systems::core::movement::record_camera_position_system;
use systems::core::mouse::mouse_look_system;
use systems::core::exit::toggle_menu_system;
use systems::core::fullscreen::toggle_fullscreen_system;
use systems::ui::hud::{ui_update_system, cursor_follow_system};
use systems::core::collision::asteroid_collision_system;
use systems::core::movement::{asteroid_movement_system, player_movement_system};
use systems::scenes::space_scene::{follow_sky_dome_system, update_ring_lod_system, update_ring_orbit_system};
use systems::scenes::scene_manager::{spawn_active_scene_system, despawn_scene_entities};
use systems::ui::menu::{button_appearance_system, menu_ui_system, menu_button_system, sensitivity_button_system, sensitivity_text_system, key_capture_system};
use systems::ui::start_menu::{
    setup_start_menu, teardown_start_menu,
    start_menu_button_system, start_menu_button_appearance_system,
    enter_playing, spawn_timer_ui, despawn_timer_ui, update_timer,
    danger_hud_system,
};
use systems::ui::death_screen::{
    setup_death_screen, teardown_death_screen,
    death_screen_button_system, death_screen_button_appearance_system,
};
use systems::ui::minimap::{spawn_minimap_ui, despawn_minimap_ui, update_minimap_system};
use systems::enemies::missiles::{missile_spawner_system, missile_movement_system, despawn_missiles};
use systems::enemies::alien_ships::{alien_ship_spawner_system, alien_ship_movement_system, alien_ship_shoot_system, despawn_alien_ships};
use systems::enemies::combat::{shoot_laser_system, laser_movement_system, portal_animation_system, explosion_animation_system, health_pip_update_system, despawn_effects};

const SHUTTLE_SPEED: f32 = 20_000.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // ── State ────────────────────────────────────────────────────────────
        .add_state::<GameState>()
        // ── Resources ────────────────────────────────────────────────────────
        .insert_resource(MouseLook { yaw: 0.0, pitch: 0.0, sensitivity: 1.0 })
        .insert_resource(TimePaused(false))
        .insert_resource(MenuState::default())
        .insert_resource(Keybindings::default())
        .insert_resource(RebindState::default())
        .insert_resource(Throttle(0.0))
        .insert_resource(PrevCameraPosition::default())
        .insert_resource(VelocityUpdates::default())
        .insert_resource(RingLodUpdateTimer(Timer::from_seconds(0.2, TimerMode::Repeating)))
        .insert_resource(GameTimer::default())
        .insert_resource(SpawnTransform::default())
        .insert_resource(SceneLeaderboard::load())
        .insert_resource(ActiveScene::default())
        .insert_resource(KillCount::default())
        .insert_resource(FreeLook::default())
        .insert_resource(MissileSpawnTimer(Timer::from_seconds(18.0, TimerMode::Repeating)))
        .insert_resource(AlienSpawnTimer(Timer::from_seconds(30.0, TimerMode::Repeating)))
        .insert_resource(DeathCause::default())
        // ── Startup ──────────────────────────────────────────────────────────
        .add_systems(Startup, setup::setup)
        // ── State enter/exit hooks ───────────────────────────────────────────
        .add_systems(OnEnter(GameState::StartMenu), setup_start_menu)
        .add_systems(OnExit(GameState::StartMenu), teardown_start_menu)
        .add_systems(OnEnter(GameState::Playing), (
            spawn_active_scene_system,
            enter_playing.after(spawn_active_scene_system),
            spawn_timer_ui,
            spawn_minimap_ui,
        ))
        .add_systems(OnExit(GameState::Playing), (despawn_timer_ui, despawn_minimap_ui, despawn_missiles, despawn_alien_ships, despawn_effects, despawn_scene_entities))
        .add_systems(OnEnter(GameState::Dead), setup_death_screen)
        .add_systems(OnExit(GameState::Dead), (teardown_death_screen, despawn_missiles, despawn_alien_ships, despawn_effects, despawn_scene_entities))
        // ── Update: always ────────────────────────────────────────────────────
        .add_systems(Update, toggle_fullscreen_system)
        // ── Update: StartMenu state ───────────────────────────────────────────
        .add_systems(
            Update,
            (
                start_menu_button_appearance_system,
                start_menu_button_system.after(start_menu_button_appearance_system),
            )
                .run_if(in_state(GameState::StartMenu)),
        )
        // ── Update: Playing state (batch A – input / movement / HUD) ────────
        .add_systems(
            Update,
            (
                mouse_look_system,
                toggle_menu_system,
                player_movement_system.after(mouse_look_system),
                record_camera_position_system.after(player_movement_system),
                ui_update_system.after(player_movement_system),
                cursor_follow_system.after(ui_update_system),
                update_timer.after(player_movement_system),
                menu_ui_system.after(toggle_menu_system),
                button_appearance_system.after(menu_ui_system),
                menu_button_system.after(button_appearance_system),
                sensitivity_button_system.after(menu_button_system),
                sensitivity_text_system.after(sensitivity_button_system),
                key_capture_system.after(menu_button_system),
                shoot_laser_system,
            )
                .run_if(in_state(GameState::Playing)),
        )
        // ── Update: Playing state (batch B – world / missiles / scene) ───────
        .add_systems(
            Update,
            (
                asteroid_collision_system.after(player_movement_system),
                asteroid_movement_system.after(asteroid_collision_system),
                missile_spawner_system.after(player_movement_system),
                missile_movement_system.after(missile_spawner_system),
                danger_hud_system.after(missile_movement_system),
                alien_ship_spawner_system,
                alien_ship_movement_system.after(alien_ship_spawner_system),
                alien_ship_shoot_system.after(alien_ship_movement_system),
                laser_movement_system,
                portal_animation_system,
                explosion_animation_system,
                health_pip_update_system,
                update_minimap_system,
            )
                .run_if(in_state(GameState::Playing)),
        )
        // ── Update: Playing state (space-scene-only systems) ─────────────────
        .add_systems(
            Update,
            (
                follow_sky_dome_system,
                update_ring_orbit_system,
                update_ring_lod_system.after(update_ring_orbit_system),
            )
                .run_if(in_state(GameState::Playing))
                .run_if(resource_exists::<systems::scenes::space_scene::RingMeshLibrary>()),
        )
        // ── Update: Dead state ────────────────────────────────────────────────
        .add_systems(
            Update,
            (
                death_screen_button_appearance_system,
                death_screen_button_system.after(death_screen_button_appearance_system),
            )
                .run_if(in_state(GameState::Dead)),
        )
        .run();
}

