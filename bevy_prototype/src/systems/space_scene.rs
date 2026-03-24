use bevy::core_pipeline::bloom::BloomSettings;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::Rng;
use std::f32::consts::TAU;

use crate::components::{AngularVelocity, Asteroid, BeltAsteroid, MainCamera, Radius, Saturn, SkyDome, Velocity};
use crate::resources::{PrevCameraPosition, RingLodUpdateTimer, TimePaused, GameState, GameTimer};

const SCENE_SCALE: f32 = 100.0;
const SATURN_RADIUS: f32 = 260.0 * SCENE_SCALE;
const BACKDROP_RADIUS: f32 = 12_000.0 * SCENE_SCALE;
const RING_ASTEROID_COUNT: usize = 1_080;
const RING_INNER_RADIUS: f32 = SATURN_RADIUS * 1.55;
const RING_OUTER_RADIUS: f32 = SATURN_RADIUS * 2.55;
const LOD_HIGH_DISTANCE: f32 = 12_000.0;
const LOD_MID_DISTANCE: f32 = 30_000.0;
const ASTEROID_LOD_VARIANTS: usize = 4;

#[derive(Component, Copy, Clone, PartialEq, Eq)]
enum AsteroidLod {
    High,
    Mid,
    Low,
}

#[derive(Component)]
pub struct RingAsteroid {
    orbit_radius: f32,
    orbit_angle: f32,
    orbit_speed: f32,
    vertical_offset: f32,
    shape_index: usize,
    _base_radius: f32,
}

#[derive(Resource)]
pub struct RingMeshLibrary {
    high: Vec<Handle<Mesh>>,
    mid: Vec<Handle<Mesh>>,
    low: Vec<Handle<Mesh>>,
}

fn make_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
    )
}

fn paint_blob(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    center_x: f32,
    center_y: f32,
    radius: f32,
    color: [u8; 4],
) {
    let min_x = (center_x - radius).floor().max(0.0) as i32;
    let max_x = (center_x + radius).ceil().min(width as f32 - 1.0) as i32;
    let min_y = (center_y - radius).floor().max(0.0) as i32;
    let max_y = (center_y + radius).ceil().min(height as f32 - 1.0) as i32;

    let radius_sq = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > radius_sq {
                continue;
            }

            let dist = dist_sq.sqrt();
            let falloff = (1.0 - dist / radius).clamp(0.0, 1.0);
            let intensity = falloff * falloff * color[3] as f32 / 255.0;
            let idx = ((y as u32 * width + x as u32) * 4) as usize;

            pixels[idx] = (pixels[idx] as f32 + color[0] as f32 * intensity).min(255.0) as u8;
            pixels[idx + 1] = (pixels[idx + 1] as f32 + color[1] as f32 * intensity).min(255.0) as u8;
            pixels[idx + 2] = (pixels[idx + 2] as f32 + color[2] as f32 * intensity).min(255.0) as u8;
            pixels[idx + 3] = (pixels[idx + 3] as f32 + color[3] as f32 * intensity).min(255.0) as u8;
        }
    }
}

fn make_starfield_texture(rng: &mut impl Rng) -> Image {
    let width = 1024;
    let height = 512;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        let vertical = y as f32 / height as f32;
        let base_r = (2.0 + vertical * 3.0) as u8;
        let base_g = (5.0 + vertical * 7.0) as u8;
        let base_b = (12.0 + vertical * 16.0) as u8;
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = base_r;
            pixels[idx + 1] = base_g;
            pixels[idx + 2] = base_b;
            pixels[idx + 3] = 255;
        }
    }

    for _ in 0..42 {
        let cx = rng.gen_range(0.0..width as f32);
        let cy = rng.gen_range(0.0..height as f32);
        let radius = rng.gen_range(30.0..130.0);
        let tint = if rng.gen_bool(0.5) {
            [46, 26, 96, 34]
        } else {
            [18, 64, 90, 28]
        };
        paint_blob(&mut pixels, width, height, cx, cy, radius, tint);
    }

    for _ in 0..500 {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);
        let idx = ((y * width + x) * 4) as usize;
        let brightness = rng.gen_range(200..255) as u8;
        pixels[idx] = brightness;
        pixels[idx + 1] = brightness;
        pixels[idx + 2] = brightness.saturating_add(rng.gen_range(0..30));
        pixels[idx + 3] = 255;

        // ~30% of stars get a 1-pixel halo to look like a point of light
        if rng.gen_bool(0.3) && x + 1 < width {
            let neighbor = idx + 4;
            pixels[neighbor] = brightness / 2;
            pixels[neighbor + 1] = brightness / 2;
            pixels[neighbor + 2] = brightness;
            pixels[neighbor + 3] = 255;
        }
    }

    make_image(width, height, pixels)
}

