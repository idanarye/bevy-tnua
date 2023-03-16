use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    TnuaMotor, TnuaProximitySensor, TnuaProximitySensorOutput, TnuaRigidBodyTracker, TnuaSystemSet,
};

/// Add this plugin to use bevy_rapier2d as a physics backend.
///
/// This plugin should be used in addition to
/// [`TnuaPlatformerPlugin`](crate::TnuaPlatformerPlugin).
pub struct TnuaRapier2dPlugin;

impl Plugin for TnuaRapier2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                update_rigid_body_trackers_system,
                update_proximity_sensors_system,
            )
                .in_set(TnuaSystemSet::Sensors),
        );
        app.add_system(apply_motors_system.in_set(TnuaSystemSet::Motors));
    }
}

/// Add this component to make [`TnuaProximitySensor`] cast a shape instead of a ray.
#[derive(Component)]
pub struct TnuaRapier2dSensorShape(pub Collider);

fn update_rigid_body_trackers_system(
    rapier_config: Res<RapierConfiguration>,
    mut query: Query<(&Velocity, &mut TnuaRigidBodyTracker)>,
) {
    for (velocity, mut tracker) in query.iter_mut() {
        *tracker = TnuaRigidBodyTracker {
            velocity: velocity.linvel.extend(0.0),
            angvel: Vec3::new(0.0, 0.0, velocity.angvel),
            gravity: rapier_config.gravity.extend(0.0),
        };
    }
}

fn update_proximity_sensors_system(
    rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut TnuaProximitySensor,
        Option<&TnuaRapier2dSensorShape>,
        Option<&CollisionGroups>,
        Option<&SolverGroups>,
    )>,
    other_object_query_query: Query<(&GlobalTransform, &Velocity)>,
    solver_groups_query: Query<&SolverGroups>,
) {
    for (owner_entity, transform, mut sensor, shape, collision_groups, solver_groups) in
        query.iter_mut()
    {
        let cast_origin = transform.transform_point(sensor.cast_origin);
        let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
        let cast_direction = owner_rotation * sensor.cast_direction;

        struct CastResult {
            entity: Entity,
            proximity: f32,
            intersection_point: Vec2,
            normal: Vec2,
        }

        let mut query_filter = QueryFilter::new().exclude_rigid_body(owner_entity);

        query_filter.groups = collision_groups.copied();

        let predicate_for_solver_groups;
        if let Some(solver_groups) = solver_groups {
            assert!(
                query_filter.predicate.is_none(),
                "predicate already set by something else"
            );
            predicate_for_solver_groups = |other_entity: Entity| {
                if let Ok(other_solver_groups) = solver_groups_query.get(other_entity) {
                    solver_groups
                        .memberships
                        .intersects(other_solver_groups.filters)
                        && solver_groups
                            .filters
                            .intersects(other_solver_groups.memberships)
                } else {
                    true
                }
            };
            query_filter.predicate = Some(&predicate_for_solver_groups);
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
                    query_filter,
                )
                .map(|(entity, toi)| CastResult {
                    entity,
                    proximity: toi.toi,
                    intersection_point: toi.witness1,
                    normal: toi.normal1,
                })
        } else {
            rapier_context
                .cast_ray_and_get_normal(
                    cast_origin.truncate(),
                    cast_direction.truncate(),
                    sensor.cast_range,
                    false,
                    query_filter,
                )
                .map(|(entity, toi)| CastResult {
                    entity,
                    proximity: toi.toi,
                    intersection_point: toi.point,
                    normal: toi.normal,
                })
        };
        if let Some(CastResult {
            entity,
            proximity,
            intersection_point,
            normal,
        }) = cast_result
        {
            let entity_linvel;
            let entity_angvel;
            if let Ok((entity_transform, entity_velocity)) = other_object_query_query.get(entity) {
                entity_angvel = Vec3::new(0.0, 0.0, entity_velocity.angvel);
                entity_linvel = entity_velocity.linvel.extend(0.0)
                    + if 0.0 < entity_velocity.angvel.abs() {
                        let relative_point =
                            intersection_point.extend(0.0) - entity_transform.translation();
                        // NOTE: no need to project relative_point on the rotation plane, it will not
                        // affect the cross product.
                        entity_angvel.cross(relative_point)
                    } else {
                        Vec3::ZERO
                    };
            } else {
                entity_angvel = Vec3::ZERO;
                entity_linvel = Vec3::ZERO;
            }
            sensor.output = Some(TnuaProximitySensorOutput {
                entity,
                proximity,
                normal: normal.extend(0.0),
                entity_linvel,
                entity_angvel,
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
        velocity.angvel += motor.desired_angacl.z;
    }
}
