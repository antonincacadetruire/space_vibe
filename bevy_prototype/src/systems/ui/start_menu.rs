use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window, CursorGrabMode, CursorIcon};

use crate::components::*;
use crate::resources::{ActiveScene, DeathCause, GameState, GameTimer, IdfConfig, KillCount, SceneKind, SceneLeaderboard, SpawnTransform, MouseLook, Throttle, TimePaused, PrevCameraPosition, MissileSpawnTimer, AlienSpawnTimer, ShipSkin};
use crate::setup::resolve_ui_font_path;
use crate::systems::data_loader::{CarouselState, MapCatalog, MapCatalogImages, SkinCatalog, SkinCatalogImages};
use crate::systems::ui::copilot_chat::LlmChatState;
use crate::systems::scenes::idf_transport::IDF_STATIONS;

/// Return indices of notable hub stations shown in the UI picker.
fn idf_picker_hub_indices() -> Vec<usize> {
    const HUB_NAMES: &[&str] = &[
        "Châtelet–Les Halles", "Gare du Nord", "Gare de Lyon",
        "La Défense", "Saint-Lazare", "Nation", "République",
        "Montparnasse–Bienvenüe", "Bastille", "Denfert-Rochereau",
        "Charles-de-Gaulle–Étoile", "Opéra", "Place d'Italie",
        "Trocadéro", "Invalides", "Auber / Opéra",
        "CDG Terminal 2", "Massy-Palaiseau", "Place de Clichy",
        "Strasbourg–Saint-Denis", "Bercy", "Gare d'Austerlitz",
        "Concorde", "Pigalle", "Belleville",
    ];
    let mut indices = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    for (i, s) in IDF_STATIONS.iter().enumerate() {
        if HUB_NAMES.contains(&s.1) && seen_names.insert(s.1) {
            indices.push(i);
        }
    }
    indices
}

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

