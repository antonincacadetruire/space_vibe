use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, CursorGrabMode, CursorIcon};

use crate::components::*;
use crate::resources::{GameState, GameTimer};
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
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let t = game_timer.0;
    let mins = (t / 60.0) as u32;
    let secs = (t % 60.0) as u32;
    let tenths = ((t % 1.0) * 10.0) as u32;
    let score_text = format!("Survived  {:02}:{:02}.{}", mins, secs, tenths);

    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = true;
        window.cursor.icon = CursorIcon::Arrow;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    let font = asset_server.load(resolve_ui_font_path());
    let label = TextStyle { font: font.clone(), font_size: 22.0, color: hud_text_color() };

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
                        width: Val::Px(620.0),
                        height: Val::Px(440.0),
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
                    // Title
                    panel.spawn(TextBundle::from_section(
                        "CRASHED",
                        TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(0.95, 0.20, 0.20) },
                    ));
                    // Score
                    panel.spawn(TextBundle::from_section(
                        score_text,
                        TextStyle { font: font.clone(), font_size: 28.0, color: hud_text_color() },
                    ));
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
