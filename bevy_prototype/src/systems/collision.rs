use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::{Asteroid, BeltAsteroid, Radius, Velocity};
use crate::resources::VelocityUpdates;

const BROADPHASE_CELL_SIZE: f32 = 700.0;

fn world_to_cell(position: Vec3) -> (i32, i32, i32) {
    (
        (position.x / BROADPHASE_CELL_SIZE).floor() as i32,
        (position.y / BROADPHASE_CELL_SIZE).floor() as i32,
        (position.z / BROADPHASE_CELL_SIZE).floor() as i32,
    )
}

pub fn asteroid_collision_system(
    asteroids: Query<(Entity, &Velocity, &Radius, &Transform), (With<Asteroid>, Without<BeltAsteroid>)>,
    mut updates: ResMut<VelocityUpdates>,
) {
    struct AstState {
        entity: Entity,
        pos: Vec3,
        vel: Vec3,
        radius: f32,
        mass: f32,
        cell: (i32, i32, i32),
    }

    let mut states: Vec<AstState> = Vec::new();
    let mut buckets: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (entity, vel, radius, transform) in asteroids.iter() {
        let cell = world_to_cell(transform.translation);
        let index = states.len();
        states.push(AstState {
            entity,
            pos: transform.translation,
            vel: vel.0,
            radius: radius.0,
            mass: radius.0 * radius.0,
            cell,
        });
        buckets.entry(cell).or_default().push(index);
    }

    updates.0.clear();
    for s in &states {
        updates.0.insert(s.entity, s.vel);
    }

    let e = 1.0; // restitution
    for i in 0..states.len() {
        let a = &states[i];
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor = (a.cell.0 + dx, a.cell.1 + dy, a.cell.2 + dz);
                    let Some(indices) = buckets.get(&neighbor) else {
                        continue;
                    };

                    for &j in indices {
                        if j <= i {
                            continue;
                        }

                        let b = &states[j];
                        let delta = a.pos - b.pos;
                        let dist = delta.length();
                        let min_dist = a.radius + b.radius;
                        if dist <= 0.0 || dist >= min_dist {
                            continue;
                        }

                        let v1 = *updates.0.get(&a.entity).unwrap_or(&a.vel);
                        let v2 = *updates.0.get(&b.entity).unwrap_or(&b.vel);
                        let rv = v1 - v2;
                        let n = if dist > 0.0 { delta / dist } else { Vec3::X };
                        let vel_along_normal = rv.dot(n);
                        if vel_along_normal >= 0.0 {
                            continue;
                        }

                        let j_impulse = -(1.0 + e) * vel_along_normal / (1.0 / a.mass + 1.0 / b.mass);
                        let impulse = n * j_impulse;
                        let v1_after = v1 + impulse / a.mass;
                        let v2_after = v2 - impulse / b.mass;

                        updates.0.insert(a.entity, v1_after);
                        updates.0.insert(b.entity, v2_after);
                    }
                }
            }
        }
    }
}