#[allow(dead_code)]
fn wide_btn_style() -> Style {
    Style {
        width: Val::Px(580.0),
        height: Val::Px(80.0),
        margin: UiRect::all(Val::Px(6.0)),
        padding: UiRect::all(Val::Px(12.0)),
        justify_content: JustifyContent::FlexStart,
        align_items: AlignItems::Center,
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

// ── OnEnter(GameState::StartMenu) ─────────────────────────────────────────────
pub fn setup_start_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    leaderboard: Res<SceneLeaderboard>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    map_catalog: Res<MapCatalog>,
    skin_catalog: Res<SkinCatalog>,
    carousel_state: Res<CarouselState>,
    map_images: Res<MapCatalogImages>,
    skin_images: Res<SkinCatalogImages>,
    idf_config: Res<IdfConfig>,
) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = true;
        window.cursor.icon = CursorIcon::Arrow;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    let font = asset_server.load(resolve_ui_font_path());

    let title_style   = TextStyle { font: font.clone(), font_size: 72.0, color: hud_text_color() };
    let sub_style     = TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.45, 0.80, 0.85) };
    let section_style = TextStyle { font: font.clone(), font_size: 14.0, color: Color::rgb(0.35, 0.65, 0.68) };
    let label_style   = TextStyle { font: font.clone(), font_size: 28.0, color: hud_text_color() };
    let desc_style    = TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(0.60, 0.80, 0.82) };
    let score_style   = TextStyle { font: font.clone(), font_size: 15.0, color: Color::rgb(1.00, 0.75, 0.25) };

    let fmt_time = |v: f32| {
        let mins = (v / 60.0) as u32;
        let secs = (v % 60.0) as u32;
        let tenths = ((v % 1.0) * 10.0) as u32;
        format!("{:02}:{:02}.{}", mins, secs, tenths)
    };

    // ── Resolve initial catalog data ─────────────────────────────────────────
    let skin_idx = carousel_state.skin_idx.min(skin_catalog.skins.len().saturating_sub(1));
    let map_idx  = carousel_state.map_idx .min(map_catalog.maps.len().saturating_sub(1));

    let (skin_label, skin_desc) = skin_catalog.skins.get(skin_idx)
        .map(|s| (s.label.clone(), s.description.clone()))
        .unwrap_or_else(|| ("War Plane".into(), "".into()));

    let active_map = map_catalog.maps.get(map_idx);
    let (map_label, map_desc, map_scene) = active_map
        .map(|m| {
            let scene = match m.id.as_str() {
                "ice_caves"     => SceneKind::IceCaves,
                "desert_planet" => SceneKind::DesertPlanet,
                "idf_transport" => SceneKind::IdfTransport,
                _               => SceneKind::SpaceAsteroids,
            };
            (m.label.clone(), m.description.clone(), scene)
        })
        .unwrap_or_else(|| ("Asteroid Field".into(), "".into(), SceneKind::SpaceAsteroids));

    let scores_text = {
        let scores = leaderboard.scores(&map_scene);
        if scores.is_empty() {
            "No records yet".into()
        } else {
            scores.iter().enumerate()
                .map(|(i, &s)| format!("{}  {}", ["#1", "#2", "#3"][i], fmt_time(s)))
                .collect::<Vec<_>>()
                .join("   ")
        }
    };

    let skin_img_handle = skin_images.handles.get(skin_idx)
        .cloned()
        .unwrap_or_default();
    let map_img_handle = map_images.handles.get(map_idx)
        .cloned()
        .unwrap_or_default();

    // ── Root overlay ──────────────────────────────────────────────────────────
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.78).into(),
                ..default()
            },
            StartMenuRoot,
        ))
        .with_children(|root| {
            // ── Main panel ────────────────────────────────────────────────────
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(880.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(14.0),
                    padding: UiRect::all(Val::Px(36.0)),
                    ..default()
                },
                background_color: panel_background().into(),
                ..default()
            })
            .with_children(|panel| {

                // ── Title ─────────────────────────────────────────────────────
                panel.spawn(TextBundle::from_section("SPACE VIBE", title_style.clone()));
                panel.spawn(TextBundle::from_section("Choose your mission", sub_style.clone()));

                // Divider
                panel.spawn(NodeBundle {
                    style: Style { width: Val::Px(800.0), height: Val::Px(1.0), margin: UiRect::vertical(Val::Px(6.0)), ..default() },
                    background_color: Color::rgba(0.18, 0.95, 0.98, 0.15).into(),
                    ..default()
                });

                // ── SKIN section label ────────────────────────────────────────
                panel.spawn(NodeBundle {
                    style: Style { width: Val::Px(800.0), justify_content: JustifyContent::FlexStart, ..default() },
                    ..default()
                }).with_children(|r| {
                    r.spawn(TextBundle::from_section("SKIN", section_style.clone()));
                });

                // Skin card: [preview img | label + description]
                panel.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(800.0),
                        height: Val::Px(148.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(20.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    background_color: Color::rgba(0.04, 0.10, 0.12, 0.7).into(),
                    ..default()
                })
                .with_children(|card| {
                    // Preview image
                    card.spawn((
                        ImageBundle {
                            style: Style { width: Val::Px(128.0), height: Val::Px(128.0), flex_shrink: 0.0, ..default() },
                            image: UiImage::new(skin_img_handle),
                            ..default()
                        },
                        SkinPreviewImage,
                    ));
                    // Text column
                    card.spawn(NodeBundle {
                        style: Style { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() },
                        ..default()
                    }).with_children(|col| {
                        col.spawn(TextBundle::from_section(skin_label.clone(), label_style.clone())).insert(SkinLabel);
                        col.spawn(TextBundle::from_section(skin_desc,  desc_style.clone())).insert(SkinDescLabel);
                    });
                });

                // Skin carousel controls
                spawn_carousel_row(panel, &font, SkinLeftButton, SkinRightButton);

                // Divider
                panel.spawn(NodeBundle {
                    style: Style { width: Val::Px(800.0), height: Val::Px(1.0), margin: UiRect::vertical(Val::Px(6.0)), ..default() },
                    background_color: Color::rgba(0.18, 0.95, 0.98, 0.15).into(),
                    ..default()
                });

                // ── MAP section label ─────────────────────────────────────────
                panel.spawn(NodeBundle {
                    style: Style { width: Val::Px(800.0), justify_content: JustifyContent::FlexStart, ..default() },
                    ..default()
                }).with_children(|r| {
                    r.spawn(TextBundle::from_section("MAP", section_style.clone()));
                });

                // Map card: [preview img | label + description + scores]
                panel.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(800.0),
                        height: Val::Px(148.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(20.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    background_color: Color::rgba(0.04, 0.10, 0.12, 0.7).into(),
                    ..default()
                })
                .with_children(|card| {
                    // Preview image
                    card.spawn((
                        ImageBundle {
                            style: Style { width: Val::Px(128.0), height: Val::Px(128.0), flex_shrink: 0.0, ..default() },
                            image: UiImage::new(map_img_handle),
                            ..default()
                        },
                        MapPreviewImage,
                    ));
                    // Text column
                    card.spawn(NodeBundle {
                        style: Style { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..default() },
                        ..default()
                    }).with_children(|col| {
                        col.spawn(TextBundle::from_section(map_label.clone(), label_style.clone())).insert(MapLabel);
                        col.spawn(TextBundle::from_section(map_desc,   desc_style.clone())).insert(MapDescLabel);
                        col.spawn(TextBundle::from_section(scores_text, score_style.clone())).insert(MapScoresLabel);
                    });
                });

                // Map carousel controls
                spawn_carousel_row(panel, &font, MapLeftButton, MapRightButton);

                // Divider
                panel.spawn(NodeBundle {
                    style: Style { width: Val::Px(800.0), height: Val::Px(1.0), margin: UiRect::vertical(Val::Px(8.0)), ..default() },
                    background_color: Color::rgba(0.18, 0.95, 0.98, 0.15).into(),
                    ..default()
                });

                // ── IDF Station Picker (visible only when IDF map selected) ──
                let show_picker = matches!(map_scene, SceneKind::IdfTransport);
                panel.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(800.0),
                            flex_direction: FlexDirection::Column,
                            display: if show_picker { Display::Flex } else { Display::None },
                            ..default()
                        },
                        ..default()
                    },
                    IdfStationPickerRoot,
                ))
                .with_children(|picker| {
                    // ── Collapsible header button ────────────────────────────
                    picker.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(800.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                ..default()
                            },
                            background_color: Color::rgb(0.04, 0.14, 0.16).into(),
                            ..default()
                        },
                        IdfPickerHeaderBtn,
                    ))
                    .with_children(|hdr| {
                        hdr.spawn((
                            TextBundle::from_section(
                                "▼  Select stations to track  (click to collapse)",
                                TextStyle { font: font.clone(), font_size: 14.0, color: Color::rgb(0.40, 0.75, 0.80) },
                            ),
                            IdfPickerHeaderText,
                        ));
                    });

                    // ── Scrollable content area ─────────────────────────────
                    picker.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(800.0),
                                max_height: Val::Px(180.0),
                                flex_direction: FlexDirection::Column,
                                overflow: Overflow::clip_y(),
                                ..default()
                            },
                            background_color: Color::rgba(0.02, 0.06, 0.10, 0.8).into(),
                            ..default()
                        },
                        IdfPickerScrollContent,
                    ))
                    .with_children(|scroll_area| {
                        // Inner content node — its `top` is adjusted by scroll system
                        scroll_area.spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            ..default()
                        }).with_children(|inner| {
                            let hub_stations = idf_picker_hub_indices();
                            let all_selected = idf_config.selected_stations.is_empty();
                            for &idx in &hub_stations {
                                if let Some(s) = IDF_STATIONS.get(idx) {
                                    let is_on = all_selected || idf_config.selected_stations.contains(&idx);
                                    let label_text = format!("{} {}", if is_on { "●" } else { "○" }, s.1);
                                    inner.spawn((
                                        ButtonBundle {
                                            style: Style {
                                                width: Val::Px(780.0),
                                                min_height: Val::Px(28.0),
                                                justify_content: JustifyContent::FlexStart,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::horizontal(Val::Px(8.0)),
                                                flex_shrink: 0.0,
                                                ..default()
                                            },
                                            background_color: if is_on {
                                                Color::rgb(0.06, 0.20, 0.22)
                                            } else {
                                                Color::rgb(0.03, 0.08, 0.10)
                                            }.into(),
                                            ..default()
                                        },
                                        IdfStationToggleBtn { station_idx: idx },
                                    ))
                                    .with_children(|b| {
                                        b.spawn((
                                            TextBundle::from_section(
                                                label_text,
                                                TextStyle { font: font.clone(), font_size: 13.0, color: Color::rgb(0.70, 0.90, 0.92) },
                                            ),
                                            IdfStationToggleText { station_idx: idx },
                                        ));
                                    });
                                }
                            }
                        });
                    });
                });

                // ── Action buttons ───────────────────────────────────────────
                panel.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(24.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    // PLAY
                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(280.0),
                                height: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: btn_normal().into(),
                            ..default()
                        },
                        PlayButton,
                    ))
                    .with_children(|b| {
                        b.spawn(TextBundle::from_section(
                            "PLAY",
                            TextStyle { font: font.clone(), font_size: 34.0, color: hud_text_color() },
                        ));
                    });

                    // COPILOT CHAT
                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(240.0),
                                height: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: btn_normal().into(),
                            ..default()
                        },
                        CopilotMenuButton,
                    ))
                    .with_children(|b| {
                        b.spawn(TextBundle::from_section(
                            "AI Chat [F2]",
                            TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.20, 0.85, 0.95) },
                        ));
                    });

                    // EXIT
                    row.spawn((
                        ButtonBundle {
                            style: btn_style(),
                            background_color: btn_normal().into(),
                            ..default()
                        },
                        QuitButton,
                    ))
                    .with_children(|b| {
                        b.spawn(TextBundle::from_section(
                            "Exit",
                            TextStyle { font: font.clone(), font_size: 22.0, color: hud_text_color() },
                        ));
                    });
                });
            });
        });
}