fn make_nebula_texture(rng: &mut impl Rng) -> Image {
    let width = 1024;
    let height = 512;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for _ in 0..34 {
        let cx = rng.gen_range(0.0..width as f32);
        let cy = rng.gen_range(0.0..height as f32);
        let radius = rng.gen_range(70.0..220.0);
        let color = if rng.gen_bool(0.5) {
            [105, 72, 220, 42]
        } else {
            [34, 146, 220, 32]
        };
        paint_blob(&mut pixels, width, height, cx, cy, radius, color);
    }

    make_image(width, height, pixels)
}

fn make_saturn_texture() -> Image {
    let width = 1024;
    let height = 512;
    let mut pixels = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        let latitude = y as f32 / height as f32;
        let equator_boost = 1.0 - ((latitude - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        let polar_falloff = (1.0 - equator_boost).powf(1.5);
        for x in 0..width {
            let longitude = x as f32 / width as f32;
            let band_a = (latitude * TAU * 16.0 + longitude * TAU * 4.0).sin();
            let band_b = (latitude * TAU * 8.0 - longitude * TAU * 3.0).cos();
            let storm = ((longitude * TAU * 2.0).sin() * (latitude * TAU * 11.0).cos()).abs();

            let mut r = 176.0 + band_a * 16.0 + band_b * 10.0 + storm * 20.0;
            let mut g = 150.0 + band_a * 13.0 + band_b * 8.0 + storm * 14.0;
            let mut b = 112.0 + band_a * 9.0 + band_b * 6.0 + storm * 10.0;

            r -= polar_falloff * 42.0;
            g -= polar_falloff * 34.0;
            b -= polar_falloff * 24.0;

            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = r.clamp(0.0, 255.0) as u8;
            pixels[idx + 1] = g.clamp(0.0, 255.0) as u8;
            pixels[idx + 2] = b.clamp(0.0, 255.0) as u8;
            pixels[idx + 3] = 255;
        }
    }

    make_image(width, height, pixels)
}

fn build_asteroid_shape_mesh(radius: f32, detail: usize, rng: &mut impl Rng) -> Mesh {
    let mut mesh = Mesh::from(shape::UVSphere {
        radius: 1.0,
        sectors: detail * 2 + 4,
        stacks: detail + 4,
    });

    let stretch_x = rng.gen_range(0.6..1.4);
    let stretch_y = rng.gen_range(0.6..1.4);
    let stretch_z = rng.gen_range(0.6..1.4);
    let noise_a = rng.gen_range(0.9..2.4);
    let noise_b = rng.gen_range(0.9..2.4);
    let noise_c = rng.gen_range(0.9..2.4);
    let noise_phase = rng.gen_range(0.0..TAU);

    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for position in positions.iter_mut() {
            let mut point = Vec3::new(position[0], position[1], position[2]);
            let direction = point.normalize_or_zero();
            let surface_noise = (direction.x * noise_a + noise_phase).sin() * 0.34
                + (direction.y * noise_b - noise_phase * 0.6).cos() * 0.28
                + (direction.z * noise_c + noise_phase * 1.3).sin() * 0.22;
            let crater = if direction.y > 0.25 { -0.18 * rng.gen_range(0.6..1.0) } else { 0.0 };
            let irregularity = (1.0 + surface_noise + crater + rng.gen_range(-0.08..0.08)).clamp(0.55, 1.45);

            point.x *= stretch_x * irregularity;
            point.y *= stretch_y * irregularity;
            point.z *= stretch_z * irregularity;
            point *= radius;

            *position = [point.x, point.y, point.z];
        }
    }

    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}

