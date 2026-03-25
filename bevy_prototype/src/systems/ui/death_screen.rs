use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, CursorGrabMode, CursorIcon};

use crate::components::*;
use crate::resources::{ActiveScene, DeathCause, GameState, GameTimer, KillCount, SceneLeaderboard};
use crate::setup::resolve_ui_font_path;

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

// ── OnEnter(GameState::Dead) ──────────────────────────────────────────────────
pub fn setup_death_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_timer: Res<GameTimer>,
    death_cause: Res<DeathCause>,
    kill_count: Res<KillCount>,
    active_scene: Res<ActiveScene>,
    mut leaderboard: ResMut<SceneLeaderboard>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let t = game_timer.0;
    leaderboard.submit(&active_scene.0, t);
    leaderboard.save(&active_scene.0);

    let (death_title, death_subtitle, title_color) = match *death_cause {
        DeathCause::Missile => (
            "INTERCEPTED",
            "Your ship was destroyed by a missile",
            Color::rgb(0.95, 0.45, 0.05),
        ),
        DeathCause::Asteroid => (
            "CRASHED",
            "You collided with an asteroid",
            Color::rgb(0.95, 0.20, 0.20),
        ),
        DeathCause::Terrain => (
            "IMPACT",
            "You flew into the terrain",
            Color::rgb(0.80, 0.55, 0.10),
        ),
    };

    let fmt = |v: f32| {
        let mins = (v / 60.0) as u32;
        let secs = (v % 60.0) as u32;
        let tenths = ((v % 1.0) * 10.0) as u32;
        format!("{:02}:{:02}.{}", mins, secs, tenths)
    };
    let score_text = format!("Survived {}   Kills: {}", fmt(t), kill_count.0);
    let scene_name = active_scene.0.label();

    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = true;
        window.cursor.icon = CursorIcon::Arrow;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    let font = asset_server.load(resolve_ui_font_path());
    let label = TextStyle { font: font.clone(), font_size: 22.0, color: hud_text_color() };

    // Build leaderboard lines for the active scene
    let scores = leaderboard.scores(&active_scene.0);
    let lb_lines: Vec<String> = scores.iter().enumerate().map(|(i, &s)| {
        let medal = match i { 0 => "#1", 1 => "#2", _ => "#3" };
        format!("{}  {}", medal, fmt(s))
    }).collect();

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.72).into(),
                ..default()
            },
            DeathScreenRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(660.0),
                        height: Val::Px(560.0),
                        margin: UiRect::all(Val::Auto),
                        padding: UiRect::all(Val::Px(24.0)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceEvenly,
                        ..default()
                    },
                    background_color: panel_background().into(),
                    ..default()
                })
                .with_children(|panel| {
                    // Title
                    panel.spawn(TextBundle::from_section(
                        death_title,
                        TextStyle { font: font.clone(), font_size: 52.0, color: title_color },
                    ));
                    // Scene name (small subtitle above death reason)
                    panel.spawn(TextBundle::from_section(
                        scene_name,
                        TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(0.55, 0.75, 0.90) },
                    ));
                    // Subtitle
                    panel.spawn(TextBundle::from_section(
                        death_subtitle,
                        TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.80, 0.70, 0.50) },
                    ));
                    // Score this run
                    panel.spawn(TextBundle::from_section(
                        score_text,
                        TextStyle { font: font.clone(), font_size: 28.0, color: hud_text_color() },
                    ));
                    // Leaderboard separator
                    panel.spawn(TextBundle::from_section(
                        "── Best Runs ──",
                        TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(0.25, 0.90, 0.92) },
                    ));
                    for line in &lb_lines {
                        panel.spawn(TextBundle::from_section(
                            line.clone(),
                            TextStyle { font: font.clone(), font_size: 20.0, color: hud_text_color() },
                        ));
                    }
                    // Play Again
                    panel
                        .spawn((
                            ButtonBundle {
                                style: btn_style(),
                                background_color: btn_normal().into(),
                                ..default()
                            },
                            PlayAgainButton,
                        ))
                        .with_children(|b| {
                            b.spawn(TextBundle::from_section("Play Again", label.clone()));
                        });
                    // Main Menu
                    panel
                        .spawn((
                            ButtonBundle {
                                style: btn_style(),
                                background_color: btn_normal().into(),
                                ..default()
                            },
                            HomeButton,
                        ))
                        .with_children(|b| {
                            b.spawn(TextBundle::from_section("Main Menu", label.clone()));
                        });
                    // Exit
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

// ── OnExit(GameState::Dead) ───────────────────────────────────────────────────
pub fn teardown_death_screen(mut commands: Commands, q: Query<Entity, With<DeathScreenRoot>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// ── Update – button appearance (death screen only) ───────────────────────────
pub fn death_screen_button_appearance_system(
    mut q: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&PlayAgainButton>,
            Option<&HomeButton>,
            Option<&QuitButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut bg, play_again, home, quit) in q.iter_mut() {
        if play_again.is_some() || home.is_some() || quit.is_some() {
            bg.0 = match interaction {
                Interaction::Pressed => btn_pressed(),
                Interaction::Hovered => btn_hovered(),
                Interaction::None    => btn_normal(),
            };
        }
    }
}

// ── Update – button action (death screen only) ───────────────────────────────
pub fn death_screen_button_system(
    mut q: Query<
        (
            &Interaction,
            Option<&PlayAgainButton>,
            Option<&HomeButton>,
            Option<&QuitButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, play_again, home, quit) in q.iter_mut() {
        if *interaction == Interaction::Pressed {
            if play_again.is_some() {
                next_state.set(GameState::Playing);
            } else if home.is_some() {
                next_state.set(GameState::StartMenu);
            } else if quit.is_some() {
                std::process::exit(0);
            }
        }
    }
}