/// Spawns a `< ... >` carousel row.  The type params let us reuse this for
/// both the skin and map carousels without a runtime flag.
fn spawn_carousel_row<L, R>(
    parent: &mut ChildBuilder,
    font: &Handle<Font>,
    _left_marker: L,
    _right_marker: R,
) where
    L: Component + Default,
    R: Component + Default,
{
    let arrow_style = TextStyle { font: font.clone(), font_size: 34.0, color: Color::rgb(0.18, 0.95, 0.98) };
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Px(800.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            ButtonBundle { style: btn_style(), background_color: btn_normal().into(), ..default() },
            L::default(),
        ))
        .with_children(|b| { b.spawn(TextBundle::from_section("<", arrow_style.clone())); });

        row.spawn((
            ButtonBundle { style: btn_style(), background_color: btn_normal().into(), ..default() },
            R::default(),
        ))
        .with_children(|b| { b.spawn(TextBundle::from_section(">", arrow_style.clone())); });
    });
}

// ── OnExit(GameState::StartMenu) ──────────────────────────────────────────────
pub fn teardown_start_menu(mut commands: Commands, q: Query<Entity, With<StartMenuRoot>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// ── OnEnter(GameState::Playing) ───────────────────────────────────────────────
pub fn enter_playing(
    mut game_timer: ResMut<GameTimer>,
    mut kill_count: ResMut<KillCount>,
    mut throttle: ResMut<Throttle>,
    mut paused: ResMut<TimePaused>,
    mut mouse_look: ResMut<MouseLook>,
    mut prev_cam: ResMut<PrevCameraPosition>,
    mut free_look: ResMut<crate::resources::FreeLook>,
    mut missile_timer: ResMut<MissileSpawnTimer>,
    mut alien_timer: ResMut<AlienSpawnTimer>,
    mut death_cause: ResMut<DeathCause>,
    mut chat: ResMut<LlmChatState>,
    mut idf_config: ResMut<IdfConfig>,
    active_scene: Res<ActiveScene>,
    spawn_transform: Res<SpawnTransform>,
    mut camera_q: Query<&mut Transform, With<MainCamera>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    game_timer.0 = 0.0;
    kill_count.0 = 0;
    throttle.0 = 0.0;
    paused.0 = false;
    *free_look = crate::resources::FreeLook::default();
    missile_timer.0.reset();
    alien_timer.0.reset();
    *death_cause = DeathCause::default();
    chat.open = false; // close chat panel when entering gameplay
    // Auto-select all stations when entering IDF map
    if let SceneKind::IdfTransport = &active_scene.0 {
        if idf_config.selected_stations.is_empty() {
            let count = crate::systems::scenes::idf_transport::IDF_STATIONS.len();
            idf_config.selected_stations = (0..count).collect();
        }
    }
    mouse_look.yaw = spawn_transform.yaw;
    mouse_look.pitch = spawn_transform.pitch;
    prev_cam.0 = spawn_transform.transform.translation;
    if let Ok(mut transform) = camera_q.get_single_mut() {
        *transform = spawn_transform.transform;
    }
    if let Ok(mut window) = windows.get_single_mut() {
        use crate::systems::core::exit::apply_game_cursor;
        apply_game_cursor(&mut window);
    }
}

// ── OnEnter(GameState::Playing) – spawn timer UI + danger HUD ────────────────
pub fn spawn_timer_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(resolve_ui_font_path());
    // Timer + kill counter (top-right)
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                ..default()
            },
            text: Text::from_sections([
                TextSection::new(
                    "00:00.0",
                    TextStyle { font: font.clone(), font_size: 30.0, color: hud_text_color() },
                ),
                TextSection::new(
                    "  Kills: 0",
                    TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.55, 1.0, 0.35) },
                ),
            ]),
            ..default()
        },
        TimerUi,
    ));
    // Missile warning text (top center, hidden until needed)
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Px(60.0),
                ..default()
            },
            text: Text::from_section(
                "",
                TextStyle { font: font.clone(), font_size: 24.0, color: Color::rgb(1.0, 0.15, 0.05) },
            ),
            visibility: Visibility::Hidden,
            ..default()
        },
        MissileWarningUi,
        TimerUi, // share despawn marker
    ));
    // Full-screen danger vignette (transparent by default)
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::rgba(0.6, 0.0, 0.0, 0.0).into(),
            z_index: ZIndex::Global(10),
            ..default()
        },
        DangerVignette,
        TimerUi, // share despawn marker
    ));
}

