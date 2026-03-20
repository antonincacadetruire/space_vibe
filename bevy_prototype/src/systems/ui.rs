use bevy::prelude::*;

use crate::components::{SpeedUi, Shuttle};
use crate::resources::Throttle;
use crate::components::CursorCross;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

pub fn ui_update_system(
    throttle: Res<Throttle>,
    mut speed_text_q: Query<&mut Text, With<SpeedUi>>,
    shuttle_q: Query<&Transform, (With<Shuttle>, Without<crate::components::CompassNeedle>)>,
    mut needle_q: Query<&mut Transform, (With<crate::components::CompassNeedle>, Without<Shuttle>)>,
) {
    // update speed text
    if let Ok(mut text) = speed_text_q.get_single_mut() {
        let speed_val = throttle.0 * crate::SHUTTLE_SPEED;
        text.sections[0].value = format!("Speed: {:.1}", speed_val);
    }

    // rotate needle image to match heading
    if let Ok(mut ntrans) = needle_q.get_single_mut() {
        // needle should point to heading; convert deg to radians and rotate about Z
        let Ok(transform) = shuttle_q.get_single() else { return };
        let forward = transform.rotation.mul_vec3(Vec3::NEG_Z).normalize_or_zero();
        let ang = forward.z.atan2(forward.x).to_degrees();
        let mut deg = (90.0 - ang) % 360.0;
        if deg < 0.0 { deg += 360.0 }
        let rad = deg.to_radians();
        ntrans.rotation = Quat::from_rotation_z(-rad);
    }
}

pub fn cursor_follow_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cross_q: Query<&mut Style, With<CursorCross>>,
) {
    let Ok(window) = windows.get_single() else { return };
    // keep crosshair fixed at screen center
    let cross_w = 24.0_f32;
    let left = (window.width() / 2.0) - (cross_w / 2.0);
    let bottom = (window.height() / 2.0) - (cross_w / 2.0);
    for mut style in cross_q.iter_mut() {
        style.position_type = PositionType::Absolute;
        style.left = Val::Px(left);
        style.bottom = Val::Px(bottom);
    }
}