fn make_ring_mesh_library(meshes: &mut ResMut<Assets<Mesh>>, rng: &mut impl Rng) -> RingMeshLibrary {
    let mut high = Vec::new();
    let mut mid = Vec::new();
    let mut low = Vec::new();

    for _ in 0..ASTEROID_LOD_VARIANTS {
        high.push(meshes.add(build_asteroid_shape_mesh(1.0, 12, rng)));
        mid.push(meshes.add(build_asteroid_shape_mesh(1.0, 8, rng)));
        low.push(meshes.add(build_asteroid_shape_mesh(1.0, 4, rng)));
    }

    RingMeshLibrary { high, mid, low }
}

fn make_ring_camera_transform(anchor: Vec3, anchor_radius: f32) -> Transform {
    let pull_back = anchor_radius * 5.0 + 8_000.0;
    let lift = anchor_radius * 2.0 + 3_500.0;
    let mut transform = Transform::from_translation(anchor + Vec3::new(0.0, lift, pull_back));
    transform.look_at(Vec3::ZERO, Vec3::Y);
    transform
}

fn choose_lod(distance: f32) -> AsteroidLod {
    if distance < LOD_HIGH_DISTANCE {
        AsteroidLod::High
    } else if distance < LOD_MID_DISTANCE {
        AsteroidLod::Mid
    } else {
        AsteroidLod::Low
    }
}

/// Squared-distance variant — avoids sqrt in hot LOD update loop.
fn choose_lod_sq(distance_sq: f32) -> AsteroidLod {
    const HIGH_SQ: f32 = LOD_HIGH_DISTANCE * LOD_HIGH_DISTANCE;
    const MID_SQ: f32 = LOD_MID_DISTANCE * LOD_MID_DISTANCE;
    if distance_sq < HIGH_SQ {
        AsteroidLod::High
    } else if distance_sq < MID_SQ {
        AsteroidLod::Mid
    } else {
        AsteroidLod::Low
    }
}

fn mesh_handle_for_lod(lod: AsteroidLod, shape_index: usize, library: &RingMeshLibrary) -> Handle<Mesh> {
    let idx = shape_index % ASTEROID_LOD_VARIANTS;
    match lod {
        AsteroidLod::High => library.high[idx].clone(),
        AsteroidLod::Mid => library.mid[idx].clone(),
        AsteroidLod::Low => library.low[idx].clone(),
    }
}

fn spawn_ring_asteroid(
    commands: &mut Commands,
    library: &RingMeshLibrary,
    palette: &[Handle<StandardMaterial>],
    rng: &mut impl Rng,
    index: usize,
) -> (Vec3, f32) {
    let ring_mix = index as f32 / RING_ASTEROID_COUNT as f32;
    let orbit_radius = rng.gen_range(RING_INNER_RADIUS..RING_OUTER_RADIUS) * (0.94 + ring_mix * 0.08);
    let angle = ring_mix * TAU * 14.0 + rng.gen_range(-0.12..0.12);
    let y = rng.gen_range(-180.0..180.0) * (0.35 + ring_mix * 0.3);
    let base_radius = rng.gen_range(70.0..320.0);
    let shape_index = rng.gen_range(0..ASTEROID_LOD_VARIANTS);
    let position = Vec3::new(angle.cos() * orbit_radius, y, angle.sin() * orbit_radius);
    let initial_lod = choose_lod(position.length());
    let mesh = mesh_handle_for_lod(initial_lod, shape_index, library);
    let orbit_speed = rng.gen_range(0.00010..0.00035) * (SATURN_RADIUS / orbit_radius).sqrt();

    let spin = Vec3::new(
        rng.gen_range(-0.22..0.22),
        rng.gen_range(-0.22..0.22),
        rng.gen_range(-0.22..0.22),
    );

    // Reuse a palette material — shared (mesh, material) pairs enable automatic GPU instancing.
    let material = palette[index % palette.len()].clone();

    commands.spawn((
        PbrBundle {
            mesh,
            material,
            transform: Transform::from_translation(position).with_scale(Vec3::splat(base_radius)),
            ..default()
        },
        RingAsteroid {
            orbit_radius,
            orbit_angle: angle,
            orbit_speed,
            vertical_offset: y,
            shape_index,
            _base_radius: base_radius,
        },
        Asteroid,
        BeltAsteroid,
        Velocity(Vec3::ZERO),
        Radius(base_radius),
        AngularVelocity(spin),
    ));

    (position, base_radius)
}

