use bevy::prelude::*;

#[derive(Component)]
pub struct Asteroid;

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct Radius(pub f32);

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct SpeedUi;
#[derive(Component)]
pub struct CompassDial;

#[derive(Component)]
pub struct CompassNeedle;

#[derive(Component)]
pub struct CompassPitchText;

#[derive(Component)]
pub struct AngularVelocity(pub Vec3);

#[derive(Component)]
pub struct Saturn;

#[derive(Component)]
pub struct SkyDome;

#[derive(Component)]
pub struct CursorCross;

/// Marker for ring-belt asteroids — excluded from free-flight collision and movement systems.
#[derive(Component)]
pub struct BeltAsteroid;

// Menu/UI markers
#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MainMenuPanel;

#[derive(Component)]
pub struct SettingsPanel;

#[derive(Component)]
pub struct ResumeButton;

#[derive(Component)]
pub struct SettingsButton;

#[derive(Component)]
pub struct CommandsButton;

#[derive(Component)]
pub struct QuitButton;

#[derive(Component)]
pub struct SensIncreaseButton;

#[derive(Component)]
pub struct SensDecreaseButton;

#[derive(Component)]
pub struct SettingsBackButton;

#[derive(Component)]
pub struct SensitivityText;

#[derive(Component)]
pub struct RebindButton;

#[derive(Component)]
pub struct RebindText;

// ── Missile components ────────────────────────────────────────────────────────
#[derive(Component)]
pub struct Missile {
    pub speed: f32,
    pub turn_rate: f32, // radians / second
    pub lifetime: f32,  // seconds remaining
}

#[derive(Component)]
pub struct MissileTrail;

// ── Start / death screen markers ─────────────────────────────────────────────
#[derive(Component)]
pub struct StartMenuRoot;

#[derive(Component)]
pub struct DeathScreenRoot;

#[derive(Component)]
pub struct PlayButton;

#[derive(Component)]
pub struct PlayAgainButton;

#[derive(Component)]
pub struct HomeButton;

#[derive(Component)]
pub struct TimerUi;

// ── Danger / threat HUD ───────────────────────────────────────────────────────
#[derive(Component)]
pub struct MissileWarningUi;

#[derive(Component)]
pub struct DangerVignette;
// ── Alien ship component ──────────────────────────────────────────────────────
#[derive(Component)]
pub struct AlienShip {
    pub speed: f32,
    pub shoot_timer: f32,
    pub shoot_interval: f32,
    pub health: i32,
}

// ── Combat: laser bolt fired by the player ───────────────────────────────────
#[derive(Component)]
pub struct Laser {
    pub speed: f32,
    pub lifetime: f32,
}

// ── Expanding portal ring when an alien spawns ────────────────────────────────
#[derive(Component)]
pub struct SpawnPortal {
    /// Negative = pre-delay before animation starts.
    pub timer: f32,
    pub max_time: f32,
}

// ── Expanding explosion sphere effect ────────────────────────────────────────
#[derive(Component)]
pub struct Explosion {
    pub timer: f32,
    pub max_time: f32,
    pub max_scale: f32,
}

// ── Health indicator pip — child entity of AlienShip ─────────────────────────
#[derive(Component)]
pub struct AlienHealthPip {
    pub index: usize,
    pub mat_active: Handle<StandardMaterial>,
    pub mat_inactive: Handle<StandardMaterial>,
}