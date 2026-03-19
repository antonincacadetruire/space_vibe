use bevy::prelude::*;

use crate::components::*;
use crate::resources::VelocityUpdates;

pub fn asteroid_collision_system(
    asteroids: Query<(Entity, &Velocity, &Radius, &Transform), With<Asteroid>>,
    mut updates: ResMut<VelocityUpdates>,
) {
    struct AstState {
        entity: Entity,
        pos: Vec2,
        vel: Vec2,
        radius: f32,
        mass: f32,
    }

    let mut states: Vec<AstState> = Vec::new();
    for (entity, vel, radius, transform) in asteroids.iter() {
        states.push(AstState {
            entity,
            pos: transform.translation.truncate(),
            vel: vel.0,
            radius: radius.0,
            mass: radius.0 * radius.0,
        });
    }

    updates.0.clear();
    for s in &states {
        updates.0.insert(s.entity, s.vel);
    }

    let e = 1.0;
    for i in 0..states.len() {
        for j in (i + 1)..states.len() {
            let a = &states[i];
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
            let n = if dist > 0.0 { delta / dist } else { Vec2::X };
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
