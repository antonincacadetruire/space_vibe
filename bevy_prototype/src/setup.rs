use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, CursorIcon, CursorGrabMode};

use crate::resources::{MouseLook, PrevCameraPosition, SpawnTransform};
use crate::systems::scenes::space_scene::make_cinematic_bloom;

pub fn resolve_ui_font_path() -> &'static str {
    use std::path::Path;

    if Path::new("assets/fonts/FiraSans-Bold.ttf").exists() {
        "fonts/FiraSans-Bold.ttf"
    } else if Path::new("C:\\Windows\\Fonts\\arial.ttf").exists() {
        "C:\\Windows\\Fonts\\arial.ttf"
    } else if Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf").exists() {
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
    } else {
        warn!("No local or system font found; text UI may not render until a font is added to assets/fonts/");
        "fonts/FiraSans-Bold.ttf"
    }
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    mut mouse_look: ResMut<MouseLook>,
    mut prev_camera_position: ResMut<PrevCameraPosition>,
) {
    // Camera starts at a neutral position; the actual spawn position is set
    // by spawn_active_scene_system when OnEnter(Playing) fires.
    let default_transform = Transform::from_xyz(0.0, 5_000.0, 60_000.0)
        .looking_at(Vec3::ZERO, Vec3::Y);
    mouse_look.yaw = 0.0;
    mouse_look.pitch = 0.0;
    prev_camera_position.0 = default_transform.translation;
    commands.insert_resource(SpawnTransform { transform: default_transform, yaw: 0.0, pitch: 0.0 });

    commands.spawn((
        Camera3dBundle {
            transform: default_transform,
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: PerspectiveProjection {
                far: 4_000_000.0,
                ..default()
            }
            .into(),
            ..default()
        },
        make_cinematic_bloom(),
        crate::components::MainCamera,
    ));

    // Start with visible cursor — the start menu is the first screen shown.
    // OnEnter(GameState::Playing) will lock it when gameplay begins.
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = true;
        window.cursor.icon = CursorIcon::Arrow;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    // (OS cursor hidden) — we draw an on-screen UI cross to indicate pointer

    // UI: speed and compass (bottom-left)
    // Use the same font fallback logic as the menu so text renders consistently.
    let font = asset_server.load(resolve_ui_font_path());

    // Procedurally generate a neon compass dial and needle for the futuristic HUD
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let dial_size: u32 = 176;
    let mut dial_data = vec![0u8; (dial_size * dial_size * 4) as usize];
    let cx = (dial_size / 2) as i32;
    let cy = (dial_size / 2) as i32;
    let radius = (dial_size as i32) / 2 - 2;
    for y in 0..(dial_size as i32) {
        for x in 0..(dial_size as i32) {
            let dx = x - cx;
            let dy = y - cy;
            let dist2 = dx * dx + dy * dy;
            let dist = (dist2 as f32).sqrt();
            let idx = ((y * dial_size as i32 + x) * 4) as usize;
            if dist2 <= radius * radius {
                // neon teal outer rim, darker inner fills for HUD look
                if dist > radius as f32 * 0.92 {
                    // bright neon rim
                    dial_data[idx] = 10;
                    dial_data[idx + 1] = 240;
                    dial_data[idx + 2] = 230;
                } else if dist > radius as f32 * 0.70 {
                    // mid ring / subtle surface
                    dial_data[idx] = 6;
                    dial_data[idx + 1] = 18;
                    dial_data[idx + 2] = 20;
                } else {
                    // central darker fill
                    dial_data[idx] = 3;
                    dial_data[idx + 1] = 8;
                    dial_data[idx + 2] = 10;
                }
                dial_data[idx + 3] = 255; // a
            } else {
                // transparent
                dial_data[idx] = 0;
                dial_data[idx + 1] = 0;
                dial_data[idx + 2] = 0;
                dial_data[idx + 3] = 0;
            }
        }
    }

    // add simple tick marks (every 45 deg)
    for &angle_deg in &[0.0_f32, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0] {
        let a = angle_deg.to_radians();
        let (sx, sy) = ( (cx as f32 + (a.cos() * (radius as f32 * 0.9))).round() as i32,
                         (cy as f32 - (a.sin() * (radius as f32 * 0.9))).round() as i32 );
        for dy in -2..=2 {
            for dx in -2..=2 {
                let x = sx + dx;
                let y = sy + dy;
                if x>=0 && x < dial_size as i32 && y>=0 && y<dial_size as i32 {
                    let idx = ((y * dial_size as i32 + x) * 4) as usize;
                    // tick mark bright cyan
                    dial_data[idx] = 48;
                    dial_data[idx+1] = 255;
                    dial_data[idx+2] = 240;
                    dial_data[idx+3] = 255;
                }
            }
        }
    }

    // crosshair and center glow
    for offset in -1..=1 {
        for x in (cx - 10)..=(cx + 10) {
            let y = cy + offset;
            if x>=0 && x < dial_size as i32 && y>=0 && y<dial_size as i32 {
                let idx = ((y * dial_size as i32 + x) * 4) as usize;
                dial_data[idx] = 60;
                dial_data[idx+1] = 255;
                dial_data[idx+2] = 170;
                dial_data[idx+3] = 180;
            }
        }
        for y in (cy - 10)..=(cy + 10) {
            let x = cx + offset;
            if x>=0 && x < dial_size as i32 && y>=0 && y<dial_size as i32 {
                let idx = ((y * dial_size as i32 + x) * 4) as usize;
                // small center cross glow in neon cyan
                dial_data[idx] = 48;
                dial_data[idx+1] = 255;
                dial_data[idx+2] = 240;
                dial_data[idx+3] = 200;
            }
        }
    }

    let dial_image = Image::new(
        Extent3d {
            width: dial_size,
            height: dial_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        dial_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    let dial_handle = images.add(dial_image);

    // needle: tall neon triangle
    let n_w: u32 = 18;
    let n_h: u32 = 120;
    let mut needle_data = vec![0u8; (n_w * n_h * 4) as usize];
    let mid = (n_w / 2) as i32;
    for y in 0..(n_h as i32) {
        // triangle width decreases towards tip
        let t = 1.0 - (y as f32 / n_h as f32);
        let half = ((mid as f32) * t) as i32;
        for x in 0..(n_w as i32) {
            let idx = ((y * n_w as i32 + x) * 4) as usize;
            if (x - mid).abs() <= half {
                // neon cyan needle
                needle_data[idx] = 60;
                needle_data[idx + 1] = 255;
                needle_data[idx + 2] = 245;
                needle_data[idx + 3] = 255;
            } else {
                needle_data[idx] = 0;
                needle_data[idx + 1] = 0;
                needle_data[idx + 2] = 0;
                needle_data[idx + 3] = 0;
            }
        }
    }
    let needle_image = Image::new(
        Extent3d {
            width: n_w,
            height: n_h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        needle_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    let needle_handle = images.add(needle_image);

    // spawn dial and needle as UI ImageBundles (bottom-left)
    commands.spawn(ImageBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(72.0),
            width: Val::Px(dial_size as f32),
            height: Val::Px(dial_size as f32),
            ..default()
        },
        image: UiImage::new(dial_handle.clone()),
        ..default()
    })
    .insert(crate::components::CompassDial);

    // needle placed over dial; add Transform so we can rotate it
    commands.spawn(ImageBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0 + (dial_size as f32 - n_w as f32) / 2.0),
            bottom: Val::Px(72.0 + (dial_size as f32 - n_h as f32) / 2.0),
            width: Val::Px(n_w as f32),
            height: Val::Px(n_h as f32),
            ..default()
        },
        image: UiImage::new(needle_handle.clone()),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..default()
    })
    .insert(crate::components::CompassNeedle);

    // spawn a small crosshair UI image that we'll follow with the mouse

    // small cross image (24x24) white cross on transparent background
    let cx_size: u32 = 24;
    let mut cross_data = vec![0u8; (cx_size * cx_size * 4) as usize];
    let mid = (cx_size / 2) as i32;
    for y in 0..(cx_size as i32) {
        for x in 0..(cx_size as i32) {
            let idx = ((y * cx_size as i32 + x) * 4) as usize;
            // draw a 1px cross centered
            if x == mid || y == mid {
                cross_data[idx] = 255;
                cross_data[idx + 1] = 255;
                cross_data[idx + 2] = 255;
                cross_data[idx + 3] = 255;
            } else {
                cross_data[idx] = 0;
                cross_data[idx + 1] = 0;
                cross_data[idx + 2] = 0;
                cross_data[idx + 3] = 0;
            }
        }
    }
    let cross_image = Image::new(
        Extent3d {
            width: cx_size,
            height: cx_size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        cross_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    let cross_handle = images.add(cross_image);

    // spawn cross UI image; we'll update its Style each frame to follow cursor
    commands.spawn(ImageBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            bottom: Val::Px(0.0),
            width: Val::Px(cx_size as f32),
            height: Val::Px(cx_size as f32),
            ..default()
        },
        image: UiImage::new(cross_handle.clone()),
        ..default()
    })
    .insert(crate::components::CursorCross);

    // North (above dial) and South (below dial) indicators
    let dial_left = 12.0_f32;
    let dial_bottom = 72.0_f32;
    let dial_size_f = dial_size as f32;
    // approximate center x for single-character labels
    let center_x = dial_left + (dial_size_f / 2.0) - 8.0;

    // North label above the dial
    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(center_x),
            bottom: Val::Px(dial_bottom + dial_size_f + 6.0),
            ..default()
        },
        text: Text::from_section(
            "N",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::rgb(0.18, 0.95, 1.0),
            },
        ),
        ..default()
    });

    // East label right of the dial
    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(dial_left + dial_size_f + 4.0),
            bottom: Val::Px(dial_bottom + (dial_size_f / 2.0) - 10.0),
            ..default()
        },
        text: Text::from_section(
            "E",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::rgb(0.18, 0.95, 1.0),
            },
        ),
        ..default()
    });

    // South label below the dial
    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(center_x),
            bottom: Val::Px(dial_bottom - 22.0),
            ..default()
        },
        text: Text::from_section(
            "S",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::rgb(0.18, 0.95, 1.0),
            },
        ),
        ..default()
    });

    // West label left of the dial
    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(dial_left - 20.0),
            bottom: Val::Px(dial_bottom + (dial_size_f / 2.0) - 10.0),
            ..default()
        },
        text: Text::from_section(
            "W",
            TextStyle {
                font: font.clone(),
                font_size: 18.0,
                color: Color::rgb(0.18, 0.95, 1.0),
            },
        ),
        ..default()
    });

    // vertical angle readout in the center of the compass
    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(dial_left + 34.0),
            bottom: Val::Px(dial_bottom + (dial_size_f / 2.0) - 18.0),
            ..default()
        },
        text: Text::from_section(
            "PITCH +0.0°",
            TextStyle {
                font: font.clone(),
                font_size: 20.0,
                color: Color::rgb(0.18, 0.95, 0.98),
            },
        ),
        ..default()
    })
    .insert(crate::components::CompassPitchText);

    commands.spawn(TextBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            bottom: Val::Px(8.0),
            ..default()
        },
        text: Text::from_section(
            "Speed: 0.0",
            TextStyle {
                font: font.clone(),
                font_size: 26.0,
                color: Color::rgb(0.18, 0.95, 0.98),
            },
        ),
        ..default()
    })
    .insert(crate::components::SpeedUi);

}
