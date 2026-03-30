//! In-game AI / Copilot chat panel.
//!
//! Press **F2** while flying to toggle the chat overlay.
//! Type a request (e.g. "generate an icy enemy ship") and press **Enter** or
//! click **Send**.  The model responds with a JSON block that can be saved
//! directly into `data/maps/`, `data/skins/` or `data/enemies/` by pressing
//! the **Save** button.
//!
//! The HTTP request runs inside a detached `std::thread` so it never blocks
//! the game loop.  The result is written into a shared `Arc<Mutex<...>>` and
//! picked up each frame.

use bevy::prelude::*;
use bevy::ecs::system::ParamSet;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::window::{PrimaryWindow, CursorGrabMode, CursorIcon};

use std::sync::{Arc, Mutex};

use crate::components::{
    CopilotChatRoot, CopilotChatBlocker, CopilotChatLog, CopilotInputBox, CopilotInputText,
    CopilotSendButton, CopilotSaveButton, CopilotChangeKeyButton, CopilotStatusText,
    CopilotScrollThumb, CopilotCopyButton,
};
use crate::resources::{GameState, ShipSkin, ZoneBoundary, MaxSpeed, TeleportRequest};
use crate::resources::TimePaused;
use crate::setup::resolve_ui_font_path;
use crate::systems::data_loader::{LlmConfigResource, MapCatalog, MapCatalogImages, SkinCatalog, SkinCatalogImages, svg_to_image, CarouselState};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Save `api_key` to `data/secrets.json`.  Returns Err string on failure.
fn save_api_key(key: &str) -> Result<(), String> {
    let data_dir = {
        let cwd = std::path::PathBuf::from("data");
        if cwd.exists() {
            cwd
        } else if let Some(p) = std::env::current_exe().ok()
            .and_then(|e| e.parent().map(|d| d.join("data")))
        {
            p
        } else {
            std::path::PathBuf::from("data")
        }
    };
    let path = data_dir.join("secrets.json");
    let json = serde_json::json!({ "api_key": key });
    std::fs::write(&path, json.to_string())
        .map_err(|e| format!("write {:?}: {e}", path))
}

// ── Shared result slot ────────────────────────────────────────────────────────

/// Shared slot between the HTTP worker thread and the main game thread.
/// The worker writes `Some(Ok(text))` or `Some(Err(msg))` when done.
type ResultSlot = Arc<Mutex<Option<Result<String, String>>>>;

// ── Chat state resource ───────────────────────────────────────────────────────

/// Status of an in-flight LLM request.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum LlmStatus {
    #[default]
    Idle,
    Waiting,
    Error(String),
}

/// A single message in the conversation log.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// True → sent by the player, False → sent by the AI.
    pub is_user: bool,
    pub text: String,
}

/// Central state for the copilot chat overlay.
#[derive(Resource, Default)]
pub struct LlmChatState {
    /// Whether the chat panel is currently visible.
    pub open: bool,
    /// Text the player is currently typing.
    pub input_buffer: String,
    /// All messages shown in the log.
    pub conversation: Vec<ChatMessage>,
    /// Current request status.
    pub status: LlmStatus,
    /// Shared result slot written by the background thread.
    pub pending_result: Option<ResultSlot>,
    /// If the last AI reply contained a JSON block, this holds the raw JSON.
    pub last_json: Option<String>,
    /// When true, the next submitted text is treated as an API key, not a chat message.
    pub awaiting_api_key: bool,
    /// Whether the game was already paused before the chat was opened (so we
    /// don't accidentally un-pause when the user closes the chat).
    pub was_paused_before_chat: bool,
    /// Scroll offset: how many messages from the END to skip (0 = show latest).
    pub scroll_offset: usize,
    /// A game command extracted from the last AI reply (e.g. `set_speed 30000`).
    /// Player must type `/confirm` or `/cancel` to act on it.
    pub pending_command: Option<String>,
    /// Becomes true when the player types `/confirm` — consumed by save_system.
    pub command_confirmed: bool,
}

impl LlmChatState {
    fn add_user(&mut self, text: &str) {
        self.conversation.push(ChatMessage { is_user: true, text: text.to_owned() });
    }
    fn add_ai(&mut self, text: &str) {
        self.conversation.push(ChatMessage { is_user: false, text: text.to_owned() });
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Soft-wrap a single text line into segments of at most `max_chars` *Unicode
/// scalar values*, breaking on word boundaries where possible.
fn soft_wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.chars().count() <= max_chars {
        return vec![line.to_string()];
    }
    let mut result = Vec::new();
    let mut remaining = line;
    while remaining.chars().count() > max_chars {
        // Collect the first max_chars chars and find the last space byte-offset within them.
        let char_boundary: usize = remaining.char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());
        let slice = &remaining[..char_boundary];
        let cut = slice.rfind(' ').unwrap_or(char_boundary);
        // Avoid infinite loop if there is no space at all.
        let cut = if cut == 0 { char_boundary } else { cut };
        result.push(remaining[..cut].to_string());
        remaining = remaining[cut..].trim_start_matches(' ');
    }
    if !remaining.is_empty() {
        result.push(remaining.to_string());
    }
    result
}

