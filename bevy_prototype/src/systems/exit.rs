use bevy::prelude::*;
use bevy::window::{PrimaryWindow, CursorGrabMode, CursorIcon};
use crate::resources::{MenuState, TimePaused, Keybindings};

pub fn apply_game_cursor(window: &mut Window) {
    window.cursor.visible = false;
    window.cursor.icon = CursorIcon::Crosshair;
    window.cursor.grab_mode = CursorGrabMode::Locked;
}

pub fn apply_menu_cursor(window: &mut Window) {
    window.cursor.visible = true;
    window.cursor.icon = CursorIcon::Arrow;
    window.cursor.grab_mode = CursorGrabMode::None;
}

pub fn toggle_menu_system(
    keyboard: Res<Input<KeyCode>>,
    keyb: Res<Keybindings>,
    mut menu: ResMut<MenuState>,
    mut paused: ResMut<TimePaused>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let toggle = keyb.toggle_menu;
    if keyboard.just_pressed(toggle) {
        // toggle menu
        if !menu.open {
            // opening menu: remember previous paused state and pause the world
            menu.prev_paused = paused.0;
            paused.0 = true;
            menu.open = true;
            menu.settings_open = false;
            if let Ok(mut window) = windows.get_single_mut() {
                apply_menu_cursor(&mut window);
            }
        } else {
            // closing menu: restore previous paused state
            paused.0 = menu.prev_paused;
            menu.open = false;
            menu.settings_open = false;
            if let Ok(mut window) = windows.get_single_mut() {
                apply_game_cursor(&mut window);
            }
        }
    }
}
