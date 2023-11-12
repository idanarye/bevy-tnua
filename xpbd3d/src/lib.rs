use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use bevy_tnua_physics_integration_layer::data_for_backends::TnuaGhostPlatform;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaGhostSensor;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaToggle;
use bevy_tnua_physics_integration_layer::data_for_backends::{
    TnuaMotor, TnuaProximitySensor, TnuaProximitySensorOutput, TnuaRigidBodyTracker,
};
use bevy_tnua_physics_integration_layer::subservient_sensors::TnuaSubservientSensor;
use bevy_tnua_physics_integration_layer::TnuaPipelineStages;
use bevy_tnua_physics_integration_layer::TnuaSystemSet;

/// Add this plugin to use bevy_xpbd_3d as a physics backend.
///
/// This plugin should be used in addition to
/// [`TnuaControllerPlugin`](crate::prelude::TnuaControllerPlugin).
pub struct TnuaXpbd3dPlugin;

impl Plugin for TnuaXpbd3dPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            TnuaSystemSet.run_if(|physics_time: Res<Time<Physics>>| !physics_time.is_paused()),
        );
        app.add_systems(
            Update,
            (
                update_rigid_body_trackers_system,
                update_proximity_sensors_system,
            )
                .in_set(TnuaPipelineStages::Sensors),
        );
        app.add_systems(
            Update,
            apply_motors_system.in_set(TnuaPipelineStages::Motors),
        );
    }
}

/// `bevy_rapier3d`-specific components required for Tnua to work.
#[derive(Bundle, Default)]
pub struct TnuaXpbd3dIOBundle {
    // pub velocity: LinearVelocity,
    // pub external_force: ExternalForce,
    // pub mass: Mass,
}

/// Add this component to make [`TnuaProximitySensor`] cast a shape instead of a ray.
#[derive(Component)]
pub struct TnuaXpbd3dSensorShape(pub Collider);

fn update_rigid_body_trackers_system(
    gravity: Res<Gravity>,
    mut query: Query<(
        &GlobalTransform,
        &LinearVelocity,
        &AngularVelocity,
        &mut TnuaRigidBodyTracker,
        Option<&TnuaToggle>,
    )>,
) {
    for (transform, linaer_velocity, angular_velocity, mut tracker, tnua_toggle) in query.iter_mut()
    {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        *tracker = TnuaRigidBodyTracker {
            translation,
            rotation,
            velocity: linaer_velocity.0,
            angvel: angular_velocity.0,
            gravity: gravity.0,
        };
    }
}

