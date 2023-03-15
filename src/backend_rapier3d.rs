use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    TnuaMotor, TnuaProximitySensor, TnuaProximitySensorOutput, TnuaRigidBodyTracker, TnuaSystemSet,
};

pub struct TnuaRapier3dPlugin;

impl Plugin for TnuaRapier3dPlugin {
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
    velocity_query: Query<&Velocity>,
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
            normal: Vec3,
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
                    normal: toi.normal,
                })
        };

        if let Some(CastResult {
            entity,
            proximity,
            normal,
        }) = cast_result
        {
            let entity_linvel;
            let entity_angvel;
            if let Ok(entity_velocity) = velocity_query.get(entity) {
                // TODO: When there is angular velocity, the linear velocity needs
                // to be calculated for the point in the rigid body where the
                // casted ray/shape hits.
                entity_linvel = entity_velocity.linvel;
                entity_angvel = entity_velocity.angvel;
            } else {
                entity_linvel = Vec3::ZERO;
                entity_angvel = Vec3::ZERO;
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
