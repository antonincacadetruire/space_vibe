use bevy::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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
    IdfTransport,
}

impl SceneKind {
    pub fn label(&self) -> &'static str {
        match self {
            SceneKind::SpaceAsteroids => "Asteroid Field",
            SceneKind::IceCaves      => "Ice Caves",
            SceneKind::DesertPlanet  => "Desert Planet",
            SceneKind::IdfTransport  => "\u{00ce}le-de-France",
        }
    }
    pub fn file_key(&self) -> &'static str {
        match self {
            SceneKind::SpaceAsteroids => "space",
            SceneKind::IceCaves      => "ice",
            SceneKind::DesertPlanet  => "desert",
            SceneKind::IdfTransport  => "idf",
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

#[allow(dead_code)]
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
    pub commands_open: bool,
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

/// JSON-friendly keybinding representation.  KeyCode names are stored as
/// debug-format strings (e.g. `"W"`, `"Space"`, `"Escape"`).
#[derive(Serialize, Deserialize)]
struct KeybindingsJson {
    throttle_up: String,
    throttle_down: String,
    vertical_up: String,
    vertical_down: String,
    toggle_pause: String,
    toggle_menu: String,
}

impl Keybindings {
    /// All actions, in display order.
    pub const ACTIONS: &'static [Action] = &[
        Action::ThrottleUp,
        Action::ThrottleDown,
        Action::VerticalUp,
        Action::VerticalDown,
        Action::TogglePause,
        Action::ToggleMenu,
    ];

    pub fn get(&self, action: Action) -> KeyCode {
        match action {
            Action::ThrottleUp   => self.throttle_up,
            Action::ThrottleDown => self.throttle_down,
            Action::VerticalUp   => self.vertical_up,
            Action::VerticalDown => self.vertical_down,
            Action::TogglePause  => self.toggle_pause,
            Action::ToggleMenu   => self.toggle_menu,
        }
    }

    pub fn set(&mut self, action: Action, code: KeyCode) {
        match action {
            Action::ThrottleUp   => self.throttle_up = code,
            Action::ThrottleDown => self.throttle_down = code,
            Action::VerticalUp   => self.vertical_up = code,
            Action::VerticalDown => self.vertical_down = code,
            Action::TogglePause  => self.toggle_pause = code,
            Action::ToggleMenu   => self.toggle_menu = code,
        }
    }

    fn keybindings_path() -> std::path::PathBuf {
        let cwd = std::path::PathBuf::from("data/keybindings.json");
        if cwd.parent().map(|p| p.exists()).unwrap_or(false) {
            cwd
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|d| d.join("data").join("keybindings.json")))
                .unwrap_or_else(|| std::path::PathBuf::from("data/keybindings.json"))
        }
    }

    /// Load keybindings from `data/keybindings.json`, falling back to defaults.
    pub fn load() -> Self {
        let path = Self::keybindings_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Keybindings::default();
        };
        let Ok(json) = serde_json::from_str::<KeybindingsJson>(&text) else {
            warn!("Could not parse {:?}. Using default keybindings.", path);
            return Keybindings::default();
        };
        info!("Loaded keybindings from {:?}", path);
        Keybindings {
            throttle_up:   keycode_from_str(&json.throttle_up).unwrap_or(KeyCode::W),
            throttle_down: keycode_from_str(&json.throttle_down).unwrap_or(KeyCode::S),
            vertical_up:   keycode_from_str(&json.vertical_up).unwrap_or(KeyCode::E),
            vertical_down: keycode_from_str(&json.vertical_down).unwrap_or(KeyCode::Q),
            toggle_pause:  keycode_from_str(&json.toggle_pause).unwrap_or(KeyCode::Space),
            toggle_menu:   keycode_from_str(&json.toggle_menu).unwrap_or(KeyCode::Escape),
        }
    }

    /// Persist current keybindings to `data/keybindings.json`.
    pub fn save(&self) {
        let json = KeybindingsJson {
            throttle_up:   keycode_to_str(self.throttle_up),
            throttle_down: keycode_to_str(self.throttle_down),
            vertical_up:   keycode_to_str(self.vertical_up),
            vertical_down: keycode_to_str(self.vertical_down),
            toggle_pause:  keycode_to_str(self.toggle_pause),
            toggle_menu:   keycode_to_str(self.toggle_menu),
        };
        let path = Self::keybindings_path();
        if let Ok(text) = serde_json::to_string_pretty(&json) {
            if let Err(e) = std::fs::write(&path, text) {
                warn!("Could not save keybindings to {:?}: {e}", path);
            }
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

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::ThrottleUp   => "Throttle Up",
            Action::ThrottleDown => "Throttle Down",
            Action::VerticalUp   => "Vertical Up",
            Action::VerticalDown => "Vertical Down",
            Action::TogglePause  => "Pause",
            Action::ToggleMenu   => "Menu",
        }
    }

    pub fn next(self) -> Option<Action> {
        match self {
            Action::ThrottleUp   => Some(Action::ThrottleDown),
            Action::ThrottleDown => Some(Action::VerticalUp),
            Action::VerticalUp   => Some(Action::VerticalDown),
            Action::VerticalDown => Some(Action::TogglePause),
            Action::TogglePause  => Some(Action::ToggleMenu),
            Action::ToggleMenu   => None,
        }
    }
}

