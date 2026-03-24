use bevy::prelude::*;
use crate::components::{AlienShip, MainCamera, TimerUi};

/// Marker for the minimap root panel.
#[derive(Component)]
pub struct MinimapRoot;

/// Each enemy gets a blip child of MinimapRoot.
#[derive(Component)]
pub struct MinimapBlip {
    /// The alien entity this blip tracks (used to remove stale blips).
    pub tracked: Entity,
}

/// World-units that map to the minimap radius.
const MINIMAP_SCALE: f32 = 60_000.0;
/// Pixel radius of the minimap circle.
const MAP_RADIUS: f32 = 72.0;
/// Total panel size (circle fits inside with a 4 px border).
const PANEL_SIZE: f32 = (MAP_RADIUS + 4.0) * 2.0;
/// Blip dot size.
const BLIP_SIZE: f32 = 8.0;

// ── Spawn ─────────────────────────────────────────────────────────────────────
pub fn spawn_minimap_ui(mut commands: Commands) {
    // Outer positioning wrapper – anchored bottom-right
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    bottom: Val::Px(16.0),
                    width: Val::Px(PANEL_SIZE),
                    height: Val::Px(PANEL_SIZE),
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.55).into(),
                ..default()
            },
            MinimapRoot,
            TimerUi, // despawned together with the rest of the HUD
        ))
        // Player dot (white, always centred)
        .with_children(|parent| {
            parent.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(PANEL_SIZE / 2.0 - BLIP_SIZE / 2.0),
                    top: Val::Px(PANEL_SIZE / 2.0 - BLIP_SIZE / 2.0),
                    width: Val::Px(BLIP_SIZE),
                    height: Val::Px(BLIP_SIZE),
                    ..default()
                },
                background_color: Color::WHITE.into(),
                ..default()
            });
        });
}

// ── Despawn ───────────────────────────────────────────────────────────────────
pub fn despawn_minimap_ui(
    mut commands: Commands,
    q: Query<Entity, With<MinimapRoot>>,
) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// ── Update ────────────────────────────────────────────────────────────────────
pub fn update_minimap_system(
    camera_q: Query<&Transform, With<MainCamera>>,
    aliens_q: Query<(Entity, &Transform), With<AlienShip>>,
    mut blips_q: Query<(Entity, &MinimapBlip, &mut Style)>,
    minimap_q: Query<Entity, With<MinimapRoot>>,
    mut commands: Commands,
) {
    let Ok(cam_tf) = camera_q.get_single() else { return };
    let Ok(minimap_entity) = minimap_q.get_single() else { return };

    let cam_pos = cam_tf.translation;
    // Forward direction projected onto XZ plane (used to rotate the minimap
    // so "up" is always where the player is looking).
    let forward_xz = {
        let f = cam_tf.forward();
        Vec2::new(f.x, -f.z).normalize_or_zero()
    };
    // We rotate blip positions by the camera yaw so the map is camera-relative.
    let cos_a = forward_xz.y; // camera looks down -Z by default
    let sin_a = forward_xz.x;

    // --- Remove blips whose alien is gone ---
    let alive: Vec<Entity> = aliens_q.iter().map(|(e, _)| e).collect();
    let stale: Vec<Entity> = blips_q
        .iter()
        .filter_map(|(e, blip, _)| if !alive.contains(&blip.tracked) { Some(e) } else { None })
        .collect();
    for blip_entity in &stale {
        // Must remove from parent's Children before despawning, otherwise the UI
        // clipping system panics on the dangling child reference.
        commands.entity(minimap_entity).remove_children(&[*blip_entity]);
        commands.entity(*blip_entity).despawn();
    }

    // --- Existing alien entities: find or create blip ---
    let existing_blips: Vec<Entity> = blips_q.iter().map(|(_, b, _)| b.tracked).collect();

    for (alien_entity, alien_tf) in aliens_q.iter() {
        let delta = alien_tf.translation - cam_pos;
        // World XZ → minimap UV (camera-relative)
        let dx = delta.x;
        let dz = -delta.z; // Bevy's -Z is forward
        // Rotate by camera yaw
        let rx = dx * cos_a + dz * sin_a;
        let ry = -dx * sin_a + dz * cos_a;

        // Normalise to [-1, 1] and clamp to circle edge
        let mut nx = rx / MINIMAP_SCALE;
        let mut ny = ry / MINIMAP_SCALE;
        let mag = (nx * nx + ny * ny).sqrt();
        if mag > 1.0 {
            nx /= mag;
            ny /= mag;
        }

        // Convert to pixel offsets from panel centre
        let px = PANEL_SIZE / 2.0 + nx * MAP_RADIUS - BLIP_SIZE / 2.0;
        let py = PANEL_SIZE / 2.0 - ny * MAP_RADIUS - BLIP_SIZE / 2.0;

        if existing_blips.contains(&alien_entity) {
            // Update position
            for (_, blip, mut style) in blips_q.iter_mut() {
                if blip.tracked == alien_entity {
                    style.left = Val::Px(px);
                    style.top = Val::Px(py);
                }
            }
        } else {
            // Spawn new blip as child of minimap
            let blip_entity = commands
                .spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(px),
                            top: Val::Px(py),
                            width: Val::Px(BLIP_SIZE),
                            height: Val::Px(BLIP_SIZE),
                            ..default()
                        },
                        background_color: Color::rgb(1.0, 0.2, 0.2).into(),
                        ..default()
                    },
                    MinimapBlip { tracked: alien_entity },
                ))
                .id();
            commands.entity(minimap_entity).add_child(blip_entity);
        }
    }
}
