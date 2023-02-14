use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    tnua_system_set_for_applying_motors, tnua_system_set_for_reading_sensor,
    TnuaDataSynchronizedFromBackend, TnuaMotor, TnuaProximitySensor, TnuaProximitySensorOutput,
};

pub struct TnuaRapier2dPlugin;

impl Plugin for TnuaRapier2dPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TnuaDataSynchronizedFromBackend::default());
        app.add_system(synchronize_data_from_backend);
        app.add_system_set(
            tnua_system_set_for_reading_sensor().with_system(update_proximity_sensors_system),
        );
        app.add_system_set(tnua_system_set_for_applying_motors().with_system(apply_motors_system));
    }
}

#[derive(Component)]
pub struct TnuaRapier2dSensorShape(pub Collider);

fn synchronize_data_from_backend(
    backend_data: Res<RapierConfiguration>,
    mut data_synchronized_from_backend: ResMut<TnuaDataSynchronizedFromBackend>,
) {
    *data_synchronized_from_backend = TnuaDataSynchronizedFromBackend {
        gravity: backend_data.gravity.extend(0.0),
    };
}

fn update_proximity_sensors_system(
    rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &Velocity,
        &mut TnuaProximitySensor,
        Option<&TnuaRapier2dSensorShape>,
    )>,
) {
    for (owner_entity, transform, velocity, mut sensor, shape) in query.iter_mut() {
        let cast_origin = transform.transform_point(sensor.cast_origin);
        let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
        let cast_direction = owner_rotation * sensor.cast_direction;
        sensor.velocity = velocity.linvel.extend(0.0);

        struct CastResult {
            entity: Entity,
            proximity: f32,
            normal: Vec2,
        }

        let cast_result = if let Some(TnuaRapier2dSensorShape(shape)) = shape {
            let (_, _, rotation_z) = owner_rotation.to_euler(EulerRot::XYZ);
            rapier_context
                .cast_shape(
                    cast_origin.truncate(),
                    rotation_z,
                    cast_direction.truncate(),
                    shape,
                    sensor.cast_range,
                    QueryFilter::new().exclude_rigid_body(owner_entity),
                )
                .map(|(entity, toi)| CastResult {
                    entity,
                    proximity: toi.toi,
                    normal: toi.normal1,
                })
        } else {
            rapier_context
                .cast_ray_and_get_normal(
                    cast_origin.truncate(),
                    cast_direction.truncate(),
                    sensor.cast_range,
                    false,
                    QueryFilter::new().exclude_rigid_body(owner_entity),
                )
                .map(|(entity, toi)| CastResult {
                    entity,
                    proximity: toi.toi,
                    normal: toi.normal,
                })
        };
        if let Some(CastResult {
            entity,
            proximity,
            normal,
        }) = cast_result
        {
            sensor.output = Some(TnuaProximitySensorOutput {
                entity,
                proximity,
                normal: normal.extend(0.0),
                // TODO: make it relative to the entity's velocity
                relative_velocity: velocity.linvel.extend(0.0),
            });
        } else {
            sensor.output = None;
        }
    }
}

fn apply_motors_system(mut query: Query<(&TnuaMotor, &mut Velocity)>) {
    for (motor, mut velocity) in query.iter_mut() {
        if !motor.desired_acceleration.is_finite() {
            continue;
        }
        velocity.linvel += motor.desired_acceleration.truncate();
    }
}