/// Flatten all messages into soft-wrapped lines: (line_text, is_user).
/// `max_chars` should match the approximate visual width of the text area.
fn flatten_to_lines(conversation: &[ChatMessage], max_chars: usize) -> Vec<(String, bool)> {
    let mut all_lines: Vec<(String, bool)> = Vec::new();
    for msg in conversation {
        let is_user = msg.is_user;
        let prefix = if is_user { "You: " } else { "AI:  " };
        let indent = "     ";
        let full = format!("{prefix}{}", msg.text);
        let mut first = true;
        for raw_line in full.lines() {
            let wrapped = soft_wrap_line(raw_line, max_chars);
            for (wi, w) in wrapped.iter().enumerate() {
                if first && wi == 0 {
                    all_lines.push((w.clone(), is_user));
                    first = false;
                } else {
                    all_lines.push((format!("{indent}{w}"), is_user));
                }
            }
        }
        all_lines.push((String::new(), is_user)); // blank spacer between messages
    }
    all_lines
}

/// Extract the first ```json … ``` block from a string.
fn extract_json_block(text: &str) -> Option<String> {
    // Accept both ```json and ``` as opening fence
    let start = text.find("```json")
        .map(|i| i + 7)
        .or_else(|| text.find("```").map(|i| i + 3))?;
    let rest = &text[start..];
    let end = rest.find("```")?;
    let raw = rest[..end].trim().to_owned();
    if raw.starts_with('{') { Some(raw) } else { None }
}

/// Extract the first `[CMD: ...]` block from an AI response.
fn extract_command_block(text: &str) -> Option<String> {
    let start = text.find("[CMD:")?;
    let inner = start + 5; // skip "[CMD:"
    let rest = &text[inner..];
    let end = rest.find(']')?;
    Some(rest[..end].trim().to_owned())
}

/// Execute a game command string (e.g. `"set_speed 30000"`).
/// Returns a human-readable result message.
fn apply_command(
    cmd: &str,
    boundary: &mut ZoneBoundary,
    max_speed: &mut MaxSpeed,
    teleport: &mut TeleportRequest,
) -> String {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    match parts.as_slice() {
        ["set_speed", val] => match val.trim().parse::<f32>() {
            Ok(v) => {
                max_speed.0 = v.clamp(1_000.0, 500_000.0);
                format!("✓ Max speed set to {:.0} units/s", max_speed.0)
            }
            Err(_) => "✗ Invalid value. Example: [CMD: set_speed 30000]".to_owned(),
        },
        ["set_boundary", val] => match val.trim().parse::<f32>() {
            Ok(v) => {
                boundary.0 = v.clamp(10_000.0, 10_000_000.0);
                format!("✓ Zone boundary set to {:.0} units", boundary.0)
            }
            Err(_) => "✗ Invalid value. Example: [CMD: set_boundary 500000]".to_owned(),
        },
        ["teleport_origin"] | ["teleport"] => {
            teleport.0 = Some(Vec3::ZERO);
            "✓ Teleporting to origin…".to_owned()
        }
        _ => format!(
            "✗ Unknown command: '{}'\nAvailable: set_speed <v>, set_boundary <r>, teleport_origin",
            cmd
        ),
    }
}

/// Infer whether the JSON is a MAP, SKIN, or ENEMY definition from its keys.
fn infer_kind(json: &str) -> &'static str {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(v) => {
            if v.get("hull_color").is_some() { "enemies" }
            else if v.get("boundary_radius").is_some() { "maps" }
            else { "skins" }
        }
        Err(_) => "skins",
    }
}

/// Save `json` to `data/<kind>/<id>.json`.  Returns the path written.
fn save_json(json: &str, kind: &str) -> Result<String, String> {
    let id = serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("id").and_then(|s| s.as_str()).map(|s| s.to_owned()))
        .unwrap_or_else(|| "generated".to_owned());

    // Sanitise the id so it's safe as a filename
    let safe_id: String = id.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();

    // Locate the data/ directory (same logic as data_loader)
    let data = {
        let cwd = std::path::PathBuf::from("data");
        if cwd.exists() {
            cwd
        } else if let Some(p) = std::env::current_exe().ok()
            .and_then(|e| e.parent().map(|d| d.join("data")))
        {
            p
        } else {
            std::path::PathBuf::from("data")
        }
    };

    let dir = data.join(kind);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("mkdir {:?}: {e}", dir))?;

    let path = dir.join(format!("{safe_id}.json"));
    std::fs::write(&path, json)
        .map_err(|e| format!("write {:?}: {e}", path))?;

    Ok(path.to_string_lossy().to_string())
}