// ── OnExit(GameState::Playing) – despawn timer UI ────────────────────────────
pub fn despawn_timer_ui(mut commands: Commands, q: Query<Entity, With<TimerUi>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

// ── Update – tick the timer and refresh the HUD text ─────────────────────────
pub fn update_timer(
    time: Res<Time>,
    paused: Res<TimePaused>,
    kill_count: Res<KillCount>,
    mut game_timer: ResMut<GameTimer>,
    mut text_q: Query<&mut Text, With<TimerUi>>,
) {
    if !paused.0 {
        game_timer.0 += time.delta_seconds();
    }
    let t = game_timer.0;
    let mins = (t / 60.0) as u32;
    let secs = (t % 60.0) as u32;
    let tenths = ((t % 1.0) * 10.0) as u32;
    for mut text in text_q.iter_mut() {
        text.sections[0].value = format!("{:02}:{:02}.{}", mins, secs, tenths);
        if text.sections.len() > 1 {
            text.sections[1].value = format!("  Kills: {}", kill_count.0);
        }
    }
}

// ── Update – button appearance (start menu only) ─────────────────────────────
pub fn start_menu_button_appearance_system(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut bg) in q.iter_mut() {
        bg.0 = match interaction {
            Interaction::Pressed => btn_pressed(),
            Interaction::Hovered => btn_hovered(),
            Interaction::None    => btn_normal(),
        };
    }
}

