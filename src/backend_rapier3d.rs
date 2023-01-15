use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    tnua_system_set_for_applying_motors, tnua_system_set_for_reading_sensor, TnuaMotor,
    TnuaProximitySensor, TnuaProximitySensorOutput,
};

pub struct TnuaRapier3dPlugin;

impl Plugin for TnuaRapier3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            tnua_system_set_for_reading_sensor().with_system(update_proximity_sensors_system),
        );
        app.add_system_set(tnua_system_set_for_applying_motors().with_system(apply_motors_system));
    }
}

fn update_proximity_sensors_system(
    rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &Velocity,
        &mut TnuaProximitySensor,
    )>,
) {
    for (owner_entity, transform, velocity, mut sensor) in query.iter_mut() {
        let cast_origin = transform.transform_point(sensor.cast_origin);
        let cast_direction = transform.to_scale_rotation_translation().1 * sensor.cast_direction;
        if let Some((entity, toi)) = rapier_context.cast_ray_and_get_normal(
            cast_origin,
            cast_direction,
            sensor.cast_range,
            false,
            QueryFilter::new().exclude_rigid_body(owner_entity),
        ) {
            sensor.output = Some(TnuaProximitySensorOutput {
                entity,
                proximity: toi.toi,
                normal: toi.normal,
                // TODO: make it relative to the entity's velocity
                relative_velocity: velocity.linvel,
            });
        } else {
            sensor.output = None;
        }
    }
}

fn apply_motors_system(time: Res<Time>, mut query: Query<(&TnuaMotor, &mut Velocity)>) {
    for (motor, mut velocity) in query.iter_mut() {
        if !motor.desired_acceleration.is_finite() {
            continue;
        }
        velocity.linvel += time.delta().as_secs_f32() * motor.desired_acceleration;
    }
}
