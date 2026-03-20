use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, CursorIcon, CursorGrabMode};

use crate::systems::spawner::spawn_asteroid;

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 200.0, 600.0),
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: PerspectiveProjection {
                far: 10000.0,
                ..default()
            }
            .into(),
            ..default()
        },
        crate::components::MainCamera,
    ));

    // Hide the OS cursor so only our UI crosshair indicates direction
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = false;
        window.cursor.icon = CursorIcon::Crosshair;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }

    // (OS cursor hidden) — we draw an on-screen UI cross to indicate pointer

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        ..default()
    });

    let mut rng = rand::thread_rng();
    for _ in 0..6 {
        spawn_asteroid(&mut commands, &mut rng, &mut meshes, &mut materials);
    }

    // UI: speed and compass (bottom-left)
    // Prefer a bundled font in `assets/fonts/`, otherwise try common system fonts.
    use std::path::Path;
    let font_path = if Path::new("assets/fonts/FiraSans-Bold.ttf").exists() {
        "fonts/FiraSans-Bold.ttf"
    } else if Path::new("C:\\Windows\\Fonts\\arial.ttf").exists() {
        "C:\\Windows\\Fonts\\arial.ttf"
    } else if Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf").exists() {
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
    } else {
        warn!("No local or system font found; text UI may not render until a font is added to assets/fonts/");
        "fonts/FiraSans-Bold.ttf"
    };

    let font = asset_server.load(font_path);

    // Procedurally generate a simple green dial and a red needle for compass UI
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let dial_size: u32 = 128;
    let mut dial_data = vec![0u8; (dial_size * dial_size * 4) as usize];
    let cx = (dial_size / 2) as i32;
    let cy = (dial_size / 2) as i32;
    let radius = (dial_size as i32) / 2 - 2;
    for y in 0..(dial_size as i32) {
        for x in 0..(dial_size as i32) {
            let dx = x - cx;
            let dy = y - cy;
            let dist2 = dx * dx + dy * dy;
            let idx = ((y * dial_size as i32 + x) * 4) as usize;
            if dist2 <= radius * radius {
                // green dial background
                dial_data[idx] = 30; // r
                dial_data[idx + 1] = 180; // g
                dial_data[idx + 2] = 30; // b
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
                    dial_data[idx] = 255;
                    dial_data[idx+1] = 255;
                    dial_data[idx+2] = 255;
                    dial_data[idx+3] = 255;
                }
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

    // needle: tall thin red triangle
    let n_w: u32 = 16;
    let n_h: u32 = 96;
    let mut needle_data = vec![0u8; (n_w * n_h * 4) as usize];
    let mid = (n_w / 2) as i32;
    for y in 0..(n_h as i32) {
        // triangle width decreases towards tip
        let t = 1.0 - (y as f32 / n_h as f32);
        let half = ((mid as f32) * t) as i32;
        for x in 0..(n_w as i32) {
            let idx = ((y * n_w as i32 + x) * 4) as usize;
            if (x - mid).abs() <= half {
                // red
                needle_data[idx] = 200;
                needle_data[idx + 1] = 30;
                needle_data[idx + 2] = 30;
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
            left: Val::Px(8.0),
            bottom: Val::Px(80.0),
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
            left: Val::Px(8.0 + (dial_size as f32 - n_w as f32) / 2.0),
            bottom: Val::Px(80.0 + (dial_size as f32 - n_h as f32) / 2.0),
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
    let dial_left = 8.0_f32;
    let dial_bottom = 80.0_f32;
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
                color: Color::WHITE,
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
                color: Color::WHITE,
            },
        ),
        ..default()
    });

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
                color: Color::GREEN,
            },
        ),
        ..default()
    })
    .insert(crate::components::SpeedUi);

    // previously there was a CompassUi text here; removed per UI simplification
}
