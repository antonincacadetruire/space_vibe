use bevy::prelude::*;
use bevy::app::AppExit;

pub fn exit_on_escape_system(keyboard: Res<Input<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}
