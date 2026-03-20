use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;

use crate::components::*;
use crate::resources::{MenuState, Keybindings, RebindState, Action, MouseLook};

pub fn menu_ui_system(
    mut commands: Commands,
    menu: Res<MenuState>,
    menu_q: Query<Entity, With<MenuRoot>>,
    asset_server: Res<AssetServer>,
) {
    // spawn UI when menu opens, despawn when closed
    if menu.open {
        if menu_q.iter().next().is_none() {
            // spawn menu root as a full-screen overlay
            let font = asset_server.load("fonts/FiraSans-Bold.ttf");
            commands.spawn((NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.4).into(),
                ..default()
            }, MenuRoot)).with_children(|parent| {
                // centered panel
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(600.0),
                        height: Val::Px(360.0),
                        margin: UiRect::all(Val::Auto),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceEvenly,
                        ..default()
                    },
                    background_color: Color::rgba(0.05, 0.05, 0.05, 0.95).into(),
                    ..default()
                }).with_children(|panel| {
                    panel.spawn(TextBundle::from_section("Menu", TextStyle { font: font.clone(), font_size: 36.0, color: Color::WHITE }));

                    // Buttons column (stacked vertically) matching requested layout
                    panel.spawn(NodeBundle { style: Style { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }, ..default() }).with_children(|col| {
                        let btn_style = Style { width: Val::Percent(80.0), height: Val::Px(56.0), margin: UiRect::all(Val::Px(6.0)), padding: UiRect::all(Val::Px(6.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() };

                        // Settings
                        col.spawn((ButtonBundle { style: btn_style.clone(), background_color: Color::WHITE.into(), ..default() }, SettingsButton)).with_children(|b| {
                            b.spawn(TextBundle::from_section("Settings", TextStyle { font: font.clone(), font_size: 22.0, color: Color::BLACK }));
                        });

                        // Commands
                        col.spawn((ButtonBundle { style: btn_style.clone(), background_color: Color::WHITE.into(), ..default() }, CommandsButton)).with_children(|b| {
                            b.spawn(TextBundle::from_section("Commands", TextStyle { font: font.clone(), font_size: 22.0, color: Color::BLACK }));
                        });

                        // Exit / Quit
                        col.spawn((ButtonBundle { style: btn_style.clone(), background_color: Color::rgb(0.9,0.2,0.2).into(), ..default() }, QuitButton)).with_children(|b| {
                            b.spawn(TextBundle::from_section("Exit", TextStyle { font: font.clone(), font_size: 22.0, color: Color::WHITE }));
                        });
                    });

                    // Settings row with sensitivity controls
                    panel.spawn(NodeBundle { style: Style { flex_direction: FlexDirection::Row, ..default() }, ..default() }).with_children(|settings| {
                        settings.spawn(TextBundle::from_section("Sensitivity:", TextStyle { font: font.clone(), font_size: 20.0, color: Color::WHITE }));
                        settings.spawn((ButtonBundle { ..default() }, SensDecreaseButton)).with_children(|b| { b.spawn(TextBundle::from_section("-", TextStyle { font: font.clone(), font_size: 20.0, color: Color::WHITE })); });
                        settings.spawn((TextBundle::from_section("", TextStyle { font: font.clone(), font_size: 20.0, color: Color::WHITE }), SensitivityText));
                        settings.spawn((ButtonBundle { ..default() }, SensIncreaseButton)).with_children(|b| { b.spawn(TextBundle::from_section("+", TextStyle { font: font.clone(), font_size: 20.0, color: Color::WHITE })); });
                    });

                    // Commands hint
                    panel.spawn(TextBundle::from_section("Open 'Commands' to rebind keys", TextStyle { font: font.clone(), font_size: 14.0, color: Color::GRAY }));
                });
            });
        }
    } else {
        // menu closed: despawn existing menu
        for e in menu_q.iter() {
            commands.entity(e).despawn_recursive();
        }
    }
}

pub fn menu_button_system(
    mut interaction_q: Query<(&Interaction, &Children, Option<&ResumeButton>, Option<&SettingsButton>, Option<&CommandsButton>, Option<&QuitButton>), (Changed<Interaction>, With<Button>)>,
    mut menu: ResMut<MenuState>,
    mut paused: ResMut<crate::resources::TimePaused>,
    mut rebind: ResMut<RebindState>,
    keyb: Res<Keybindings>,
) {
    for (interaction, children, resume, settings, commands_btn, quit) in interaction_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            if resume.is_some() {
                // close menu and restore previous pause
                paused.0 = menu.prev_paused;
                menu.open = false;
            } else if quit.is_some() {
                std::process::exit(0);
            } else if commands_btn.is_some() {
                // entering rebind mode for demo: set first action as waiting
                rebind.0 = Some(Action::ThrottleUp);
            }
        }
    }
}

pub fn sensitivity_button_system(
    mut interaction_q: Query<(&Interaction, Option<&SensIncreaseButton>, Option<&SensDecreaseButton>), (Changed<Interaction>, With<Button>)>,
    mut mouse: ResMut<MouseLook>,
) {
    for (interaction, inc, dec) in interaction_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            if inc.is_some() {
                mouse.sensitivity = (mouse.sensitivity + 0.0005).min(0.05);
            }
            if dec.is_some() {
                mouse.sensitivity = (mouse.sensitivity - 0.0005).max(0.0001);
            }
        }
    }
}

pub fn sensitivity_text_system(mut text_q: Query<&mut Text, With<SensitivityText>>, mouse: Res<MouseLook>) {
    if let Ok(mut text) = text_q.get_single_mut() {
        text.sections[0].value = format!("{:.4}", mouse.sensitivity);
    }
}

pub fn key_capture_system(
    mut key_evr: EventReader<KeyboardInput>,
    mut rebind: ResMut<RebindState>,
    mut keyb: ResMut<Keybindings>,
) {
    if rebind.0.is_none() { return; }
    for ev in key_evr.iter() {
        if let Some(code) = ev.key_code {
            if ev.state == bevy::input::ButtonState::Pressed {
                match rebind.0.unwrap() {
                    Action::ThrottleUp => keyb.throttle_up = code,
                    Action::ThrottleDown => keyb.throttle_down = code,
                    Action::VerticalUp => keyb.vertical_up = code,
                    Action::VerticalDown => keyb.vertical_down = code,
                    Action::TogglePause => keyb.toggle_pause = code,
                    Action::ToggleMenu => keyb.toggle_menu = code,
                }
                rebind.0 = None;
                break;
            }
        }
    }
}
