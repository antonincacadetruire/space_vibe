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