// ── Update – button action (start menu only) ─────────────────────────────────
pub fn start_menu_button_system(
    mut q: Query<
        (&Interaction, Option<&QuitButton>, Option<&CopilotMenuButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut chat: ResMut<LlmChatState>,
) {
    for (interaction, quit, copilot_btn) in q.iter_mut() {
        if *interaction == Interaction::Pressed {
            // When chat is open, ignore all other button presses (blocker intercepts clicks,
            // but guard here just in case).
            if chat.open && quit.is_some() {
                continue;
            }
            if quit.is_some() {
                std::process::exit(0);
            }
            if copilot_btn.is_some() {
                // Just toggle the flag; llm_chat_toggle_system owns the Style sync.
                chat.open = !chat.open;
            }
        }
    }
}

// New: handle carousel buttons and the Play button
pub fn start_menu_carousel_system(
    mut q: Query<(
        &Interaction,
        Option<&SkinLeftButton>, Option<&SkinRightButton>,
        Option<&MapLeftButton>,  Option<&MapRightButton>,
        Option<&PlayButton>,
    ), (Changed<Interaction>, With<Button>)>,
    mut ship_skin: ResMut<ShipSkin>,
    mut active_scene: ResMut<ActiveScene>,
    mut carousel_state: ResMut<CarouselState>,
    skin_catalog: Res<SkinCatalog>,
    map_catalog: Res<MapCatalog>,
    skin_images: Res<SkinCatalogImages>,
    map_images: Res<MapCatalogImages>,
    leaderboard: Res<SceneLeaderboard>,
    mut text_q: ParamSet<(
        Query<&mut Text, With<SkinLabel>>,
        Query<&mut Text, With<SkinDescLabel>>,
        Query<&mut Text, With<MapLabel>>,
        Query<&mut Text, With<MapDescLabel>>,
        Query<&mut Text, With<MapScoresLabel>>,
    )>,
    mut img_q: ParamSet<(
        Query<&mut UiImage, With<SkinPreviewImage>>,
        Query<&mut UiImage, With<MapPreviewImage>>,
    )>,
    mut next_state: ResMut<NextState<GameState>>,
    mut picker_q: Query<&mut Style, With<IdfStationPickerRoot>>,
) {
    let skin_count = skin_catalog.skins.len().max(1);
    let map_count  = map_catalog.maps.len().max(1);

    let fmt_time = |v: f32| {
        let mins = (v / 60.0) as u32;
        let secs = (v % 60.0) as u32;
        let tenths = ((v % 1.0) * 10.0) as u32;
        format!("{:02}:{:02}.{}", mins, secs, tenths)
    };

    let mut skin_changed = false;
    let mut map_changed  = false;

    for (interaction, skin_l, skin_r, map_l, map_r, play) in q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }

        if skin_l.is_some() {
            carousel_state.skin_idx = (carousel_state.skin_idx + skin_count - 1) % skin_count;
            skin_changed = true;
        }
        if skin_r.is_some() {
            carousel_state.skin_idx = (carousel_state.skin_idx + 1) % skin_count;
            skin_changed = true;
        }
        if map_l.is_some() {
            carousel_state.map_idx = (carousel_state.map_idx + map_count - 1) % map_count;
            map_changed = true;
        }
        if map_r.is_some() {
            carousel_state.map_idx = (carousel_state.map_idx + 1) % map_count;
            map_changed = true;
        }
        if play.is_some() {
            next_state.set(GameState::Playing);
        }
    }

    if skin_changed {
        let idx = carousel_state.skin_idx;
        *ship_skin = ShipSkin(
            skin_catalog.skins.get(idx)
                .map(|s| s.id.clone())
                .unwrap_or_else(|| "war_plane".to_owned()),
        );
        if let Some(s) = skin_catalog.skins.get(idx) {
            let label = s.label.clone();
            let desc  = s.description.clone();
            for mut t in text_q.p0().iter_mut() { t.sections[0].value = label.clone(); }
            for mut t in text_q.p1().iter_mut() { t.sections[0].value = desc.clone(); }
        }
        if let Some(handle) = skin_images.handles.get(idx) {
            let h = handle.clone();
            for mut img in img_q.p0().iter_mut() { img.texture = h.clone(); }
        }
    }

    if map_changed {
        let idx = carousel_state.map_idx;
        if let Some(m) = map_catalog.maps.get(idx) {
            let scene = match m.id.as_str() {
                "ice_caves"     => SceneKind::IceCaves,
                "desert_planet" => SceneKind::DesertPlanet,
                "idf_transport" => SceneKind::IdfTransport,
                _               => SceneKind::SpaceAsteroids,
            };
            active_scene.0 = scene.clone();

            let map_label = m.label.clone();
            let map_desc  = m.description.clone();
            for mut t in text_q.p2().iter_mut() { t.sections[0].value = map_label.clone(); }
            for mut t in text_q.p3().iter_mut() { t.sections[0].value = map_desc.clone(); }

            let scores = leaderboard.scores(&scene);
            let scores_text = if scores.is_empty() {
                "No records yet".into()
            } else {
                scores.iter().enumerate()
                    .map(|(i, &s)| format!("{}  {}", ["#1", "#2", "#3"][i], fmt_time(s)))
                    .collect::<Vec<_>>()
                    .join("   ")
            };
            for mut t in text_q.p4().iter_mut() { t.sections[0].value = scores_text.clone(); }
        }
        if let Some(handle) = map_images.handles.get(idx) {
            let h = handle.clone();
            for mut img in img_q.p1().iter_mut() { img.texture = h.clone(); }
        }

        // Show/hide IDF station picker
        let is_idf = active_scene.0 == SceneKind::IdfTransport;
        for mut style in picker_q.iter_mut() {
            style.display = if is_idf { Display::Flex } else { Display::None };
        }
    }
}

