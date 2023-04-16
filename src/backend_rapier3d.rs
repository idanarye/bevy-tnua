use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    TnuaMotor, TnuaPipelineStages, TnuaProximitySensor, TnuaProximitySensorOutput,
    TnuaRigidBodyTracker,
};

/// Add this plugin to use bevy_rapier3d as a physics backend.
///
/// This plugin should be used in addition to
/// [`TnuaPlatformerPlugin`](crate::TnuaPlatformerPlugin).
pub struct TnuaRapier3dPlugin;

impl Plugin for TnuaRapier3dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                update_rigid_body_trackers_system,
                update_proximity_sensors_system,
            )
                .in_set(TnuaPipelineStages::Sensors),
        );
        app.add_system(apply_motors_system.in_set(TnuaPipelineStages::Motors));
    }
}

/// Add this component to make [`TnuaProximitySensor`] cast a shape instead of a ray.
#[derive(Component)]
pub struct TnuaRapier3dSensorShape(pub Collider);

fn update_rigid_body_trackers_system(
    rapier_config: Res<RapierConfiguration>,
    mut query: Query<(&Velocity, &mut TnuaRigidBodyTracker)>,
) {
    for (velocity, mut tracker) in query.iter_mut() {
        *tracker = TnuaRigidBodyTracker {
            velocity: velocity.linvel,
            angvel: velocity.angvel,
            gravity: rapier_config.gravity,
        };
    }
}

fn update_proximity_sensors_system(
    rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut TnuaProximitySensor,
        Option<&TnuaRapier3dSensorShape>,
        Option<&CollisionGroups>,
        Option<&SolverGroups>,
    )>,
    other_object_query: Query<(&GlobalTransform, &Velocity)>,
    solver_groups_query: Query<&SolverGroups>,
    sensor_collider_query: Query<(), With<Sensor>>,
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
            intersection_point: Vec3,
            normal: Vec3,
        }

        let mut query_filter = QueryFilter::new().exclude_rigid_body(owner_entity);

        query_filter.groups = collision_groups.copied();

        let predicate = |other_entity: Entity| {
            if sensor_collider_query.contains(other_entity) {
                return false;
            }
            if let Some(solver_groups) = solver_groups {
                if let Ok(other_solver_groups) = solver_groups_query.get(other_entity) {
                    if !(solver_groups
                        .memberships
                        .intersects(other_solver_groups.filters)
                        && solver_groups
                            .filters
                            .intersects(other_solver_groups.memberships))
                    {
                        return false;
                    }
                }
            }
            if let Some(contact) = rapier_context.contact_pair(owner_entity, other_entity) {
                let same_order = owner_entity == contact.collider1();
                for manifold in contact.manifolds() {
                    if 0 < manifold.num_points() {
                        let manifold_normal = if same_order {
                            manifold.local_n2()
                        } else {
                            manifold.local_n1()
                        };
                        if sensor.intersection_match_prevention_cutoff
                            < manifold_normal.dot(cast_direction)
                        {
                            return false;
                        }
                    }
                }
            }
            true
        };
        query_filter.predicate = Some(&predicate);

        let cast_result = if let Some(TnuaRapier3dSensorShape(shape)) = shape {
            rapier_context
                .cast_shape(
                    cast_origin,
                    owner_rotation,
                    cast_direction,
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
                    cast_origin,
                    cast_direction,
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
            if let Ok((entity_transform, entity_velocity)) = other_object_query.get(entity) {
                entity_angvel = entity_velocity.angvel;
                entity_linvel = entity_velocity.linvel
                    + if 0.0 < entity_angvel.length_squared() {
                        let relative_point = intersection_point - entity_transform.translation();
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
                normal,
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
        velocity.linvel += motor.desired_acceleration;
        velocity.angvel += motor.desired_angacl;
    }
}
