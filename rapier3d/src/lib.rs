//! # bevy_rapier3d Integration for bevy-tnua
//!
//! In addition to the instruction in bevy-tnua's documentation:
//!
//! * Add [`TnuaRapier3dPlugin`] to the Bevy app.
//! * Optionally: Add [`TnuaRapier3dSensorShape`] to either entity of the character controller by
//!   Tnua or to the to the sensor entities. If exists, the shape on the sensor entity overrides
//!   the one on the character entity for that specific sensor.
mod helpers;
mod spatial_ext;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_rapier3d::rapier;
use bevy_rapier3d::rapier::prelude::InteractionGroups;
use bevy_rapier3d::{parry, prelude::*};

use bevy_tnua_physics_integration_layer::TnuaPipelineSystems;
use bevy_tnua_physics_integration_layer::TnuaSystems;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaGravity;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaToggle;
use bevy_tnua_physics_integration_layer::data_for_backends::{TnuaGhostPlatform, TnuaNotPlatform};
use bevy_tnua_physics_integration_layer::data_for_backends::{TnuaGhostSensor, TnuaSensorOf};
use bevy_tnua_physics_integration_layer::data_for_backends::{
    TnuaMotor, TnuaProximitySensor, TnuaProximitySensorOutput, TnuaRigidBodyTracker,
};
use bevy_tnua_physics_integration_layer::obstacle_radar::TnuaObstacleRadar;
pub use spatial_ext::TnuaSpatialExtRapier3d;

use self::helpers::PretendToBeRapierContext;

pub mod prelude {
    pub use crate::{TnuaRapier3dPlugin, TnuaRapier3dSensorShape, TnuaSpatialExtRapier3d};
}

/// Add this plugin to use bevy_rapier2d as a physics backend.
///
/// This plugin should be used in addition to `TnuaControllerPlugin`, and both plugins must use the
/// same schedule - which should match the schedule Rapier runs in. By default, Rapier runs in
/// [`PostUpdate`] - which means this plugin and `TnuaControllerPlugin` should run in [`Update`].
pub struct TnuaRapier3dPlugin {
    schedule: InternedScheduleLabel,
}

impl TnuaRapier3dPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Plugin for TnuaRapier3dPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<TnuaMotor, Velocity>()
            .register_required_components::<TnuaMotor, ExternalForce>()
            .register_required_components::<TnuaMotor, ReadMassProperties>()
            .register_required_components_with::<TnuaGravity, GravityScale>(|| GravityScale(0.0));
        app.configure_sets(
            self.schedule,
            TnuaSystems.before(PhysicsSet::SyncBackend).run_if(
                |rapier_config: Single<&RapierConfiguration>| rapier_config.physics_pipeline_active,
            ),
        );
        app.add_systems(
            self.schedule,
            (
                update_rigid_body_trackers_system,
                update_proximity_sensors_system,
                update_obstacle_radars_system,
            )
                .in_set(TnuaPipelineSystems::Sensors),
        );
        app.add_systems(
            self.schedule,
            apply_motors_system.in_set(TnuaPipelineSystems::Motors),
        );
    }
}

/// Add this component to make [`TnuaProximitySensor`] cast a shape instead of a ray.
///
/// The [`SharedShape`](parry::shape::SharedShape) can be constructed using the re-exported
/// [`bevy_rapier3d::parry`], or by constructing a [`Collider`] first and taking it's
/// [`raw`](Collider::raw) field.
#[derive(Component)]
pub struct TnuaRapier3dSensorShape(pub parry::shape::SharedShape);

#[allow(clippy::type_complexity)]
fn update_rigid_body_trackers_system(
    rapier_config: Single<&RapierConfiguration>,
    mut query: Query<(
        &GlobalTransform,
        &Velocity,
        &mut TnuaRigidBodyTracker,
        Option<&TnuaToggle>,
        Option<&TnuaGravity>,
    )>,
) {
    for (transform, velocity, mut tracker, tnua_toggle, tnua_gravity) in query.iter_mut() {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        *tracker = TnuaRigidBodyTracker {
            translation,
            rotation,
            velocity: velocity.linvel,
            angvel: velocity.angvel,
            gravity: tnua_gravity.map(|g| g.0).unwrap_or(rapier_config.gravity),
        };
    }
}

