use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::rapier;
use bevy_rapier2d::rapier::prelude::InteractionGroups;

use crate::subservient_sensors::TnuaSubservientSensor;
use crate::TnuaGhostPlatform;
use crate::TnuaGhostSensor;
use crate::{
    TnuaMotor, TnuaPipelineStages, TnuaProximitySensor, TnuaProximitySensorOutput,
    TnuaRigidBodyTracker,
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
                .in_set(TnuaPipelineStages::Sensors),
        );
        app.add_system(apply_motors_system.in_set(TnuaPipelineStages::Motors));
    }
}

/// `bevy_rapier_2d`-specific components required for Tnua to work.
#[derive(Bundle, Default)]
pub struct TnuaRapier2dIOBundle {
    pub velocity: Velocity,
    pub external_force: ExternalForce,
    pub read_mass_properties: ReadMassProperties,
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

fn get_collider(
    rapier_context: &RapierContext,
    entity: Entity,
) -> Option<&rapier::geometry::Collider> {
    let collider_handle = rapier_context.entity2collider().get(&entity)?;
    rapier_context.colliders.get(*collider_handle)
    //if let Some(owner_collider) = rapier_context.entity2collider().get(&owner_entity).and_then(|handle| rapier_context.colliders.get(*handle)) {
}

fn update_proximity_sensors_system(
    rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut TnuaProximitySensor,
        Option<&TnuaRapier2dSensorShape>,
        Option<&mut TnuaGhostSensor>,
        Option<&TnuaSubservientSensor>,
    )>,
    ghost_platforms_query: Query<With<TnuaGhostPlatform>>,
    other_object_query_query: Query<(&GlobalTransform, &Velocity)>,
) {
    query.par_iter_mut().for_each_mut(
        |(owner_entity, transform, mut sensor, shape, mut ghost_sensor, subservient)| {
            let cast_origin = transform.transform_point(sensor.cast_origin);
            let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
            let cast_direction = owner_rotation * sensor.cast_direction;

            struct CastResult {
                entity: Entity,
                proximity: f32,
                intersection_point: Vec2,
                normal: Vec2,
            }

            let owner_entity = if let Some(subservient) = subservient {
                subservient.owner_entity
            } else {
                owner_entity
            };

            let mut query_filter = QueryFilter::new().exclude_rigid_body(owner_entity);
            let owner_solver_groups: InteractionGroups;

            if let Some(owner_collider) = get_collider(&rapier_context, owner_entity) {
                let collision_groups = owner_collider.collision_groups();
                query_filter.groups = Some(CollisionGroups {
                    memberships: Group::from_bits_truncate(collision_groups.memberships.bits()),
                    filters: Group::from_bits_truncate(collision_groups.filter.bits()),
                });
                owner_solver_groups = owner_collider.solver_groups();
            } else {
                owner_solver_groups = InteractionGroups::all();
            }

            let mut already_visited_ghost_entities = HashSet::<Entity>::default();

            let has_ghost_sensor = ghost_sensor.is_some();

            let do_cast = |cast_range_skip: f32,
                           already_visited_ghost_entities: &HashSet<Entity>|
             -> Option<CastResult> {
                let predicate = |other_entity: Entity| {
                    if let Some(other_collider) = get_collider(&rapier_context, other_entity) {
                        if !other_collider.solver_groups().test(owner_solver_groups) {
                            if has_ghost_sensor && ghost_platforms_query.contains(other_entity) {
                                if already_visited_ghost_entities.contains(&other_entity) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        if other_collider.is_sensor() {
                            return false;
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
                                    < manifold_normal.dot(cast_direction.truncate())
                                {
                                    return false;
                                }
                            }
                        }
                    }
                    true
                };
                let query_filter = query_filter.clone().predicate(&predicate);
                let cast_origin = cast_origin + cast_range_skip * cast_direction;
                let cast_range = sensor.cast_range - cast_range_skip;
                if let Some(TnuaRapier2dSensorShape(shape)) = shape {
                    let (_, _, rotation_z) = owner_rotation.to_euler(EulerRot::XYZ);
                    rapier_context
                        .cast_shape(
                            cast_origin.truncate(),
                            rotation_z,
                            cast_direction.truncate(),
                            shape,
                            cast_range,
                            query_filter,
                        )
                        .map(|(entity, toi)| CastResult {
                            entity,
                            proximity: toi.toi + cast_range_skip,
                            intersection_point: toi.witness1,
                            normal: toi.normal1,
                        })
                } else {
                    rapier_context
                        .cast_ray_and_get_normal(
                            cast_origin.truncate(),
                            cast_direction.truncate(),
                            cast_range,
                            false,
                            query_filter,
                        )
                        .map(|(entity, toi)| CastResult {
                            entity,
                            proximity: toi.toi + cast_range_skip,
                            intersection_point: toi.point,
                            normal: toi.normal,
                        })
                }
            };

            let mut cast_range_skip = 0.0;
            if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                ghost_sensor.0.clear();
            }
            sensor.output = 'sensor_output: loop {
                if let Some(CastResult {
                    entity,
                    proximity,
                    intersection_point,
                    normal,
                }) = do_cast(cast_range_skip, &already_visited_ghost_entities)
                {
                    let entity_linvel;
                    let entity_angvel;
                    if let Ok((entity_transform, entity_velocity)) =
                        other_object_query_query.get(entity)
                    {
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
                    let sensor_output = TnuaProximitySensorOutput {
                        entity,
                        proximity,
                        normal: normal.extend(0.0),
                        entity_linvel,
                        entity_angvel,
                    };
                    if ghost_platforms_query.contains(entity) {
                        cast_range_skip = proximity;
                        already_visited_ghost_entities.insert(entity);
                        if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                            ghost_sensor.0.push(sensor_output);
                        }
                    } else {
                        break 'sensor_output Some(sensor_output);
                    }
                } else {
                    break 'sensor_output None;
                }
            };
        },
    );
}

fn apply_motors_system(
    mut query: Query<(
        &TnuaMotor,
        &mut Velocity,
        &ReadMassProperties,
        &mut ExternalForce,
    )>,
) {
    for (motor, mut velocity, mass_properties, mut external_force) in query.iter_mut() {
        if motor.lin.boost.is_finite() {
            velocity.linvel += motor.lin.boost.truncate();
        }
        if motor.lin.acceleration.is_finite() {
            external_force.force = motor.lin.acceleration.truncate() * mass_properties.0.mass;
        }
        if motor.ang.boost.is_finite() {
            velocity.angvel += motor.ang.boost.z;
        }
        if motor.ang.acceleration.is_finite() {
            external_force.torque = motor.ang.acceleration.z * mass_properties.0.principal_inertia;
        }
    }
}
