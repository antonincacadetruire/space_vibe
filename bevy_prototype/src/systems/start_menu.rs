use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, CursorGrabMode, CursorIcon};

use crate::components::*;
use crate::resources::{DeathCause, GameState, GameTimer, SpawnTransform, MouseLook, Throttle, TimePaused, PrevCameraPosition, Leaderboard, MissileSpawnTimer, AlienSpawnTimer};use crate::setup::resolve_ui_font_path;

// ── Shared style helpers (matching existing menu palette) ────────────────────
fn hud_text_color() -> Color { Color::rgb(0.18, 0.95, 0.98) }
fn panel_background() -> Color { Color::rgba(0.01, 0.04, 0.05, 0.96) }
fn btn_normal()  -> Color { Color::rgb(0.03, 0.12, 0.12) }
fn btn_hovered() -> Color { Color::rgb(0.06, 0.26, 0.26) }
fn btn_pressed() -> Color { Color::rgb(0.10, 0.45, 0.45) }

fn btn_style() -> Style {
    Style {
        width: Val::Px(320.0),
        height: Val::Px(56.0),
        margin: UiRect::all(Val::Px(6.0)),
        padding: UiRect::all(Val::Px(10.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

// ── OnEnter(GameState::StartMenu) ─────────────────────────────────────────────
pub fn setup_start_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    leaderboard: Res<Leaderboard>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = true;
        window.cursor.icon = CursorIcon::Arrow;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    let font = asset_server.load(resolve_ui_font_path());
    let label = TextStyle { font: font.clone(), font_size: 22.0, color: hud_text_color() };

    let fmt = |v: f32| {
        let mins = (v / 60.0) as u32;
        let secs = (v % 60.0) as u32;
        let tenths = ((v % 1.0) * 10.0) as u32;
        format!("{:02}:{:02}.{}", mins, secs, tenths)
    };
    let lb_lines: Vec<String> = leaderboard.scores.iter().enumerate().map(|(i, &s)| {
        let medal = match i { 0 => "#1", 1 => "#2", _ => "#3" };
        format!("{}  {}", medal, fmt(s))
    }).collect();
    let panel_height = if lb_lines.is_empty() { 400.0_f32 } else { 500.0_f32 };

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.65).into(),
                ..default()
            },
            StartMenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(620.0),
                        height: Val::Px(panel_height),
                        margin: UiRect::all(Val::Auto),
                        padding: UiRect::all(Val::Px(22.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceEvenly,
                        ..default()
                    },
                    background_color: panel_background().into(),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn(TextBundle::from_section(
                        "SPACE VIBE",
                        TextStyle { font: font.clone(), font_size: 56.0, color: hud_text_color() },
                    ));
                    panel.spawn(TextBundle::from_section(
                        "Navigate the asteroid field",
                        TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.25, 0.90, 0.92) },
                    ));
                    // Leaderboard (only when there are entries)
                    if !lb_lines.is_empty() {
                        panel.spawn(TextBundle::from_section(
                            "── Best Runs ──",
                            TextStyle { font: font.clone(), font_size: 15.0, color: Color::rgb(0.25, 0.90, 0.92) },
                        ));
                        for line in &lb_lines {
                            panel.spawn(TextBundle::from_section(
                                line.clone(),
                                TextStyle { font: font.clone(), font_size: 20.0, color: hud_text_color() },
                            ));
                        }
                    }
                    // Play button
                    panel
                        .spawn((
                            ButtonBundle {
                                style: btn_style(),
                                background_color: btn_normal().into(),
                                ..default()
                            },
                            PlayButton,
                        ))
                        .with_children(|b| {
                            b.spawn(TextBundle::from_section("Play", label.clone()));
                        });
                    // Exit button
                    panel
                        .spawn((
                            ButtonBundle {
                                style: btn_style(),
                                background_color: btn_normal().into(),
                                ..default()
                            },
                            QuitButton,
                        ))
                        .with_children(|b| {
                            b.spawn(TextBundle::from_section("Exit", label.clone()));
                        });
                });
        });
}