/// Convert a `KeyCode` to a human-readable string for JSON persistence.
pub fn keycode_to_str(code: KeyCode) -> String {
    format!("{:?}", code)
}

/// Parse a `KeyCode` from its debug-format string (e.g. `"W"`, `"Space"`).
pub fn keycode_from_str(s: &str) -> Option<KeyCode> {
    // Build a quick lookup using known key names.
    // Bevy 0.11 KeyCode is an enum with ~160 variants; we match common ones.
    match s {
        "Key1" => Some(KeyCode::Key1), "Key2" => Some(KeyCode::Key2),
        "Key3" => Some(KeyCode::Key3), "Key4" => Some(KeyCode::Key4),
        "Key5" => Some(KeyCode::Key5), "Key6" => Some(KeyCode::Key6),
        "Key7" => Some(KeyCode::Key7), "Key8" => Some(KeyCode::Key8),
        "Key9" => Some(KeyCode::Key9), "Key0" => Some(KeyCode::Key0),
        "A" => Some(KeyCode::A), "B" => Some(KeyCode::B), "C" => Some(KeyCode::C),
        "D" => Some(KeyCode::D), "E" => Some(KeyCode::E), "F" => Some(KeyCode::F),
        "G" => Some(KeyCode::G), "H" => Some(KeyCode::H), "I" => Some(KeyCode::I),
        "J" => Some(KeyCode::J), "K" => Some(KeyCode::K), "L" => Some(KeyCode::L),
        "M" => Some(KeyCode::M), "N" => Some(KeyCode::N), "O" => Some(KeyCode::O),
        "P" => Some(KeyCode::P), "Q" => Some(KeyCode::Q), "R" => Some(KeyCode::R),
        "S" => Some(KeyCode::S), "T" => Some(KeyCode::T), "U" => Some(KeyCode::U),
        "V" => Some(KeyCode::V), "W" => Some(KeyCode::W), "X" => Some(KeyCode::X),
        "Y" => Some(KeyCode::Y), "Z" => Some(KeyCode::Z),
        "Escape" => Some(KeyCode::Escape), "Space" => Some(KeyCode::Space),
        "Return" => Some(KeyCode::Return), "Back" => Some(KeyCode::Back),
        "Tab" => Some(KeyCode::Tab),
        "Left" => Some(KeyCode::Left), "Right" => Some(KeyCode::Right),
        "Up" => Some(KeyCode::Up), "Down" => Some(KeyCode::Down),
        "LShift" | "ShiftLeft" => Some(KeyCode::ShiftLeft), "RShift" | "ShiftRight" => Some(KeyCode::ShiftRight),
        "LControl" | "ControlLeft" => Some(KeyCode::ControlLeft), "RControl" | "ControlRight" => Some(KeyCode::ControlRight),
        "LAlt" | "AltLeft" => Some(KeyCode::AltLeft), "RAlt" | "AltRight" => Some(KeyCode::AltRight),
        "F1" => Some(KeyCode::F1), "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3), "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5), "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7), "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9), "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11), "F12" => Some(KeyCode::F12),
        _ => None,
    }
}

#[derive(Resource, Default)]
pub struct VelocityUpdates(pub HashMap<Entity, Vec3>);

#[derive(Resource)]
pub struct Throttle(pub f32);

/// Tracks the movement mode for the new Z/S (manual accel) + E/A (preset speed) system.
#[derive(Resource)]
pub struct SpeedMode {
    /// Current preset step: 0 = off, 1/2/3 = 5000/10000/15000 forward; -1/-2/-3 = reverse.
    pub preset_step: i32,
    /// True while Z or S is actively held → overrides preset.
    pub manual_active: bool,
    /// Target speed from manual input (positive for forward, negative for reverse).
    pub manual_target: f32,
}

