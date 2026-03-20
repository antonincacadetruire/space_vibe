use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct AsteroidSpawnTimer(pub Timer);

#[derive(Resource, Default)]
pub struct MouseLook {
	pub yaw: f32,
	pub pitch: f32,
    pub sensitivity: f32,
}

#[derive(Resource, Default)]
pub struct TimePaused(pub bool);

#[derive(Resource, Default)]
pub struct MenuState {
	pub open: bool,
	pub prev_paused: bool,
	pub settings_open: bool,
}

#[derive(Resource)]
pub struct Keybindings {
	pub throttle_up: KeyCode,
	pub throttle_down: KeyCode,
	pub vertical_up: KeyCode,
	pub vertical_down: KeyCode,
	pub toggle_pause: KeyCode,
	pub toggle_menu: KeyCode,
}

impl Default for Keybindings {
	fn default() -> Self {
		Self {
			throttle_up: KeyCode::W,
			throttle_down: KeyCode::S,
			vertical_up: KeyCode::E,
			vertical_down: KeyCode::Q,
			toggle_pause: KeyCode::Space,
			toggle_menu: KeyCode::Escape,
		}
	}
}

#[derive(Resource, Default)]
pub struct RebindState(pub Option<Action>);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
	ThrottleUp,
	ThrottleDown,
	VerticalUp,
	VerticalDown,
	TogglePause,
	ToggleMenu,
}

#[derive(Resource, Default)]
pub struct VelocityUpdates(pub HashMap<Entity, Vec3>);

#[derive(Resource)]
pub struct Throttle(pub f32);