/// Generate a Rust scene template for a map JSON and write it to
/// `src/systems/scenes/<id>.rs`.  Returns `Ok(path)` on success.
/// The generated file is a compilable Bevy scene stub seeded with the map's
/// boundary radius, sky/accent colours and label.
fn generate_scene_rs(json: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse: {e}"))?;

    let id    = v.get("id").and_then(|s| s.as_str()).unwrap_or("custom_map");
    let label = v.get("label").and_then(|s| s.as_str()).unwrap_or("Custom Scene");
    let boundary = v.get("boundary_radius").and_then(|n| n.as_f64()).unwrap_or(300_000.0);

    let (r, g, b) = if let Some(arr) = v.get("accent_color").and_then(|a| a.as_array()) {
        let r = arr.get(0).and_then(|n| n.as_f64()).unwrap_or(0.3) as f32;
        let g = arr.get(1).and_then(|n| n.as_f64()).unwrap_or(0.3) as f32;
        let b = arr.get(2).and_then(|n| n.as_f64()).unwrap_or(0.3) as f32;
        (r, g, b)
    } else {
        (0.3_f32, 0.3_f32, 0.3_f32)
    };

    // Sanitise id into a valid Rust identifier
    let fn_name: String = id.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();

    let (er, eg, eb) = (r * 0.25, g * 0.25, b * 0.25);
    let sky_radius = (boundary * 1.5) as u32;

    let code = format!(
r#"//! Auto-generated scene: {label}
//! To integrate this scene add:
//!   1. `pub mod {fn_name};` in src/systems/scenes/mod.rs
//!   2. A `{fn_name_pascal}` variant to `SceneKind` in src/resources.rs
//!   3. `use super::{fn_name}::spawn_{fn_name}_scene;` in scene_manager.rs
//!   4. A match arm in `spawn_active_scene_system` in scene_manager.rs
use bevy::prelude::*;
use rand::Rng;

use crate::components::{{SceneEntity, SkyDome}};

pub const SCENE_BOUNDARY: f32 = {boundary:.0};

pub fn spawn_{fn_name}_scene(
    commands: &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    _rng:      &mut impl Rng,
) -> Transform {{
    // ── Lighting ──────────────────────────────────────────────────────────────
    commands.insert_resource(AmbientLight {{
        color:      Color::rgb({r:.2}, {g:.2}, {b:.2}),
        brightness: 0.55,
    }});

    commands.spawn((
        DirectionalLightBundle {{
            directional_light: DirectionalLight {{
                illuminance: 85_000.0,
                color: Color::WHITE,
                shadows_enabled: false,
                ..default()
            }},
            transform: Transform::from_rotation(
                Quat::from_euler(EulerRot::XYZ, -0.55, 0.35, 0.0),
            ),
            ..default()
        }},
        SceneEntity,
    ));

    // ── Sky dome ──────────────────────────────────────────────────────────────
    commands.spawn((
        PbrBundle {{
            mesh: meshes.add(Mesh::from(shape::UVSphere {{
                radius: {sky_radius}.0,
                sectors: 36,
                stacks: 20,
            }})),
            material: materials.add(StandardMaterial {{
                base_color: Color::rgb({r:.2}, {g:.2}, {b:.2}),
                emissive:   Color::rgb({er:.2}, {eg:.2}, {eb:.2}),
                unlit: true,
                cull_mode: None,
                ..default()
            }}),
            ..default()
        }},
        SkyDome,
        SceneEntity,
    ));

    // ── Player start transform ─────────────────────────────────────────────────
    Transform::from_translation(Vec3::ZERO)
        .looking_at(Vec3::NEG_Z, Vec3::Y)
}}
"#,
        label       = label,
        fn_name     = fn_name,
        fn_name_pascal = {
            let mut s = fn_name.clone();
            if let Some(c) = s.get_mut(0..1) { c.make_ascii_uppercase(); }
            s
        },
        boundary    = boundary,
        r = r, g = g, b = b,
        er = er, eg = eg, eb = eb,
        sky_radius  = sky_radius,
    );

    // Try src/systems/scenes/ relative to cwd (works with `cargo run`)
    let scenes_dir = {
        let cwd = std::path::PathBuf::from("src").join("systems").join("scenes");
        if cwd.exists() {
            cwd
        } else {
            // Fall back: walk up from the executable looking for `src/`
            std::env::current_exe()
                .ok()
                .and_then(|e| {
                    // target/debug/ → project root
                    e.parent()?.parent()?.parent().map(|root| {
                        root.join("src").join("systems").join("scenes")
                    })
                })
                .unwrap_or_else(|| std::path::PathBuf::from("src").join("systems").join("scenes"))
        }
    };

    std::fs::create_dir_all(&scenes_dir)
        .map_err(|e| format!("mkdir {:?}: {e}", scenes_dir))?;

    let path = scenes_dir.join(format!("{fn_name}.rs"));
    std::fs::write(&path, code)
        .map_err(|e| format!("write {:?}: {e}", path))?;

    Ok(path.to_string_lossy().to_string())
}

// ── HTTP worker ───────────────────────────────────────────────────────────────
/// Spawns a background thread that calls the LLM API and writes the result
/// into the shared `slot`.
fn spawn_llm_request(
    api_url: String,
    api_key: String,
    model: String,
    system_prompt: String,
    max_tokens: u32,
    user_message: String,
    slot: ResultSlot,
) {
    std::thread::spawn(move || {
        let body = serde_json::json!({
            "model": model,
            "messages": [
                { "role": "system",  "content": system_prompt },
                { "role": "user",    "content": user_message  },
            ],
            "temperature": 0.8,
            "max_tokens": max_tokens,
        });

        let mut req = ureq::post(&api_url)
            .set("Content-Type", "application/json");

        if !api_key.is_empty() {
            req = req.set("Authorization", &format!("Bearer {api_key}"));
        }

        let result = req.send_string(&body.to_string());

        let value = match result {
            Err(e) => Err(format!("HTTP error: {e}")),
            Ok(resp) => {
                match resp.into_string() {
                    Err(e) => Err(format!("Body read error: {e}")),
                    Ok(raw) => {
                        // Parse OpenAI-compatible response
                        let content = serde_json::from_str::<serde_json::Value>(&raw)
                            .ok()
                            .and_then(|v| {
                                v["choices"][0]["message"]["content"]
                                    .as_str()
                                    .map(|s| s.to_owned())
                            })
                            .unwrap_or(raw);
                        Ok(content)
                    }
                }
            }
        };

        if let Ok(mut guard) = slot.lock() {
            *guard = Some(value);
        }
    });
}

