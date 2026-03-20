use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;

use crate::components::*;
use crate::resources::{MenuState, Keybindings, RebindState, Action, MouseLook};
use crate::setup::resolve_ui_font_path;

fn green_text_color() -> Color {
    Color::rgb(0.20, 1.0, 0.35)
}

fn panel_background() -> Color {
    Color::rgba(0.03, 0.08, 0.03, 0.96)
}

fn panel_style(width: f32, height: f32) -> Style {
    Style {
        width: Val::Px(width),
        height: Val::Px(height),
        margin: UiRect::all(Val::Auto),
        padding: UiRect::all(Val::Px(22.0)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceEvenly,
        ..default()
    }
}

fn button_style(width: f32, height: f32) -> Style {
    Style {
        width: Val::Px(width),
        height: Val::Px(height),
        margin: UiRect::all(Val::Px(6.0)),
        padding: UiRect::all(Val::Px(10.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn menu_button_palette(is_quit: bool) -> (Color, Color, Color) {
    if is_quit {
        (
            Color::rgb(0.20, 0.07, 0.07),
            Color::rgb(0.34, 0.10, 0.10),
            Color::rgb(0.50, 0.14, 0.14),
        )
    } else {
        (
            Color::rgb(0.05, 0.16, 0.05),
            Color::rgb(0.09, 0.26, 0.09),
            Color::rgb(0.14, 0.36, 0.14),
        )
    }
}

fn settings_button_palette() -> (Color, Color, Color) {
    (
        Color::rgb(0.06, 0.18, 0.06),
        Color::rgb(0.10, 0.30, 0.10),
        Color::rgb(0.16, 0.42, 0.16),
    )
}

fn button_fill(interaction: &Interaction, normal: Color, hovered: Color, pressed: Color) -> Color {
    match interaction {
        Interaction::Pressed => pressed,
        Interaction::Hovered => hovered,
        Interaction::None => normal,
    }
}

fn spawn_main_menu(panel: &mut ChildBuilder, font: Handle<Font>) {
    panel.spawn(TextBundle::from_section(
        "Menu",
        TextStyle { font: font.clone(), font_size: 40.0, color: green_text_color() },
    ));

    panel
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|col| {
            let (normal, _, _) = menu_button_palette(false);
            let label_style = TextStyle { font: font.clone(), font_size: 22.0, color: green_text_color() };

            col.spawn((ButtonBundle { style: button_style(320.0, 56.0), background_color: normal.into(), ..default() }, ResumeButton))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section("Resume", label_style.clone()));
                });
            col.spawn((ButtonBundle { style: button_style(320.0, 56.0), background_color: normal.into(), ..default() }, SettingsButton))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section("Settings", label_style.clone()));
                });
            col.spawn((ButtonBundle { style: button_style(320.0, 56.0), background_color: normal.into(), ..default() }, CommandsButton))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section("Commands", label_style.clone()));
                });
            col.spawn((ButtonBundle { style: button_style(320.0, 56.0), background_color: menu_button_palette(true).0.into(), ..default() }, QuitButton))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section("Exit", label_style));
                });
        });

    panel.spawn(TextBundle::from_section(
        "Open Settings to adjust mouse sensitivity",
        TextStyle { font: font.clone(), font_size: 14.0, color: Color::rgb(0.45, 0.85, 0.45) },
    ));
}

fn spawn_settings_menu(panel: &mut ChildBuilder, font: Handle<Font>) {
    panel.spawn(TextBundle::from_section(
        "Settings",
        TextStyle { font: font.clone(), font_size: 40.0, color: green_text_color() },
    ));

    panel
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            let (normal, _, _) = settings_button_palette();
            let label_style = TextStyle { font: font.clone(), font_size: 20.0, color: green_text_color() };

            row.spawn(TextBundle::from_section(
                "Sensitivity:",
                TextStyle { font: font.clone(), font_size: 20.0, color: green_text_color() },
            ));
            row.spawn((ButtonBundle { style: button_style(48.0, 48.0), background_color: normal.into(), ..default() }, SensDecreaseButton))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section("-", label_style.clone()));
                });
            row.spawn((TextBundle::from_section("", TextStyle { font: font.clone(), font_size: 22.0, color: green_text_color() }), SensitivityText));
            row.spawn((ButtonBundle { style: button_style(48.0, 48.0), background_color: normal.into(), ..default() }, SensIncreaseButton))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section("+", label_style));
                });
        });

    panel.spawn((ButtonBundle { style: button_style(220.0, 50.0), background_color: menu_button_palette(false).0.into(), ..default() }, SettingsBackButton)).with_children(|b| {
        b.spawn(TextBundle::from_section(
            "Back",
            TextStyle { font: font.clone(), font_size: 20.0, color: green_text_color() },
        ));
    });
}