// ── OnExit(GameState::StartMenu) ──────────────────────────────────────────────
pub fn teardown_start_menu(mut commands: Commands, q: Query<Entity, With<StartMenuRoot>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// ── OnEnter(GameState::Playing) ───────────────────────────────────────────────
pub fn enter_playing(
    mut game_timer: ResMut<GameTimer>,
    mut throttle: ResMut<Throttle>,
    mut paused: ResMut<TimePaused>,
    mut mouse_look: ResMut<MouseLook>,
    mut prev_cam: ResMut<PrevCameraPosition>,
    mut free_look: ResMut<crate::resources::FreeLook>,
    mut missile_timer: ResMut<MissileSpawnTimer>,
    mut alien_timer: ResMut<AlienSpawnTimer>,
    mut death_cause: ResMut<DeathCause>,
    spawn_transform: Res<SpawnTransform>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    game_timer.0 = 0.0;
    throttle.0 = 0.0;
    paused.0 = false;
    *free_look = crate::resources::FreeLook::default();
    missile_timer.0.reset();
    alien_timer.0.reset();
    *death_cause = DeathCause::default();
    mouse_look.yaw = spawn_transform.yaw;
    mouse_look.pitch = spawn_transform.pitch;
    prev_cam.0 = spawn_transform.transform.translation;
    if let Ok(mut transform) = camera_q.get_single_mut() {
        *transform = spawn_transform.transform;
    }
    if let Ok(mut window) = windows.get_single_mut() {
        use crate::systems::exit::apply_game_cursor;
        apply_game_cursor(&mut window);
    }
}

// ── OnEnter(GameState::Playing) – spawn timer UI + danger HUD ────────────────
pub fn spawn_timer_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(resolve_ui_font_path());
    // Timer (top-right)
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                ..default()
            },
            text: Text::from_section(
                "00:00.0",
                TextStyle { font: font.clone(), font_size: 30.0, color: hud_text_color() },
            ),
            ..default()
        },
        TimerUi,
    ));
    // Missile warning text (top center, hidden until needed)
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Px(60.0),
                ..default()
            },
            text: Text::from_section(
                "",
                TextStyle { font: font.clone(), font_size: 24.0, color: Color::rgb(1.0, 0.15, 0.05) },
            ),
            visibility: Visibility::Hidden,
            ..default()
        },
        MissileWarningUi,
        TimerUi, // share despawn marker
    ));
    // Full-screen danger vignette (transparent by default)
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::rgba(0.6, 0.0, 0.0, 0.0).into(),
            z_index: ZIndex::Global(10),
            ..default()
        },
        DangerVignette,
        TimerUi, // share despawn marker
    ));
}

// ── OnExit(GameState::Playing) – despawn timer UI ────────────────────────────
pub fn despawn_timer_ui(mut commands: Commands, q: Query<Entity, With<TimerUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// ── Update – tick the timer and refresh the HUD text ─────────────────────────
pub fn update_timer(
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut game_timer: ResMut<GameTimer>,
    mut text_q: Query<&mut Text, With<TimerUi>>,
) {
    if !paused.0 {
        game_timer.0 += time.delta_seconds();
    }
    let t = game_timer.0;
    let mins = (t / 60.0) as u32;
    let secs = (t % 60.0) as u32;
    let tenths = ((t % 1.0) * 10.0) as u32;
    for mut text in text_q.iter_mut() {
        text.sections[0].value = format!("{:02}:{:02}.{}", mins, secs, tenths);
    }
}

// ── Update – button appearance (start menu only) ─────────────────────────────
pub fn start_menu_button_appearance_system(
    mut q: Query<
        (&Interaction, &mut BackgroundColor, Option<&PlayButton>, Option<&QuitButton>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg, play, quit) in q.iter_mut() {
        if play.is_some() || quit.is_some() {
            bg.0 = match interaction {
                Interaction::Pressed => btn_pressed(),
                Interaction::Hovered => btn_hovered(),
                Interaction::None    => btn_normal(),
            };
        }
    }
}

// ── Update – button action (start menu only) ─────────────────────────────────
pub fn start_menu_button_system(
    mut q: Query<
        (&Interaction, Option<&PlayButton>, Option<&QuitButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, play, quit) in q.iter_mut() {
        if *interaction == Interaction::Pressed {
            if play.is_some() {
                next_state.set(GameState::Playing);
            } else if quit.is_some() {
                std::process::exit(0);
            }
        }
    }
}

// ── Update – danger vignette + missile warning ────────────────────────────────
pub fn danger_hud_system(
    time: Res<Time>,
    missiles: Query<&Transform, With<crate::components::Missile>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    mut warning_q: Query<(&mut Text, &mut Visibility), With<MissileWarningUi>>,
    mut vignette_q: Query<&mut BackgroundColor, With<DangerVignette>>,
    mut timer_text_q: Query<&mut Text, (With<TimerUi>, Without<MissileWarningUi>)>,
) {
    let Ok(cam) = camera_q.get_single() else { return };

    // Find closest missile distance
    let closest_dist = missiles.iter()
        .map(|t| t.translation.distance(cam.translation))
        .fold(f32::MAX, f32::min);

    const WARN_DIST: f32 = 3_000.0;
    const CRIT_DIST: f32 = 800.0;

    // Vignette alpha
    if let Ok(mut bg) = vignette_q.get_single_mut() {
        let alpha = if closest_dist < WARN_DIST {
            let t = 1.0 - (closest_dist / WARN_DIST).clamp(0.0, 1.0);
            let pulse = if closest_dist < CRIT_DIST {
                ((time.elapsed_seconds() * 6.0).sin() * 0.5 + 0.5) * 0.35
            } else { 0.0 };
            t * 0.28 + pulse
        } else { 0.0 };
        bg.0 = Color::rgba(0.6, 0.0, 0.0, alpha);
    }

    // Warning text
    if let Ok((mut text, mut vis)) = warning_q.get_single_mut() {
        if closest_dist < WARN_DIST {
            *vis = Visibility::Visible;
            text.sections[0].value = format!("!! MISSILE  {:.0} m", closest_dist);
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Timer text color
    if let Ok(mut text) = timer_text_q.get_single_mut() {
        text.sections[0].style.color = if closest_dist < CRIT_DIST {
            let p = (time.elapsed_seconds() * 6.0).sin() * 0.5 + 0.5;
            Color::rgb(1.0, p * 0.2, p * 0.05)
        } else {
            Color::rgb(0.18, 0.95, 0.98)
        };
    }
}