impl Default for SpeedMode {
    fn default() -> Self {
        Self { preset_step: 0, manual_active: false, manual_target: 0.0 }
    }
}

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
    /// World-space point to orbit around when in ThirdPerson + C held.
    /// Set to Some(ship_world_pos) when C is first pressed.
    pub orbit_center: Option<Vec3>,
    /// Orbit yaw angle (radians) used while ThirdPerson orbiting.
    pub orbit_yaw: f32,
    /// Orbit pitch angle (radians) used while ThirdPerson orbiting.
    pub orbit_pitch: f32,
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
    Terrain,
}

// ── Selectable ship appearance (future skins system) ─────────────────────────
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub enum ShipSkin {
    #[default]
    WarPlane,
    Banana,
    Mosquito,
    /// Any AI-generated or custom skin identified by its JSON `id` field.
    Custom(String),
}

// ── Maximum distance the player may travel from the scene origin ──────────────
#[derive(Resource)]
pub struct ZoneBoundary(pub f32);

impl Default for ZoneBoundary {
    fn default() -> Self {
        ZoneBoundary(100_000.0)
    }
}

// ── Camera view mode (first-person or third-person) ───────────────────────────
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
}

impl Default for CameraMode {
    fn default() -> Self { CameraMode::ThirdPerson }
}

/// World-space offset currently applied to the camera for the third-person
/// spring-arm effect. Must be undone before movement and reapplied after.
#[derive(Resource, Default)]
pub struct CameraArmOffset(pub Vec3);

#[allow(dead_code)]
impl ShipSkin {
    pub fn label(&self) -> String {
        match self {
            ShipSkin::WarPlane      => "War Plane".to_owned(),
            ShipSkin::Banana        => "Banana".to_owned(),
            ShipSkin::Mosquito      => "Mosquito".to_owned(),
            ShipSkin::Custom(id)    => id.clone(),
        }
    }

    pub fn all() -> &'static [ShipSkin] {
        &[ShipSkin::WarPlane, ShipSkin::Banana, ShipSkin::Mosquito]
    }

    pub fn id(&self) -> String {
        match self {
            ShipSkin::WarPlane      => "war_plane".to_owned(),
            ShipSkin::Banana        => "banana".to_owned(),
            ShipSkin::Mosquito      => "mosquito".to_owned(),
            ShipSkin::Custom(id)    => id.clone(),
        }
    }
}

// ── Île-de-France transport map configuration ─────────────────────────────────

/// Static definition of a transit line known to SpaceVibe.
#[derive(Debug, Clone)]
pub struct IdfLineDef {
    pub id: &'static str,
    pub label: &'static str,
    pub color: [f32; 3],
}

/// Static definition of a station the player can select.
#[derive(Debug, Clone)]
pub struct IdfStationDef {
    pub id: &'static str,
    /// Display name.
    pub label: &'static str,
    /// IDF Mobilités logical stop code (used for PRIM API).
    pub prim_id: &'static str,
    /// Lines served by this station.
    pub lines: &'static [&'static str],
    /// Approximate world-space position in the IDF map (units = ~100m).
    pub pos: [f32; 3],
}

/// Which stations the player has selected to be tracked + attacked.
#[derive(Resource, Default, Clone)]
pub struct IdfConfig {
    /// Indices into `IDF_STATIONS` that are selected.
    pub selected_stations: Vec<usize>,
}

/// Cached next-departure strings fetched from PRIM API, keyed by station prim_id.
#[derive(Resource, Default, Clone)]
pub struct IdfNextTrains {
    /// keyed: prim_id → vec of display strings like "M14 → Olympiades : 2 min"
    pub departures: std::collections::HashMap<String, Vec<String>>,
    /// Arc<Mutex<>> slot written by background fetch thread.
    pub pending: Option<std::sync::Arc<std::sync::Mutex<Option<std::collections::HashMap<String, Vec<String>>>>>>,
}

// ── Desert terrain kill data ──────────────────────────────────────────────────
/// Populated by `spawn_desert_planet_scene'; queried by `desert_terrain_death_system`.
/// Only valid while the desert map is active.
#[derive(Resource, Default)]
pub struct DesertTerrainData {
    /// Y coordinate of the sand surface (player dies when below this).
    pub floor_y: f32,
    /// List of (centre, horizontal_radius, vertical_radius) ellipsoid kill zones.
    pub kill_zones: Vec<(Vec3, f32, f32)>,
}