// ── Update – live carousel refresh when catalog reloaded by AI chat ───────────
pub fn catalog_refresh_system(
    map_catalog: Res<MapCatalog>,
    skin_catalog: Res<SkinCatalog>,
    map_images: Res<MapCatalogImages>,
    skin_images: Res<SkinCatalogImages>,
    carousel_state: Res<CarouselState>,
    leaderboard: Res<SceneLeaderboard>,
    mut active_scene: ResMut<ActiveScene>,
    mut text_q: ParamSet<(
        Query<&mut Text, With<SkinLabel>>,
        Query<&mut Text, With<SkinDescLabel>>,
        Query<&mut Text, With<MapLabel>>,
        Query<&mut Text, With<MapDescLabel>>,
        Query<&mut Text, With<MapScoresLabel>>,
    )>,
    mut img_q: ParamSet<(
        Query<&mut UiImage, With<SkinPreviewImage>>,
        Query<&mut UiImage, With<MapPreviewImage>>,
    )>,
) {
    let fmt_time = |v: f32| {
        let mins = (v / 60.0) as u32;
        let secs = (v % 60.0) as u32;
        let tenths = ((v % 1.0) * 10.0) as u32;
        format!("{:02}:{:02}.{}", mins, secs, tenths)
    };

    if map_catalog.is_changed() {
        let idx = carousel_state.map_idx.min(map_catalog.maps.len().saturating_sub(1));
        if let Some(m) = map_catalog.maps.get(idx) {
            let scene = match m.id.as_str() {
                "ice_caves"     => SceneKind::IceCaves,
                "desert_planet" => SceneKind::DesertPlanet,
                "idf_transport" => SceneKind::IdfTransport,
                _               => SceneKind::SpaceAsteroids,
            };
            active_scene.0 = scene.clone();
            let label = m.label.clone();
            let desc  = m.description.clone();
            for mut t in text_q.p2().iter_mut() { t.sections[0].value = label.clone(); }
            for mut t in text_q.p3().iter_mut() { t.sections[0].value = desc.clone(); }
            let scores = leaderboard.scores(&scene);
            let scores_text = if scores.is_empty() {
                "No records yet".into()
            } else {
                scores.iter().enumerate()
                    .map(|(i, &s)| format!("{}  {}", ["#1", "#2", "#3"][i], fmt_time(s)))
                    .collect::<Vec<_>>()
                    .join("   ")
            };
            for mut t in text_q.p4().iter_mut() { t.sections[0].value = scores_text.clone(); }
        }
        let h_idx = carousel_state.map_idx.min(map_images.handles.len().saturating_sub(1));
        if let Some(handle) = map_images.handles.get(h_idx) {
            let h = handle.clone();
            for mut img in img_q.p1().iter_mut() { img.texture = h.clone(); }
        }
    }

    if skin_catalog.is_changed() {
        let idx = carousel_state.skin_idx.min(skin_catalog.skins.len().saturating_sub(1));
        if let Some(s) = skin_catalog.skins.get(idx) {
            let label = s.label.clone();
            let desc  = s.description.clone();
            for mut t in text_q.p0().iter_mut() { t.sections[0].value = label.clone(); }
            for mut t in text_q.p1().iter_mut() { t.sections[0].value = desc.clone(); }
        }
        let h_idx = carousel_state.skin_idx.min(skin_images.handles.len().saturating_sub(1));
        if let Some(handle) = skin_images.handles.get(h_idx) {
            let h = handle.clone();
            for mut img in img_q.p0().iter_mut() { img.texture = h.clone(); }
        }
    }
}