// ── Setup system ──────────────────────────────────────────────────────────────

/// Spawns the (initially hidden) chat overlay when entering Playing state.
pub fn setup_llm_chat_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(resolve_ui_font_path());

    // ── HUD colour palette ─────────────────────────────────────────────────
    let cyan_bright  = Color::rgb(0.00, 0.95, 1.00);
    let cyan_dim     = Color::rgba(0.00, 0.72, 0.85, 0.70);
    let bg_deep      = Color::rgba(0.01, 0.03, 0.09, 0.97);
    let bg_log       = Color::rgba(0.00, 0.01, 0.04, 0.88);
    let border_color = Color::rgba(0.00, 0.62, 0.82, 0.90);

    // ── Full-screen click blocker ──────────────────────────────────────────
    commands.spawn((
        CopilotChatBlocker,
        NodeBundle {
            style: Style {
                display: Display::None,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.0).into(),
            z_index: ZIndex::Global(199),
            ..default()
        },
    ));

    // ── Root outer frame — visible as 2-px HUD border ──────────────────────
    commands
        .spawn((
            CopilotChatRoot,
            NodeBundle {
                style: Style {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(8.0),
                    bottom: Val::Px(8.0),
                    width: Val::Px(504.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(2.0)), // border thickness
                    overflow: Overflow::clip(),
                    min_height: Val::Px(0.0),
                    ..default()
                },
                background_color: border_color.into(), // teal "border" peek-through
                z_index: ZIndex::Global(200),
                ..default()
            },
        ))
        .with_children(|outer| {
            // Inner dark panel
            outer.spawn(NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    min_height: Val::Px(0.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                background_color: bg_deep.into(),
                ..default()
            }).with_children(|root| {

                // ── Top accent bar ─────────────────────────────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(2.0),
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                    background_color: cyan_bright.into(),
                    ..default()
                });

                // ── Title bar ──────────────────────────────────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(22.0),
                        margin: UiRect::bottom(Val::Px(4.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    ..default()
                }).with_children(|bar| {
                    bar.spawn(TextBundle::from_section(
                        "◈ COPILOT AI ◈   [F2]",
                        TextStyle { font: font.clone(), font_size: 14.0, color: cyan_bright },
                    ));
                    bar.spawn((
                        CopilotStatusText,
                        TextBundle::from_section("", TextStyle {
                            font: font.clone(), font_size: 12.0,
                            color: Color::rgb(0.95, 0.82, 0.22),
                        }),
                    ));
                });

                // ── Title separator ────────────────────────────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::bottom(Val::Px(6.0)),
                        ..default()
                    },
                    background_color: cyan_dim.into(),
                    ..default()
                });

                // ── Conversation log — row: [text | scroll-track] ──────────
                root.spawn(NodeBundle {
                    style: Style {
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        max_height: Val::Percent(100.0),
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        overflow: Overflow::clip(),
                        margin: UiRect::bottom(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    background_color: bg_log.into(),
                    ..default()
                }).with_children(|log_row| {
                    // Text column
                    log_row.spawn(NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            min_height: Val::Px(0.0),
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        ..default()
                    }).with_children(|text_col| {
                        text_col.spawn((
                            CopilotChatLog,
                            TextBundle {
                                text: Text::from_sections(vec![
                                    TextSection::new(
                                        "[ SYS ] Ask me to generate a map, skin or enemy.\n[ SYS ] Use /setkey <PAT> to configure API access.\n",
                                        TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgba(0.28, 0.78, 0.85, 0.55) },
                                    ),
                                ]),
                                style: Style { flex_wrap: FlexWrap::Wrap, ..default() },
                                ..default()
                            },
                        ));
                    });

                    // Scrollbar track
                    log_row.spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(6.0),
                            height: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            margin: UiRect::left(Val::Px(4.0)),
                            ..default()
                        },
                        background_color: Color::rgba(0.00, 0.16, 0.22, 0.80).into(),
                        ..default()
                    }).with_children(|track| {
                        track.spawn((
                            CopilotScrollThumb,
                            NodeBundle {
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    top: Val::Percent(0.0),
                                    ..default()
                                },
                                background_color: Color::rgba(0.05, 0.90, 1.00, 0.95).into(),
                                ..default()
                            },
                        ));
                    });
                });

                // ── Input separator ────────────────────────────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                    background_color: cyan_dim.into(),
                    ..default()
                });

                // ── Input row ─────────────────────────────────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(34.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                    ..default()
                }).with_children(|row| {
                    // Input box — outer node is 1-px teal border
                    row.spawn(NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            height: Val::Px(30.0),
                            padding: UiRect::all(Val::Px(1.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: cyan_dim.into(), // border colour
                        ..default()
                    }).with_children(|border| {
                        border.spawn((
                            CopilotInputBox,
                            NodeBundle {
                                style: Style {
                                    flex_grow: 1.0,
                                    height: Val::Percent(100.0),
                                    padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: Color::rgba(0.02, 0.08, 0.13, 0.97).into(),
                                ..default()
                            },
                        )).with_children(|inp| {
                            inp.spawn((
                                CopilotInputText,
                                TextBundle::from_section(
                                    "",
                                    TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgb(0.85, 1.00, 1.00) },
                                ),
                            ));
                        });
                    });

                    // Send button
                    row.spawn((
                        CopilotSendButton,
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(62.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: Color::rgb(0.00, 0.22, 0.28).into(),
                            ..default()
                        },
                    )).with_children(|btn| {
                        btn.spawn(TextBundle::from_section(
                            "SEND",
                            TextStyle { font: font.clone(), font_size: 13.0, color: cyan_bright },
                        ));
                    });
                });

                // ── Action row (Save / ChangeKey / Copy) ──────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(28.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(5.0),
                        ..default()
                    },
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        CopilotSaveButton,
                        ButtonBundle {
                            style: Style {
                                display: Display::None,
                                width: Val::Px(160.0),
                                height: Val::Px(26.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: Color::rgb(0.02, 0.22, 0.10).into(),
                            ..default()
                        },
                    )).with_children(|btn| {
                        btn.spawn(TextBundle::from_section(
                            "◼ SAVE JSON",
                            TextStyle { font: font.clone(), font_size: 12.0, color: Color::rgb(0.40, 1.00, 0.55) },
                        ));
                    });

                    row.spawn((
                        CopilotChangeKeyButton,
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(95.0),
                                height: Val::Px(26.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: Color::rgb(0.16, 0.08, 0.02).into(),
                            ..default()
                        },
                    )).with_children(|btn| {
                        btn.spawn(TextBundle::from_section(
                            "CHG KEY",
                            TextStyle { font: font.clone(), font_size: 12.0, color: Color::rgb(1.0, 0.72, 0.22) },
                        ));
                    });

                    row.spawn((
                        CopilotCopyButton,
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(130.0),
                                height: Val::Px(26.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: Color::rgb(0.00, 0.16, 0.22).into(),
                            ..default()
                        },
                    )).with_children(|btn| {
                        btn.spawn(TextBundle::from_section(
                            "◈ COPY REPLY",
                            TextStyle { font: font.clone(), font_size: 12.0, color: cyan_bright },
                        ));
                    });
                });

                // ── Bottom accent bar ──────────────────────────────────────
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(5.0)),
                        ..default()
                    },
                    background_color: cyan_bright.into(),
                    ..default()
                });
            });
        });
}