fn update_proximity_sensors_system(
    spatial_query_pipeline: Res<SpatialQueryPipeline>,
    collisions: Res<Collisions>,
    // rapier_context: Res<RapierContext>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut TnuaProximitySensor,
        Option<&TnuaXpbd3dSensorShape>,
        Option<&mut TnuaGhostSensor>,
        Option<&TnuaSubservientSensor>,
        Option<&TnuaToggle>,
    )>,
    other_object_query: Query<(
        Option<(&GlobalTransform, &LinearVelocity, &AngularVelocity)>,
        Has<TnuaGhostPlatform>,
        Has<Sensor>,
    )>,
) {
    query.par_iter_mut().for_each(
        |(
            owner_entity,
            transform,
            mut sensor,
            shape,
            mut ghost_sensor,
            subservient,
            tnua_toggle,
        )| {
            match tnua_toggle.copied().unwrap_or_default() {
                TnuaToggle::Disabled => return,
                TnuaToggle::SenseOnly => {}
                TnuaToggle::Enabled => {}
            }
            let cast_origin = transform.transform_point(sensor.cast_origin);
            let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
            let cast_direction = owner_rotation * sensor.cast_direction;

            struct CastResult {
                entity: Entity,
                proximity: f32,
                intersection_point: Vec3,
                normal: Vec3,
            }

            let owner_entity = if let Some(subservient) = subservient {
                subservient.owner_entity
            } else {
                owner_entity
            };

            let mut final_sensor_output = None;
            if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                ghost_sensor.0.clear();
            }
            let mut apply_cast = |cast_result: CastResult| {
                let CastResult {
                    entity,
                    proximity,
                    intersection_point,
                    normal,
                } = cast_result;

                // This fixes https://github.com/idanarye/bevy-tnua/issues/14
                if let Some(contacts) = collisions.get(owner_entity, entity) {
                    let same_order = owner_entity == contacts.entity1;
                    for manifold in contacts.manifolds.iter() {
                        if !manifold.contacts.is_empty() {
                            let manifold_normal = if same_order {
                                manifold.normal2
                            } else {
                                manifold.normal1
                            };
                            if sensor.intersection_match_prevention_cutoff
                                < manifold_normal.dot(cast_direction)
                            {
                                return true;
                            }
                        }
                    }
                }

                // TODO: see if https://github.com/idanarye/bevy-tnua/issues/14 replicates in XPBD,
                // and if figure out how to port its fix to XPBD.

                let Ok((entity_kinematic_data, entity_is_ghost, entity_is_sensor)) =
                    other_object_query.get(entity)
                else {
                    return false;
                };

                let entity_linvel;
                let entity_angvel;
                if let Some((entity_transform, entity_linear_velocity, entity_angular_velocity)) =
                    entity_kinematic_data
                {
                    entity_angvel = entity_angular_velocity.0;
                    entity_linvel = entity_linear_velocity.0
                        + if 0.0 < entity_angvel.length_squared() {
                            let relative_point =
                                intersection_point - entity_transform.translation();
                            // NOTE: no need to project relative_point on the
                            // rotation plane, it will not affect the cross
                            // product.
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
                    normal,
                    entity_linvel,
                    entity_angvel,
                };
                if entity_is_ghost {
                    if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                        ghost_sensor.0.push(sensor_output);
                    }
                    true
                } else if entity_is_sensor {
                    true
                } else {
                    final_sensor_output = Some(sensor_output);
                    false
                }
            };

            let query_filter = SpatialQueryFilter::new().without_entities([owner_entity]);
            if let Some(TnuaXpbd3dSensorShape(shape)) = shape {
                spatial_query_pipeline.shape_hits_callback(
                    shape,
                    cast_origin,
                    owner_rotation,
                    cast_direction,
                    sensor.cast_range,
                    true,
                    query_filter,
                    |shape_hit_data| {
                        apply_cast(CastResult {
                            entity: shape_hit_data.entity,
                            proximity: shape_hit_data.time_of_impact,
                            intersection_point: shape_hit_data.point1,
                            normal: shape_hit_data.normal1,
                        })
                    },
                );
            } else {
                spatial_query_pipeline.ray_hits_callback(
                    cast_origin,
                    cast_direction,
                    sensor.cast_range,
                    true,
                    query_filter,
                    |ray_hit_data| {
                        apply_cast(CastResult {
                            entity: ray_hit_data.entity,
                            proximity: ray_hit_data.time_of_impact,
                            intersection_point: cast_origin
                                + ray_hit_data.time_of_impact * cast_direction,
                            normal: ray_hit_data.normal,
                        })
                    },
                );
            }
            sensor.output = final_sensor_output;
        },
    );
}

fn apply_motors_system(
    mut query: Query<(
        &TnuaMotor,
        &mut LinearVelocity,
        &mut AngularVelocity,
        &Mass,
        &Inertia,
        &mut ExternalForce,
        &mut ExternalTorque,
        Option<&TnuaToggle>,
    )>,
) {
    for (
        motor,
        mut linare_velocity,
        mut angular_velocity,
        mass,
        inertia,
        mut external_force,
        mut external_torque,
        tnua_toggle,
    ) in query.iter_mut()
    {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled | TnuaToggle::SenseOnly => {
                *external_force = Default::default();
                return;
            }
            TnuaToggle::Enabled => {}
        }
        if motor.lin.boost.is_finite() {
            linare_velocity.0 += motor.lin.boost;
        }
        if motor.lin.acceleration.is_finite() {
            external_force.set_force(motor.lin.acceleration * mass.0);
        }
        if motor.ang.boost.is_finite() {
            angular_velocity.0 += motor.ang.boost;
        }
        if motor.ang.acceleration.is_finite() {
            external_torque.set_torque(
                // NOTE: I did not actually verify that this is the correct formula. Nothing uses
                // angular acceleration yet - only angular impulses.
                inertia.0 * motor.ang.acceleration,
            );
        }
    }
}
