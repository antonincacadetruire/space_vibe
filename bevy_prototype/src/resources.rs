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

// ── Which scene / level the player is currently in ───────────────────────────
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub enum SceneKind {
    #[default]
    SpaceAsteroids,
    IceCaves,
    DesertPlanet,
}

impl SceneKind {
    pub fn label(&self) -> &'static str {
        match self {
            SceneKind::SpaceAsteroids => "Asteroid Field",
            SceneKind::IceCaves      => "Ice Caves",
            SceneKind::DesertPlanet  => "Desert Planet",
        }
    }
    pub fn file_key(&self) -> &'static str {
        match self {
            SceneKind::SpaceAsteroids => "space",
            SceneKind::IceCaves      => "ice",
            SceneKind::DesertPlanet  => "desert",
        }
    }
}

// ── Active scene resource ─────────────────────────────────────────────────────
#[derive(Resource, Default)]
pub struct ActiveScene(pub SceneKind);

// ── Kill counter (enemies destroyed this run) ────────────────────────────────
#[derive(Resource, Default)]
pub struct KillCount(pub u32);

// ── Gameplay timer (seconds elapsed since round start) ───────────────────────
#[derive(Resource, Default)]
pub struct GameTimer(pub f32);

// ── Per-scene top-3 leaderboard (survival time in seconds, descending) ───────
#[derive(Resource, Default)]
pub struct SceneLeaderboard {
    data: HashMap<String, Vec<f32>>,
}

impl SceneLeaderboard {
    pub const MAX: usize = 3;

    fn scores_path(key: &str) -> std::path::PathBuf {
        let filename = format!("spacevibe_scores_{}.dat", key);
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join(&filename)))
            .unwrap_or_else(|| std::path::PathBuf::from(&filename))
    }

    pub fn load() -> Self {
        let mut lb = SceneLeaderboard::default();
        for key in &["space", "ice", "desert"] {
            let path = Self::scores_path(key);
            let mut scores: Vec<f32> = Vec::new();
            if let Ok(contents) = std::fs::read_to_string(&path) {
                for line in contents.lines() {
                    if let Ok(v) = line.trim().parse::<f32>() {
                        scores.push(v);
                    }
                }
            }
            scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
            scores.truncate(Self::MAX);
            lb.data.insert(key.to_string(), scores);
        }
        lb
    }

    pub fn save(&self, scene: &SceneKind) {
        let key = scene.file_key();
        let path = Self::scores_path(key);
        let scores = self.data.get(key).cloned().unwrap_or_default();
        let contents = scores.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("\n");
        let _ = std::fs::write(&path, contents);
    }

    pub fn scores(&self, scene: &SceneKind) -> &[f32] {
        self.data.get(scene.file_key()).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn submit(&mut self, scene: &SceneKind, time: f32) {
        let key = scene.file_key().to_string();
        let scores = self.data.entry(key).or_default();
        scores.push(time);
        scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
        scores.truncate(Self::MAX);
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
