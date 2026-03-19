use bevy::prelude::*;

use crate::components::*;
use crate::resources::*;

pub fn asteroid_movement_system(
    time: Res<Time>,
    mut commands: Commands,
    mut asteroids: Query<(Entity, &mut Velocity, &Radius, &mut Transform), With<Asteroid>>,
    shuttle_pos: Res<ShuttlePosition>,
    updates: Res<VelocityUpdates>,
) {
    let shuttle_opt = Some(shuttle_pos.0);

    for (entity, mut vel_comp, _radius, mut transform) in asteroids.iter_mut() {
        if let Some(new_vel) = updates.0.get(&entity) {
            vel_comp.0 = *new_vel;
        }

        transform.translation += vel_comp.0.extend(0.0) * time.delta_seconds();

        if transform.translation.y < -800.0 || transform.translation.x.abs() > 1200.0 {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        if let Some(sh_pos) = shuttle_opt {
            let dist = (transform.translation.truncate() - sh_pos).length();
            let shuttle_radius = 15.0;
            let rad = 16.0;
            if dist < shuttle_radius + rad {
                info!("Collision with asteroid (shuttle)!");
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