// ── Update – danger vignette + missile warning ────────────────────────────────

/// Station picker toggle buttons
pub fn idf_station_toggle_system(
    interaction_q: Query<(&Interaction, &IdfStationToggleBtn), (Changed<Interaction>, With<Button>)>,
    mut btn_q: Query<(&IdfStationToggleBtn, &mut BackgroundColor)>,
    mut text_q: Query<(&IdfStationToggleText, &mut Text)>,
    mut idf_config: ResMut<IdfConfig>,
) {
    let mut changed = false;
    for (interaction, btn) in interaction_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        let idx = btn.station_idx;
        if let Some(pos) = idf_config.selected_stations.iter().position(|&i| i == idx) {
            idf_config.selected_stations.remove(pos);
        } else {
            idf_config.selected_stations.push(idx);
        }
        changed = true;
    }
    if !changed { return; }
    // Refresh all button visuals
    let selected = &idf_config.selected_stations;
    let all_on = selected.is_empty();
    for (btn, mut bg) in btn_q.iter_mut() {
        let is_on = all_on || selected.contains(&btn.station_idx);
        bg.0 = if is_on { Color::rgb(0.06, 0.20, 0.22) } else { Color::rgb(0.03, 0.08, 0.10) };
    }
    for (txt_comp, mut text) in text_q.iter_mut() {
        let is_on = all_on || selected.contains(&txt_comp.station_idx);
        if let Some(s) = IDF_STATIONS.get(txt_comp.station_idx) {
            text.sections[0].value = format!("{} {}", if is_on { "●" } else { "○" }, s.1);
        }
    }
}