// ── Teardown system ───────────────────────────────────────────────────────────

pub fn teardown_llm_chat_ui(
    mut commands: Commands,
    q: Query<Entity, With<CopilotChatRoot>>,
    blocker_q: Query<Entity, With<CopilotChatBlocker>>,
) {
    for e in &q { commands.entity(e).despawn_recursive(); }
    for e in &blocker_q { commands.entity(e).despawn_recursive(); }
}

// ── Toggle system (F2) ─────────────────────────────────────────────────────────

pub fn llm_chat_toggle_system(
    keys: Res<Input<KeyCode>>,
    mut chat: ResMut<LlmChatState>,
    mut q: Query<&mut Style, With<CopilotChatRoot>>,
    mut blocker_q: Query<&mut Style, (With<CopilotChatBlocker>, Without<CopilotChatRoot>)>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    game_state: Res<State<GameState>>,
    llm_cfg: Res<LlmConfigResource>,
    mut paused: ResMut<TimePaused>,
) {
    let state = game_state.get();
    if *state != GameState::Playing && *state != GameState::StartMenu {
        return;
    }

    if keys.just_pressed(KeyCode::F2) {
        chat.open = !chat.open;
        // cursor + pause management only in Playing state
        if *state == GameState::Playing {
            if let Ok(mut win) = windows.get_single_mut() {
                if chat.open {
                    win.cursor.visible = true;
                    win.cursor.grab_mode = CursorGrabMode::None;
                    win.cursor.icon = CursorIcon::Text;
                    // Save current pause state then force-pause the game
                    chat.was_paused_before_chat = paused.0;
                    paused.0 = true;
                } else {
                    win.cursor.visible = false;
                    win.cursor.grab_mode = CursorGrabMode::Locked;
                    win.cursor.icon = CursorIcon::Default;
                    // Restore previous pause state when closing chat
                    paused.0 = chat.was_paused_before_chat;
                }
            }
        }
    }

    // If chat just opened and no API key is set, enter key-prompt mode
    if chat.open && !chat.awaiting_api_key && llm_cfg.0.api_key.is_empty() && chat.conversation.is_empty() {
        chat.awaiting_api_key = true;
        chat.add_ai("No API key found. Please paste your GitHub Personal Access Token (PAT):");
    }

    // Always sync chat.open → Display so any setter (F2 or button) takes effect.
    let display = if chat.open { Display::Flex } else { Display::None };
    for mut style in &mut q {
        if style.display != display {
            style.display = display;
        }
    }
    for mut style in &mut blocker_q {
        if style.display != display {
            style.display = display;
        }
    }
}

