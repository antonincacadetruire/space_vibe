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
};
use crate::resources::GameState;
use crate::setup::resolve_ui_font_path;
use crate::systems::data_loader::LlmConfigResource;

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

// ── HTTP worker ───────────────────────────────────────────────────────────────

/// Spawns a background thread that calls the LLM API and writes the result
/// into the shared `slot`.
fn spawn_llm_request(
    api_url: String,
    api_key: String,
    model: String,
    system_prompt: String,
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
            "max_tokens": 1500,
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
    let btn_label = TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgb(0.88, 0.96, 1.00) };

    // ── Full-screen transparent click blocker (behind chat panel) ─────────
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

    // ── Root overlay (covers right portion of screen) ─────────────────────
    commands
        .spawn((
            CopilotChatRoot,
            NodeBundle {
                style: Style {
                    display: Display::None, // starts hidden
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Px(460.0),
                    height: Val::Px(480.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                background_color: Color::rgba(0.01, 0.04, 0.07, 0.94).into(),
                z_index: ZIndex::Global(200),
                ..default()
            },
        ))
        .with_children(|root| {
            // Title bar
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(26.0),
                    margin: UiRect::bottom(Val::Px(4.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ..default()
            }).with_children(|bar| {
                bar.spawn(TextBundle::from_section(
                    "COPILOT  [F2 to close]",
                    TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgb(0.18, 0.95, 0.98) },
                ));
                // Status text (right side of title bar)
                bar.spawn((
                    CopilotStatusText,
                    TextBundle::from_section("", TextStyle {
                        font: font.clone(), font_size: 12.0,
                        color: Color::rgb(0.95, 0.75, 0.20),
                    }),
                ));
            });

            // Conversation log (flex-growing)
            root.spawn(NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip(),
                    margin: UiRect::bottom(Val::Px(6.0)),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.02, 0.04, 0.60).into(),
                ..default()
            }).with_children(|log_area| {
                log_area.spawn((
                    CopilotChatLog,
                    TextBundle {
                        text: Text::from_sections(vec![
                            TextSection::new(
                                "[Ask me to generate a map, skin or enemy]\n[Type /setkey YOUR_GITHUB_PAT to set your API key]\n",
                                TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgba(0.55, 0.75, 0.80, 0.55) },
                            ),
                        ]),
                        style: Style {
                            flex_wrap: FlexWrap::Wrap,
                            ..default()
                        },
                        ..default()
                    },
                ));
            });

            // Input row
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(34.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                // Input text box
                row.spawn((
                    CopilotInputBox,
                    NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            height: Val::Px(30.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgba(0.04, 0.14, 0.18, 0.90).into(),
                        ..default()
                    },
                )).with_children(|input_node| {
                    input_node.spawn((
                        CopilotInputText,
                        TextBundle::from_section(
                            "",
                            TextStyle { font: font.clone(), font_size: 13.0, color: Color::WHITE },
                        ),
                    ));
                });

                // Send button
                row.spawn((
                    CopilotSendButton,
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(60.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.03, 0.20, 0.22).into(),
                        ..default()
                    },
                )).with_children(|btn| {
                    btn.spawn(TextBundle::from_section("Send", btn_label.clone()));
                });
            });

            // Save row
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(30.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                row.spawn((
                    CopilotSaveButton,
                    ButtonBundle {
                        style: Style {
                            display: Display::None, // hidden until a JSON block arrives
                            width: Val::Px(200.0),
                            height: Val::Px(26.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.05, 0.28, 0.12).into(),
                        ..default()
                    },
                )).with_children(|btn| {
                    btn.spawn(TextBundle::from_section(
                        "Save JSON to data/",
                        TextStyle { font: font.clone(), font_size: 12.0, color: Color::rgb(0.45, 1.0, 0.60) },
                    ));
                });

                // Change API key button (always visible)
                row.spawn((
                    CopilotChangeKeyButton,
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(100.0),
                            height: Val::Px(26.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::rgb(0.18, 0.10, 0.04).into(),
                        ..default()
                    },
                )).with_children(|btn| {
                    btn.spawn(TextBundle::from_section(
                        "Change Key",
                        TextStyle { font: font.clone(), font_size: 12.0, color: Color::rgb(1.0, 0.75, 0.30) },
                    ));
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
) {
    let state = game_state.get();
    if *state != GameState::Playing && *state != GameState::StartMenu {
        return;
    }

    if keys.just_pressed(KeyCode::F2) {
        chat.open = !chat.open;
        // cursor management only in Playing state
        if *state == GameState::Playing {
            if let Ok(mut win) = windows.get_single_mut() {
                if chat.open {
                    win.cursor.visible = true;
                    win.cursor.grab_mode = CursorGrabMode::None;
                    win.cursor.icon = CursorIcon::Text;
                } else {
                    win.cursor.visible = false;
                    win.cursor.grab_mode = CursorGrabMode::Locked;
                    win.cursor.icon = CursorIcon::Default;
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
    mut save_q: Query<&mut Style, With<CopilotSaveButton>>,
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
                let display_text = if chat.last_json.is_some() {
                    format!("{content}\n[JSON block detected – click Save to write to disk]")
                } else {
                    content
                };
                chat.add_ai(&display_text);
            }
        }
    }

    // Show/hide Save button
    let save_display = if chat.last_json.is_some() { Display::Flex } else { Display::None };
    for mut style in &mut save_q {
        style.display = save_display;
    }

    // Rebuild conversation log text
    let font = asset_server.load(resolve_ui_font_path());
    {
        let mut log_q = text_queries.p0();
        if let Ok(mut log_text) = log_q.get_single_mut() {
            if chat.conversation.is_empty() {
                log_text.sections = vec![TextSection::new(
                    "[Ask me to generate a map, skin or enemy – e.g. \"generate a toxic gas planet map\"]\n",
                    TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgba(0.55, 0.75, 0.80, 0.55) },
                )];
            } else {
                log_text.sections = chat.conversation.iter().map(|msg| {
                    let (prefix, color) = if msg.is_user {
                        ("You: ", Color::rgb(0.35, 0.88, 0.55))
                    } else {
                        ("AI:  ", Color::rgb(0.18, 0.80, 0.95))
                    };
                    TextSection::new(
                        format!("{prefix}{}\n", msg.text),
                        TextStyle { font: font.clone(), font_size: 13.0, color },
                    )
                }).collect();
            }
        }
    }
}

// ── Save button system ─────────────────────────────────────────────────────────

pub fn llm_chat_save_system(
    mut chat: ResMut<LlmChatState>,
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<CopilotSaveButton>)>,
    change_key_q: Query<&Interaction, (Changed<Interaction>, With<CopilotChangeKeyButton>)>,
    mut status_q: Query<&mut Text, (With<CopilotStatusText>, Without<CopilotChatLog>, Without<CopilotInputText>)>,
) {
    // ── Change Key button
    for interaction in &change_key_q {
        if *interaction != Interaction::Pressed { continue; }
        chat.awaiting_api_key = true;
        chat.add_ai("Please paste your new GitHub Personal Access Token (PAT):");
    }
    for interaction in &interaction_q {
        if *interaction != Interaction::Pressed { continue; }
        if let Some(json) = &chat.last_json.clone() {
            let kind = infer_kind(json);
            match save_json(json, kind) {
                Ok(path) => {
                    let msg = format!("Saved to {path}");
                    chat.add_ai(&msg);
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
}
