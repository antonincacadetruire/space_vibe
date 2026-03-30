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
pub struct CommandsPanel;

#[derive(Component)]
pub struct CommandsBackButton;

/// Text node showing the current rebind prompt / all keybindings.
#[derive(Component)]
pub struct CommandsStatusText;

#[derive(Component)]
pub struct SensitivityText;

// These UI rebind components are reserved for future use.
#[allow(dead_code)]
#[derive(Component)] pub struct RebindButton;
#[allow(dead_code)]
#[derive(Component)] pub struct RebindText;

// ── Missile components ────────────────────────────────────────────────────────
#[derive(Component)]
pub struct Missile {
    pub speed: f32,
    pub turn_rate: f32, // radians / second
    pub lifetime: f32,  // seconds remaining
}

// MissileTrail reserved for future particle trail effect.
#[allow(dead_code)]
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

// ── Marks entities belonging to the active scene (cleaned up on scene exit) ───
#[derive(Component)]
pub struct SceneEntity;

// ── Player ship 3-D model (child of the main camera) ─────────────────────────
#[derive(Component)]
pub struct PlayerShipModel;
// UI markers for start menu carousels
#[derive(Component, Default)] pub struct SkinLeftButton;
#[derive(Component, Default)] pub struct SkinRightButton;
#[derive(Component, Default)] pub struct SkinLabel;
#[derive(Component, Default)] pub struct MapLeftButton;
#[derive(Component, Default)] pub struct MapRightButton;
#[derive(Component, Default)] pub struct MapLabel;
/// Preview image node for the currently-selected skin.
#[derive(Component)] pub struct SkinPreviewImage;
/// Preview image node for the currently-selected map.
#[derive(Component)] pub struct MapPreviewImage;
/// Small description text under the skin label.
#[derive(Component)] pub struct SkinDescLabel;
/// Small description text under the map label.
#[derive(Component)] pub struct MapDescLabel;
/// Best-scores text block for the current map.
#[derive(Component)] pub struct MapScoresLabel;

// ── In-game LLM / Copilot chat UI markers ────────────────────────────────────
/// Root node of the chat overlay panel.
#[derive(Component)] pub struct CopilotChatRoot;
/// Full-screen transparent click blocker behind the chat panel.
#[derive(Component)] pub struct CopilotChatBlocker;
/// Scrolling conversation log text node.
#[derive(Component)] pub struct CopilotChatLog;
/// The text input box node.
#[derive(Component)] pub struct CopilotInputBox;
/// Marker for the actual Text inside the input box (child of CopilotInputBox node).
#[derive(Component)] pub struct CopilotInputText;
/// Send button.
#[derive(Component)] pub struct CopilotSendButton;
/// Save-last-JSON button (only visible when a JSON block was returned).
#[derive(Component)] pub struct CopilotSaveButton;
/// Button to re-enter the API key prompt at any time.
#[derive(Component)] pub struct CopilotChangeKeyButton;
/// Status / spinner text node.
#[derive(Component)] pub struct CopilotStatusText;
/// Copilot chat button shown in the start menu.
#[derive(Component)] pub struct CopilotMenuButton;
/// Thumb of the conversation scrollbar (positioned absolutely inside the track).
#[derive(Component)] pub struct CopilotScrollThumb;
/// Button to copy the last AI response to clipboard.
#[derive(Component)] pub struct CopilotCopyButton;

// ── IDF station selection dropdown UI ────────────────────────────────────────
/// Root node for the station picker panel (visible only when IDF map selected).
#[derive(Component)] pub struct IdfStationPickerRoot;
/// Collapse / expand toggle button at the top of the picker.
#[derive(Component)] pub struct IdfPickerHeaderBtn;
/// The "▼ STATIONS" / "▶ STATIONS" text on the header button.
#[derive(Component)] pub struct IdfPickerHeaderText;
/// Inner scrollable container that holds all station toggle buttons.
#[derive(Component)] pub struct IdfPickerScrollContent;
/// A toggle button for a single station in the picker list.
#[derive(Component)] pub struct IdfStationToggleBtn { pub station_idx: usize }
/// Text label inside each station toggle button.
#[derive(Component)] pub struct IdfStationToggleText { pub station_idx: usize }