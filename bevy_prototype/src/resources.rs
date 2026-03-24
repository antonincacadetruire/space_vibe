use bevy::prelude::*;
use std::collections::HashMap;

// ── Game states ──────────────────────────────────────────────────────────────
#[derive(States, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    StartMenu,
    Playing,
    Dead,
}

// ── Gameplay timer (seconds elapsed since round start) ───────────────────────
#[derive(Resource, Default)]
pub struct GameTimer(pub f32);

// ── Top-3 leaderboard (best survival times, ascending = longer is better) ───
#[derive(Resource, Default)]
pub struct Leaderboard {
    /// Sorted descending: scores[0] is the best (longest) time.
    pub scores: Vec<f32>,
}

impl Leaderboard {
    pub const MAX: usize = 3;

    fn scores_path() -> std::path::PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("spacevibe_scores.dat")))
            .unwrap_or_else(|| std::path::PathBuf::from("spacevibe_scores.dat"))
    }

    /// Load scores from disk (returns empty leaderboard if file doesn't exist).
    pub fn load() -> Self {
        let path = Self::scores_path();
        let mut lb = Leaderboard::default();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            for line in contents.lines() {
                if let Ok(v) = line.trim().parse::<f32>() {
                    lb.scores.push(v);
                }
            }
            lb.scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
            lb.scores.truncate(Self::MAX);
        }
        lb
    }

    /// Persist current scores to disk.
    pub fn save(&self) {
        let path = Self::scores_path();
        let contents = self.scores.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(&path, contents);
    }

    /// Insert a new score, keep only the top MAX entries (longest times win).
    pub fn submit(&mut self, time: f32) {
        self.scores.push(time);
        self.scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
        self.scores.truncate(Self::MAX);
    }

    pub fn is_new_best(&self, time: f32) -> bool {
        self.scores.len() < Self::MAX || time > *self.scores.last().unwrap_or(&0.0)
    }
}

// ── Initial camera spawn transform (used for respawn) ────────────────────────
#[derive(Resource, Default)]
pub struct SpawnTransform {
    pub transform: Transform,
    pub yaw: f32,
    pub pitch: f32,
}

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

#[derive(Resource, Default)]
pub struct PrevCameraPosition(pub Vec3);

#[derive(Resource)]
pub struct RingLodUpdateTimer(pub Timer);

// ── Free-look mode (hold C: look around without changing travel direction) ───
#[derive(Resource, Default)]
pub struct FreeLook {
    /// Saved yaw/pitch of travel direction when free-look started.
    pub travel_yaw: f32,
    pub travel_pitch: f32,
    pub active: bool,
}

// ── Missile spawn timer ───────────────────────────────────────────────────────
#[derive(Resource)]
pub struct MissileSpawnTimer(pub Timer);

// ── Alien ship spawn timer ────────────────────────────────────────────────────
#[derive(Resource)]
pub struct AlienSpawnTimer(pub Timer);

// ── What killed the player this round ────────────────────────────────────────
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub enum DeathCause {
    #[default]
    Asteroid,
    Missile,
}