// ── Input system (keyboard → buffer) ─────────────────────────────────────────

pub fn llm_chat_input_system(
    keys: Res<Input<KeyCode>>,
    mut key_evts: EventReader<KeyboardInput>,
    mut char_evts: EventReader<ReceivedCharacter>,
    mut chat: ResMut<LlmChatState>,
    mut llm_cfg: ResMut<LlmConfigResource>,
    send_btn_q: Query<&Interaction, With<CopilotSendButton>>,
    mut input_text_q: Query<&mut Text, (With<CopilotInputText>, Without<CopilotChatLog>, Without<CopilotStatusText>)>,
) {
    if !chat.open { return; }
    if chat.status == LlmStatus::Waiting { return; }

    // ── Typed characters
    for ev in char_evts.iter() {
        let c = ev.char;
        if !c.is_control() {
            chat.input_buffer.push(c);
        }
    }

    // ── Key events (Backspace / Enter / Ctrl+V paste)
    let mut should_send = false;
    let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    for ev in key_evts.iter() {
        if ev.state != ButtonState::Pressed { continue; }
        match ev.key_code {
            Some(KeyCode::Back) => { chat.input_buffer.pop(); }
            Some(KeyCode::Return) | Some(KeyCode::NumpadEnter) => {
                should_send = true;
            }
            Some(KeyCode::V) if ctrl_held => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        for c in text.chars() {
                            if !c.is_control() {
                                chat.input_buffer.push(c);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── Send button click
    for interaction in send_btn_q.iter() {
        if *interaction == Interaction::Pressed {
            should_send = true;
        }
    }

    // Update visible input text
    let buf_snapshot = chat.input_buffer.clone();
    for mut text in &mut input_text_q {
        if let Some(section) = text.sections.first_mut() {
            section.value = format!("{buf_snapshot}|");
        }
    }

    if should_send && !chat.input_buffer.is_empty() {
        let prompt = chat.input_buffer.trim().to_owned();
        chat.input_buffer.clear();
        chat.scroll_offset = 0; // jump to bottom on send

        // ── Command confirmation shortcuts ──────────────────────────────────
        if prompt == "/confirm" {
            chat.add_user("/confirm");
            if chat.pending_command.is_some() {
                chat.command_confirmed = true;
            } else {
                chat.add_ai("No pending command to confirm.");
            }
            return;
        }
        if prompt == "/cancel" {
            chat.add_user("/cancel");
            if chat.pending_command.take().is_some() {
                chat.add_ai("Command cancelled.");
            } else {
                chat.add_ai("No pending command to cancel.");
            }
            return;
        }

        // ── API key prompt mode ────────────────────────────────────────────
        if chat.awaiting_api_key {
            let key = prompt.trim().to_owned();
            match save_api_key(&key) {
                Ok(()) => {
                    llm_cfg.0.api_key = key;
                    chat.awaiting_api_key = false;
                    chat.add_ai("✓ API key saved! You can now ask me to generate maps, skins or enemies.");
                }
                Err(e) => {
                    chat.add_ai(&format!("✗ Failed to save API key: {e}\nPlease try again:"));
                }
            }
            return;
        }

        // ── /setkey command ────────────────────────────────────────────────
        if let Some(key) = prompt.strip_prefix("/setkey ") {
            let key = key.trim().to_owned();
            match save_api_key(&key) {
                Ok(()) => {
                    llm_cfg.0.api_key = key;
                    chat.add_ai("✓ API key saved to data/secrets.json and applied. You can now chat with the AI.");
                }
                Err(e) => {
                    chat.add_ai(&format!("✗ Failed to save API key: {e}"));
                }
            }
            return;
        }

        chat.add_user(&prompt);
        chat.status = LlmStatus::Waiting;
        chat.last_json = None;

        // Kick off async HTTP request
        let cfg = llm_cfg.0.clone();
        let slot: ResultSlot = Arc::new(Mutex::new(None));
        spawn_llm_request(
            cfg.api_url.clone(),
            cfg.api_key.clone(),
            cfg.model.clone(),
            cfg.system_prompt.clone(),
            cfg.max_tokens,
            prompt,
            Arc::clone(&slot),
        );
        chat.pending_result = Some(slot);
    }
}

// ── Poll response system ──────────────────────────────────────────────────────

pub fn llm_chat_poll_system(
    mut chat: ResMut<LlmChatState>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<CopilotChatLog>>,
        Query<&mut Text, With<CopilotStatusText>>,
    )>,
    mut save_q: Query<&mut Style, (With<CopilotSaveButton>, Without<CopilotScrollThumb>)>,
    mut scroll_thumb_q: Query<&mut Style, (With<CopilotScrollThumb>, Without<CopilotSaveButton>)>,
    asset_server: Res<AssetServer>,
) {
    // Update status spinner text
    {
        let mut status_q = text_queries.p1();
        if let Ok(mut t) = status_q.get_single_mut() {
            match &chat.status {
                LlmStatus::Idle    => t.sections[0].value.clear(),
                LlmStatus::Waiting => t.sections[0].value = "● thinking…".to_owned(),
                LlmStatus::Error(e) => t.sections[0].value = format!("ERR: {e}"),
            }
        }
    }

    // Poll the shared result slot (non-blocking)
    let response = {
        if let Some(slot) = &chat.pending_result {
            let taken = slot.lock().ok().and_then(|mut guard| guard.take());
            if taken.is_some() { chat.pending_result = None; }
            taken
        } else {
            None
        }
    };

    if let Some(result) = response {
        match result {
            Err(err) => {
                chat.status = LlmStatus::Error(err.clone());
                chat.add_ai(&format!("[Error: {err}]"));
            }
            Ok(content) => {
                chat.status = LlmStatus::Idle;
                chat.last_json = extract_json_block(&content);

                // Check for a game command block `[CMD: ...]`
                if let Some(cmd) = extract_command_block(&content) {
                    chat.pending_command = Some(cmd.clone());
                    chat.add_ai(&format!(
                        "{}\n\n⚡ Command: [{}]\nType /confirm to execute or /cancel to dismiss.",
                        content, cmd
                    ));
                } else {
                    let display_text = if chat.last_json.is_some() {
                        format!("{content}\n[JSON block detected – click Save to write to disk]")
                    } else {
                        content
                    };
                    chat.add_ai(&display_text);
                }
            }
        }
    }

    // Show/hide Save button
    let save_display = if chat.last_json.is_some() { Display::Flex } else { Display::None };
    for mut style in &mut save_q {
        style.display = save_display;
    }

    // Rebuild conversation log text (respecting scroll_offset)
    let font = asset_server.load(resolve_ui_font_path());
    {
        let mut log_q = text_queries.p0();
        if let Ok(mut log_text) = log_q.get_single_mut() {
            if chat.conversation.is_empty() {
                log_text.sections = vec![TextSection::new(
                    "[Ask me anything – e.g. \"generate a toxic gas planet map\"\n or just chat! Scroll: mouse wheel]\n",
                    TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgba(0.55, 0.75, 0.80, 0.55) },
                )];
            } else {
                // Use the same pre-wrapped lines as the scroll system so each entry
                // is already ≤55 chars — the Text widget won't re-wrap them, keeping
                // the node's natural height predictable and within the log container.
                const MAX_VIS_LINES: usize = 50;
                const MAX_CHARS: usize = 55;
                let all_lines = flatten_to_lines(&chat.conversation, MAX_CHARS);

                let total_lines = all_lines.len();
                let skip = chat.scroll_offset.min(total_lines.saturating_sub(1));
                let end = total_lines.saturating_sub(skip);
                let start = end.saturating_sub(MAX_VIS_LINES);

                let mut sections: Vec<TextSection> = all_lines[start..end].iter().map(|(line, is_user)| {
                    let color = if *is_user {
                        Color::rgb(0.35, 0.88, 0.55)
                    } else {
                        Color::rgb(0.18, 0.80, 0.95)
                    };
                    TextSection::new(
                        format!("{line}\n"),
                        TextStyle { font: font.clone(), font_size: 13.0, color },
                    )
                }).collect();

                // Scroll indicator at top when scrolled up
                if skip > 0 {
                    sections.insert(0, TextSection::new(
                        format!("↑ {} more lines above ↑\n", total_lines.saturating_sub(end)),
                        TextStyle { font: font.clone(), font_size: 11.0, color: Color::rgba(0.70, 0.70, 0.40, 0.70) },
                    ));
                }
                log_text.sections = sections;
            }
        }
    }

    // Update scrollbar thumb — same line count as the display.
    const MAX_VIS_LINES: usize = 25;
    const MAX_CHARS: usize = 55;
    let total_lines = flatten_to_lines(&chat.conversation, MAX_CHARS).len();
    if let Ok(mut thumb) = scroll_thumb_q.get_single_mut() {
        if total_lines <= MAX_VIS_LINES {
            thumb.height = Val::Percent(100.0);
            thumb.top    = Val::Percent(0.0);
        } else {
            let max_offset   = (total_lines - MAX_VIS_LINES) as f32;
            let thumb_pct    = MAX_VIS_LINES as f32 / total_lines as f32 * 100.0;
            let scroll_ratio = (chat.scroll_offset as f32).min(max_offset) / max_offset;
            // 0 = newest (bottom), 1 = oldest (top)
            let top_pct = (1.0 - scroll_ratio) * (100.0 - thumb_pct);
            thumb.height = Val::Percent(thumb_pct);
            thumb.top    = Val::Percent(top_pct);
        }
    }
}

// ── Chat scroll system ────────────────────────────────────────────────────────

pub fn llm_chat_scroll_system(
    mut chat: ResMut<LlmChatState>,
    mut scroll_evts: EventReader<bevy::input::mouse::MouseWheel>,
) {
    if !chat.open { return; }
    for ev in scroll_evts.iter() {
        // 3 lines per wheel notch for smooth, progressive scrolling
        let delta = match ev.unit {
            bevy::input::mouse::MouseScrollUnit::Line  => (ev.y * 3.0) as i32,
            bevy::input::mouse::MouseScrollUnit::Pixel => (ev.y / 15.0) as i32,
        };
        let total_lines: usize = flatten_to_lines(&chat.conversation, 55).len();
        const MAX_VIS_LINES: usize = 25;
        let max_offset = total_lines.saturating_sub(MAX_VIS_LINES);
        // Scroll up (positive delta) → increase offset (show older lines)
        let new_offset = (chat.scroll_offset as i32 - delta)
            .max(0)
            .min(max_offset as i32) as usize;
        chat.scroll_offset = new_offset;
    }
}

// ── Save button system ─────────────────────────────────────────────────────────
pub fn llm_chat_save_system(
    mut chat: ResMut<LlmChatState>,
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<CopilotSaveButton>)>,
    change_key_q: Query<&Interaction, (Changed<Interaction>, With<CopilotChangeKeyButton>)>,
    copy_btn_q: Query<&Interaction, (Changed<Interaction>, With<CopilotCopyButton>)>,
    mut status_q: Query<&mut Text, (With<CopilotStatusText>, Without<CopilotChatLog>, Without<CopilotInputText>)>,
    mut map_catalog: ResMut<MapCatalog>,
    mut map_images: ResMut<MapCatalogImages>,
    mut skin_catalog: ResMut<SkinCatalog>,
    mut skin_images: ResMut<SkinCatalogImages>,
    mut images: ResMut<Assets<Image>>,
    mut carousel_state: ResMut<CarouselState>,
    mut ship_skin: ResMut<ShipSkin>,
    mut boundary: ResMut<ZoneBoundary>,
    mut max_speed: ResMut<MaxSpeed>,
    mut teleport: ResMut<TeleportRequest>,
) {
    // ── Change Key button
    for interaction in &change_key_q {
        if *interaction != Interaction::Pressed { continue; }
        chat.awaiting_api_key = true;
        chat.add_ai("Please paste your new GitHub Personal Access Token (PAT):");
    }

    // ── Copy last AI response button
    for interaction in &copy_btn_q {
        if *interaction != Interaction::Pressed { continue; }
        let last_ai = chat.conversation.iter().rev()
            .find(|m| !m.is_user)
            .map(|m| m.text.clone());
        if let Some(text) = last_ai {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(text);
                if let Ok(mut t) = status_q.get_single_mut() {
                    t.sections[0].value = "Copied!".to_owned();
                }
            }
        }
    }
    for interaction in &interaction_q {
        if *interaction != Interaction::Pressed { continue; }
        if let Some(json) = &chat.last_json.clone() {
            let kind = infer_kind(json);
            match save_json(json, kind) {
                Ok(path) => {
                    // Live-reload the matching catalog so the carousel reflects the new entry immediately.
                    match kind {
                        "maps" => {
                            *map_catalog = MapCatalog::load();
                            map_images.handles = map_catalog.maps.iter()
                                .map(|m| images.add(svg_to_image(&m.preview_svg, 128, 128)))
                                .collect();
                            // Jump carousel to the newly added map (last entry after sort).
                            carousel_state.map_idx = map_catalog.maps.len().saturating_sub(1);

                            // Also generate a Rust scene template file.
                            let rs_note = match generate_scene_rs(json) {
                                Ok(rs_path) => format!(
                                    "\n📄 Scene stub → {rs_path}\n\
                                     ➡ Add `pub mod <id>;` to scenes/mod.rs\n\
                                     ➡ Add variant + match arm in resources.rs / scene_manager.rs"
                                ),
                                Err(e) => format!("\n⚠ Could not write .rs file: {e}"),
                            };
                            let msg = format!(
                                "Saved to {path}\n✨ Added to start-menu carousel!{rs_note}"
                            );
                            chat.add_ai(&msg);
                        }
                        "skins" => {
                            *skin_catalog = SkinCatalog::load();
                            skin_images.handles = skin_catalog.skins.iter()
                                .map(|s| images.add(svg_to_image(&s.preview_svg, 128, 128)))
                                .collect();
                            carousel_state.skin_idx = skin_catalog.skins.len().saturating_sub(1);
                            // Also update the active skin so the ship rebuilds on next play.
                            if let Some(new_skin) = skin_catalog.skins.last() {
                                *ship_skin = ShipSkin(new_skin.id.clone());
                            }
                            chat.add_ai(&format!("Saved to {path}\n✨ Added to start-menu carousel & set as active skin!"));
                        }
                        _ => {
                            chat.add_ai(&format!("Saved to {path}"));
                        }
                    }
                    chat.last_json = None;
                    if let Ok(mut t) = status_q.get_single_mut() {
                        t.sections[0].value = "Saved!".to_owned();
                    }
                }
                Err(e) => {
                    let msg = format!("[Save error: {e}]");
                    chat.add_ai(&msg);
                    if let Ok(mut t) = status_q.get_single_mut() {
                        t.sections[0].value = format!("ERR: {e}");
                    }
                }
            }
        }
    }

    // ── Command execution (triggered by /confirm) ─────────────────────────────
    if chat.command_confirmed {
        let cmd = chat.pending_command.take().unwrap_or_default();
        chat.command_confirmed = false;
        let result = apply_command(&cmd, &mut boundary, &mut max_speed, &mut teleport);
        chat.add_ai(&result);
        if let Ok(mut t) = status_q.get_single_mut() {
            t.sections[0].value = "Done!".to_owned();
        }
    }
}