pub fn update_ring_orbit_system(
    time: Res<Time>,
    paused: Res<TimePaused>,
    mut commands: Commands,
    camera_q: Query<&Transform, (With<MainCamera>, Without<Asteroid>)>,
    prev_cam: Res<PrevCameraPosition>,
    mut asteroids: Query<(Entity, &mut Transform, &mut RingAsteroid, Option<&AngularVelocity>, &Radius), (With<Asteroid>, Without<MainCamera>)>,
    game_timer: Res<GameTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if paused.0 {
        return;
    }

    let Ok(camera_transform) = camera_q.get_single() else { return };
    let dt = time.delta_seconds();

    // Swept-sphere constants — pre-compute camera path segment once.
    let cam_end = camera_transform.translation;
    let cam_start = prev_cam.0;
    let seg = cam_end - cam_start;
    let seg_len_sq = seg.length_squared();
    const CAMERA_RADIUS: f32 = 12.0;

    // Only rotate asteroids within this distance (rotation invisible beyond 50 km).
    const ANGULAR_DIST_SQ: f32 = 50_000.0 * 50_000.0;
    // Pre-cull radius for swept-sphere test: max frame displacement + generous buffer.
    const SWEEP_PRECHECK_EXTRA: f32 = 3_000.0;

    for (entity, mut transform, mut asteroid, ang_opt, radius) in asteroids.iter_mut() {
        asteroid.orbit_angle = (asteroid.orbit_angle + asteroid.orbit_speed * dt).rem_euclid(TAU);
        transform.translation = Vec3::new(
            asteroid.orbit_angle.cos() * asteroid.orbit_radius,
            asteroid.vertical_offset,
            asteroid.orbit_angle.sin() * asteroid.orbit_radius,
        );

        let to_cam = transform.translation - cam_end;
        let dist_sq = to_cam.length_squared();

        // Angular spin: skip for distant asteroids — not perceptible and expensive.
        if dist_sq < ANGULAR_DIST_SQ {
            if let Some(ang) = ang_opt {
                let ang_sq = ang.0.length_squared();
                if ang_sq > 0.0 {
                    let angle = ang_sq.sqrt() * dt;
                    let axis = ang.0 / ang_sq.sqrt();
                    transform.rotate(Quat::from_axis_angle(axis, angle));
                }
            }
        }

        // Player collision — fast pre-reject then swept-sphere.
        let collision_threshold = CAMERA_RADIUS + radius.0;
        let precheck_sq = (collision_threshold + SWEEP_PRECHECK_EXTRA).powi(2);
        if dist_sq < precheck_sq {
            let to_center = transform.translation - cam_start;
            let t = if seg_len_sq > 0.0 { seg.dot(to_center) / seg_len_sq } else { 0.0 };
            let closest = cam_start + seg * t.clamp(0.0, 1.0);
            if (transform.translation - closest).length() < collision_threshold {
                info!("Collision with ring asteroid! Score: {:.1}s", game_timer.0);
                commands.entity(entity).despawn_recursive();
                next_state.set(GameState::Dead);
            }
        }
    }
}

pub fn update_ring_lod_system(
    time: Res<Time>,
    mut timer: ResMut<RingLodUpdateTimer>,
    camera_q: Query<&Transform, With<crate::components::MainCamera>>,
    library: Res<RingMeshLibrary>,
    mut asteroids: Query<(&Transform, &RingAsteroid, &mut Handle<Mesh>)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let Ok(camera_transform) = camera_q.get_single() else { return };

    for (transform, asteroid, mut mesh_handle) in asteroids.iter_mut() {
        // Use distance_squared to avoid 1 080 sqrt calls every 0.2 s.
        let dist_sq = camera_transform.translation.distance_squared(transform.translation);
        let lod = choose_lod_sq(dist_sq);
        let desired = mesh_handle_for_lod(lod, asteroid.shape_index, &library);
        if *mesh_handle != desired {
            *mesh_handle = desired;
        }
    }
}