pub(crate) fn get_collider(
    rapier_colliders: &RapierContextColliders,
    entity: Entity,
) -> Option<&rapier::geometry::Collider> {
    let collider_handle = rapier_colliders.entity2collider().get(&entity)?;
    rapier_colliders.colliders.get(*collider_handle)
}

#[allow(clippy::type_complexity)]
fn update_proximity_sensors_system(
    rapier_context_query: Query<PretendToBeRapierContext>,
    mut sensor_query: Query<(
        &mut TnuaProximitySensor,
        &TnuaSensorOf,
        Option<&TnuaRapier3dSensorShape>,
        Option<&mut TnuaGhostSensor>,
    )>,
    owner_query: Query<(
        &RapierContextEntityLink,
        &GlobalTransform,
        Option<&TnuaRapier3dSensorShape>,
        Option<&TnuaToggle>,
    )>,
    ghost_platforms_query: Query<(), With<TnuaGhostPlatform>>,
    not_platform_query: Query<(), With<TnuaNotPlatform>>,
    other_object_query: Query<(&GlobalTransform, &Velocity)>,
) {
    sensor_query.par_iter_mut().for_each(
        |(mut sensor, &TnuaSensorOf(owner_entity), shape, mut ghost_sensor)| {
            let Ok((rapier_context_entity_link, transform, owner_shape, tnua_toggle)) =
                owner_query.get(owner_entity)
            else {
                return;
            };
            let shape = shape.or(owner_shape);
            match tnua_toggle.copied().unwrap_or_default() {
                TnuaToggle::Disabled => return,
                TnuaToggle::SenseOnly => {}
                TnuaToggle::Enabled => {}
            }

            let Ok(rapier_context) = rapier_context_query.get(rapier_context_entity_link.0) else {
                return;
            };

            let cast_origin = transform.transform_point(sensor.cast_origin);
            let cast_direction = sensor.cast_direction;

            struct CastResult {
                entity: Entity,
                proximity: f32,
                intersection_point: Vec3,
                normal: Dir3,
            }

            let mut query_filter = QueryFilter::new().exclude_rigid_body(owner_entity);
            let owner_solver_groups: InteractionGroups;

            let owner_collider = get_collider(rapier_context.colliders, owner_entity);
            if let Some(owner_collider) = owner_collider {
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
                    if not_platform_query.contains(other_entity) {
                        return false;
                    }
                    if let Some(other_collider) =
                        get_collider(rapier_context.colliders, other_entity)
                    {
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
                    true
                };
                let query_filter = query_filter.predicate(&predicate);
                let cast_origin = cast_origin + cast_range_skip * *cast_direction;
                let cast_range = sensor.cast_range - cast_range_skip;
                rapier_context.with_query_pipeline(query_filter, |query_pipeline| {
                    if let Some(TnuaRapier3dSensorShape(shape)) = shape {
                        // TODO: can I bake `owner_rotation` into
                        // `sensor.cast_shape_rotation`?
                        let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
                        let owner_rotation = Quat::from_scaled_axis(
                            owner_rotation.to_scaled_axis().dot(*cast_direction) * *cast_direction,
                        );
                        query_pipeline
                            .cast_shape(
                                cast_origin,
                                owner_rotation.mul_quat(sensor.cast_shape_rotation),
                                *cast_direction,
                                shape.as_ref(),
                                ShapeCastOptions {
                                    max_time_of_impact: cast_range,
                                    target_distance: 0.0,
                                    stop_at_penetration: false,
                                    compute_impact_geometry_on_penetration: false,
                                },
                            )
                            .and_then(|(entity, hit)| {
                                let details = hit.details?;
                                Some(CastResult {
                                    entity,
                                    proximity: hit.time_of_impact,
                                    intersection_point: details.witness1,
                                    normal: Dir3::new(details.normal1)
                                        .unwrap_or_else(|_| -cast_direction),
                                })
                            })
                    } else {
                        query_pipeline
                            .cast_ray_and_get_normal(
                                cast_origin,
                                *cast_direction,
                                cast_range,
                                false,
                            )
                            .map(|(entity, hit)| CastResult {
                                entity,
                                proximity: hit.time_of_impact,
                                intersection_point: hit.point,
                                normal: Dir3::new(hit.normal).unwrap_or_else(|_| -cast_direction),
                            })
                    }
                })
            };

            let mut cast_range_skip = 0.0;
            if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                ghost_sensor.0.clear();
            }
            let isometry: rapier::na::Isometry3<f32> = {
                let (_, rotation, translation) = transform.to_scale_rotation_translation();
                (translation, rotation).into()
            };
            sensor.output = 'sensor_output: loop {
                if let Some(CastResult {
                    entity,
                    proximity,
                    intersection_point,
                    normal,
                }) = do_cast(cast_range_skip, &already_visited_ghost_entities)
                {
                    // Alternative fix for https://github.com/idanarye/bevy-tnua/issues/14 - one
                    // that does not cause https://github.com/idanarye/bevy-tnua/issues/85
                    // Note that this does not solve https://github.com/idanarye/bevy-tnua/issues/87
                    if let Some(owner_collider) = owner_collider
                        && owner_collider
                            .shape()
                            .contains_point(&isometry, &intersection_point.into())
                    {
                        // I hate having to do this so much, but without it it sometimes enters an
                        // infinte loop...
                        cast_range_skip = proximity
                            + if sensor.cast_range.is_finite() && 0.0 < sensor.cast_range {
                                0.1 * sensor.cast_range
                            } else {
                                0.1
                            };
                        continue;
                    }

                    let entity_linvel;
                    let entity_angvel;
                    if let Ok((entity_transform, entity_velocity)) = other_object_query.get(entity)
                    {
                        entity_angvel = entity_velocity.angvel;
                        entity_linvel = entity_velocity.linvel
                            + if 0.0 < entity_angvel.length_squared() {
                                let relative_point =
                                    intersection_point - entity_transform.translation();
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
                        normal,
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

fn update_obstacle_radars_system(
    rapier_world_query: Query<(PretendToBeRapierContext, &RapierConfiguration)>,
    mut radars_query: Query<(
        Entity,
        &RapierContextEntityLink,
        &mut TnuaObstacleRadar,
        &GlobalTransform,
    )>,
) {
    if radars_query.is_empty() {
        return;
    }
    for (radar_owner_entity, rapier_context_entity_link, mut radar, radar_transform) in
        radars_query.iter_mut()
    {
        let Ok((rapier_context, rapier_config)) =
            rapier_world_query.get(rapier_context_entity_link.0)
        else {
            continue;
        };
        let (_radar_scale, radar_rotation, radar_translation) =
            radar_transform.to_scale_rotation_translation();
        radar.pre_marking_update(
            radar_owner_entity,
            radar_translation,
            Dir3::new(rapier_config.gravity).unwrap_or(Dir3::Y),
        );
        rapier_context.with_query_pipeline(Default::default(), |query_pipeline| {
            for obstacle_entity in query_pipeline.intersect_shape(
                radar_translation,
                radar_rotation,
                &parry::shape::Cylinder::new(0.5 * radar.height, radar.radius),
            ) {
                if radar_owner_entity == obstacle_entity {
                    continue;
                }
                radar.mark_seen(obstacle_entity);
            }
        });
    }
}

#[allow(clippy::type_complexity)]
fn apply_motors_system(
    mut query: Query<(
        &TnuaMotor,
        &mut Velocity,
        &ReadMassProperties,
        &mut ExternalForce,
        Option<&TnuaToggle>,
        Option<&TnuaGravity>,
    )>,
) {
    for (motor, mut velocity, mass_properties, mut external_force, tnua_toggle, tnua_gravity) in
        query.iter_mut()
    {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled | TnuaToggle::SenseOnly => {
                *external_force = Default::default();
                return;
            }
            TnuaToggle::Enabled => {}
        }
        if motor.lin.boost.is_finite() {
            velocity.linvel += motor.lin.boost;
        }
        if motor.lin.acceleration.is_finite() {
            external_force.force = motor.lin.acceleration * mass_properties.get().mass;
        }
        if motor.ang.boost.is_finite() {
            velocity.angvel += motor.ang.boost;
        }
        if motor.ang.acceleration.is_finite() {
            external_force.torque =
                motor.ang.acceleration * mass_properties.get().principal_inertia;
        }
        if let Some(gravity) = tnua_gravity {
            external_force.force += gravity.0 * mass_properties.get().mass;
        }
    }
}