pub fn menu_ui_system(
    mut commands: Commands,
    menu: Res<MenuState>,
    menu_q: Query<Entity, With<MenuRoot>>,
    main_panel_q: Query<Entity, With<MainMenuPanel>>,
    settings_panel_q: Query<Entity, With<SettingsPanel>>,
    asset_server: Res<AssetServer>,
) {
    // spawn UI when menu opens, despawn when closed
    if menu.open {
        let root_entity = menu_q.iter().next();
        let main_panel_present = main_panel_q.iter().next().is_some();
        let settings_panel_present = settings_panel_q.iter().next().is_some();
        let correct_panel_present = if menu.settings_open { settings_panel_present } else { main_panel_present };

        if root_entity.is_none() || !correct_panel_present {
            if let Some(root) = root_entity {
                commands.entity(root).despawn_recursive();
            }

            // spawn menu root as a full-screen overlay
            let font = asset_server.load(resolve_ui_font_path());
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
                if menu.settings_open {
                    parent.spawn((NodeBundle {
                        style: panel_style(620.0, 320.0),
                        background_color: panel_background().into(),
                        ..default()
                    }, SettingsPanel)).with_children(|panel| {
                        spawn_settings_menu(panel, font.clone());
                    });
                } else {
                    parent.spawn((NodeBundle {
                        style: panel_style(620.0, 380.0),
                        background_color: panel_background().into(),
                        ..default()
                    }, MainMenuPanel)).with_children(|panel| {
                        spawn_main_menu(panel, font.clone());
                    });
                }
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
    mut interaction_q: Query<(
        &Interaction,
        Option<&ResumeButton>,
        Option<&SettingsButton>,
        Option<&CommandsButton>,
        Option<&QuitButton>,
        Option<&SettingsBackButton>,
    ), (Changed<Interaction>, With<Button>)>,
    mut menu: ResMut<MenuState>,
    mut paused: ResMut<crate::resources::TimePaused>,
    mut rebind: ResMut<RebindState>,
) {
    for (interaction, resume, settings, commands_btn, quit, settings_back) in interaction_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            if resume.is_some() {
                // close menu and restore previous pause
                paused.0 = menu.prev_paused;
                menu.open = false;
                menu.settings_open = false;
            } else if settings.is_some() {
                menu.settings_open = true;
            } else if quit.is_some() {
                std::process::exit(0);
            } else if settings_back.is_some() {
                menu.settings_open = false;
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

pub fn button_appearance_system(
    mut interaction_q: Query<(
        &Interaction,
        &mut BackgroundColor,
        Option<&ResumeButton>,
        Option<&SettingsButton>,
        Option<&CommandsButton>,
        Option<&QuitButton>,
        Option<&SensIncreaseButton>,
        Option<&SensDecreaseButton>,
        Option<&SettingsBackButton>,
    ), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut background, resume, settings, commands_btn, quit, sens_inc, sens_dec, settings_back) in interaction_q.iter_mut() {
        let (normal, hovered, pressed) = if quit.is_some() {
            menu_button_palette(true)
        } else if sens_inc.is_some() || sens_dec.is_some() {
            settings_button_palette()
        } else if resume.is_some() || settings.is_some() || commands_btn.is_some() || settings_back.is_some() {
            menu_button_palette(false)
        } else {
            continue;
        };

        background.0 = button_fill(interaction, normal, hovered, pressed).into();
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