pub fn spawn_space_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
    rng: &mut impl Rng,
) -> Transform {
    let starfield = images.add(make_starfield_texture(rng));
    let nebula = images.add(make_nebula_texture(rng));
    let saturn_texture = images.add(make_saturn_texture());
    let ring_mesh_library = make_ring_mesh_library(meshes, rng);
    commands.insert_resource(RingMeshLibrary {
        high: ring_mesh_library.high.clone(),
        mid: ring_mesh_library.mid.clone(),
        low: ring_mesh_library.low.clone(),
    });

    commands.spawn((
        PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: BACKDROP_RADIUS,
            sectors: 64,
            stacks: 32,
        })),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(starfield),
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        ..default()
        },
        SkyDome,
    ));

    commands.spawn((
        PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: BACKDROP_RADIUS * 0.96,
            sectors: 48,
            stacks: 24,
        })),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(nebula),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        }),
        ..default()
        },
        SkyDome,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: SATURN_RADIUS,
                sectors: 48,
                stacks: 28,
            })),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(saturn_texture),
                perceptual_roughness: 1.0,
                metallic: 0.0,
                reflectance: 0.04,
                ..default()
            }),
            ..default()
        },
        Saturn,
    ));

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: SATURN_RADIUS * 1.04,
            sectors: 36,
            stacks: 20,
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::rgba(0.95, 0.76, 0.52, 0.12),
            emissive: Color::rgba(0.16, 0.10, 0.05, 1.0),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 180_000.0,
            shadows_enabled: false, // Shadow maps across 1080 asteroids tanks performance.
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -1.0, 0.0)),
        ..default()
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 6_500_000.0,
            range: 250_000.0,
            color: Color::rgb(1.0, 0.82, 0.63),
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-120_000.0, 80_000.0, 30_000.0)),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::rgb(0.08, 0.10, 0.16),
        brightness: 1.2,
    });

    let mut spawn_anchor = Vec3::ZERO;
    let mut spawn_anchor_radius = 0.0;

    // Build a small palette of shared materials so asteroids with the
    // same (mesh, material) pair are automatically GPU-instanced by Bevy’s
    // render batcher, reducing ~1 080 draw calls to ~50–100.
    const N_RING_MATERIALS: usize = 8;
    let ring_material_palette: Vec<Handle<StandardMaterial>> = (0..N_RING_MATERIALS)
        .map(|i| {
            let ring_mix = i as f32 / (N_RING_MATERIALS - 1) as f32;
            materials.add(StandardMaterial {
                base_color: Color::rgb(
                    0.26 + ring_mix * 0.12,
                    0.24 + ring_mix * 0.09,
                    0.22 + ring_mix * 0.06,
                ),
                perceptual_roughness: 1.0,
                metallic: 0.0,
                reflectance: 0.03,
                ..default()
            })
        })
        .collect();

    for index in 0..RING_ASTEROID_COUNT {
        let (position, radius) = spawn_ring_asteroid(commands, &ring_mesh_library, &ring_material_palette, rng, index);

        if index == RING_ASTEROID_COUNT / 3 {
            spawn_anchor = position;
            spawn_anchor_radius = radius;
        }
    }

    make_ring_camera_transform(spawn_anchor, spawn_anchor_radius)
}

pub fn make_cinematic_bloom() -> BloomSettings {
    BloomSettings::NATURAL
}

pub fn follow_sky_dome_system(
    camera_q: Query<&Transform, With<crate::components::MainCamera>>,
    mut dome_q: Query<&mut Transform, (With<SkyDome>, Without<crate::components::MainCamera>)>,
) {
    let Ok(camera_transform) = camera_q.get_single() else { return };

    for mut transform in dome_q.iter_mut() {
        transform.translation = camera_transform.translation;
    }
}