/// Collapse / expand the station picker when the header button is clicked.
pub fn idf_picker_collapse_system(
    header_q: Query<&Interaction, (Changed<Interaction>, With<IdfPickerHeaderBtn>)>,
    mut content_q: Query<&mut Style, With<IdfPickerScrollContent>>,
    mut text_q: Query<&mut Text, With<IdfPickerHeaderText>>,
) {
    for interaction in header_q.iter() {
        if *interaction != Interaction::Pressed { continue; }
        for mut style in content_q.iter_mut() {
            let is_visible = style.display != Display::None;
            style.display = if is_visible { Display::None } else { Display::Flex };
            // Update header arrow
            for mut txt in text_q.iter_mut() {
                txt.sections[0].value = if is_visible {
                    "▶  Select stations to track  (click to expand)".into()
                } else {
                    "▼  Select stations to track  (click to collapse)".into()
                };
            }
        }
    }
}

/// Mouse-wheel scrolling for the station picker content.
pub fn idf_picker_scroll_system(
    mut scroll_evts: EventReader<bevy::input::mouse::MouseWheel>,
    scroll_content_q: Query<(&Node, &Children), With<IdfPickerScrollContent>>,
    mut inner_q: Query<(&Node, &mut Style), Without<IdfPickerScrollContent>>,
) {
    let mut delta_px: f32 = 0.0;
    for ev in scroll_evts.iter() {
        delta_px += match ev.unit {
            bevy::input::mouse::MouseScrollUnit::Line => ev.y * 28.0,
            bevy::input::mouse::MouseScrollUnit::Pixel => ev.y,
        };
    }
    if delta_px.abs() < 0.1 { return; }

    for (container_node, children) in scroll_content_q.iter() {
        let container_h = container_node.size().y;
        for &child in children.iter() {
            if let Ok((inner_node, mut inner_style)) = inner_q.get_mut(child) {
                let content_h = inner_node.size().y;
                let max_scroll = (content_h - container_h).max(0.0);
                let current = match inner_style.top {
                    Val::Px(v) => v,
                    _ => 0.0,
                };
                // delta_px > 0 means scroll-up → move content down (less negative top)
                let new_top = (current + delta_px).clamp(-max_scroll, 0.0);
                inner_style.top = Val::Px(new_top);
            }
        }
    }
}

pub fn danger_hud_system(
    time: Res<Time>,
    missiles: Query<&Transform, With<crate::components::Missile>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    mut warning_q: Query<(&mut Text, &mut Visibility), With<MissileWarningUi>>,
    mut vignette_q: Query<&mut BackgroundColor, With<DangerVignette>>,
    mut timer_text_q: Query<&mut Text, (With<TimerUi>, Without<MissileWarningUi>)>,
) {
    let Ok(cam) = camera_q.get_single() else { return };

    // Find closest missile distance
    let closest_dist = missiles.iter()
        .map(|t| t.translation.distance(cam.translation))
        .fold(f32::MAX, f32::min);

    const WARN_DIST: f32 = 3_000.0;
    const CRIT_DIST: f32 = 800.0;

    // Vignette alpha
    if let Ok(mut bg) = vignette_q.get_single_mut() {
        let alpha = if closest_dist < WARN_DIST {
            let t = 1.0 - (closest_dist / WARN_DIST).clamp(0.0, 1.0);
            let pulse = if closest_dist < CRIT_DIST {
                ((time.elapsed_seconds() * 6.0).sin() * 0.5 + 0.5) * 0.35
            } else { 0.0 };
            t * 0.28 + pulse
        } else { 0.0 };
        bg.0 = Color::rgba(0.6, 0.0, 0.0, alpha);
    }

    // Warning text
    if let Ok((mut text, mut vis)) = warning_q.get_single_mut() {
        if closest_dist < WARN_DIST {
            *vis = Visibility::Visible;
            text.sections[0].value = format!("!! MISSILE  {:.0} m", closest_dist);
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Timer text color
    if let Ok(mut text) = timer_text_q.get_single_mut() {
        text.sections[0].style.color = if closest_dist < CRIT_DIST {
            let p = (time.elapsed_seconds() * 6.0).sin() * 0.5 + 0.5;
            Color::rgb(1.0, p * 0.2, p * 0.05)
        } else {
            Color::rgb(0.18, 0.95, 0.98)
        };
    }
}
