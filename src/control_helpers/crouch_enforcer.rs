use std::any::Any;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use crate::builtins::TnuaBuiltinCrouch;
use crate::subservient_sensors::TnuaSubservientSensor;
use crate::{TnuaAction, TnuaPipelineStages, TnuaProximitySensor};

pub struct TnuaCrouchEnforcerPlugin;

impl Plugin for TnuaCrouchEnforcerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_crouch_enforcer.in_set(TnuaPipelineStages::SubservientSensors),
        );
    }
}

#[derive(Component)]
pub struct TnuaCrouchEnforcer {
    sensor_entity: Option<Entity>,
    offset: Vec3,
    detection_height: f32,
    modify_sensor: Box<dyn Send + Sync + Fn(&mut EntityCommands)>,
    enforced_action: Option<(Box<dyn DynamicCrouchEnforcedAction>, bool)>,
}

impl TnuaCrouchEnforcer {
    pub fn new(
        offset: Vec3,
        modify_sensor: impl 'static + Send + Sync + Fn(&mut EntityCommands),
    ) -> Self {
        Self {
            sensor_entity: None,
            offset,
            detection_height: 0.0,
            modify_sensor: Box::new(modify_sensor),
            enforced_action: None,
        }
    }

    pub fn enforcing<A: TnuaCrouchEnforcedAction>(&mut self, crouch_action: A) -> A {
        if let Some((enforced_action, fed_this_frame)) = self.enforced_action.as_mut() {
            if enforced_action.overwrite(&crouch_action).is_ok() {
                *fed_this_frame = true;
                return crouch_action;
            }
        }
        self.enforced_action = Some((
            Box::new(BoxableCrouchEnforcedAction(crouch_action.clone())),
            true,
        ));
        crouch_action
    }
}

pub trait TnuaCrouchEnforcedAction: TnuaAction + Clone {}

impl TnuaCrouchEnforcedAction for TnuaBuiltinCrouch {}

trait DynamicCrouchEnforcedAction: Send + Sync {
    fn overwrite(&mut self, value: &dyn Any) -> Result<(), ()>;
}

struct BoxableCrouchEnforcedAction<A: TnuaCrouchEnforcedAction>(A);

impl<A: TnuaCrouchEnforcedAction> DynamicCrouchEnforcedAction for BoxableCrouchEnforcedAction<A> {
    fn overwrite(&mut self, value: &dyn Any) -> Result<(), ()> {
        if let Some(concrete) = value.downcast_ref::<A>() {
            self.0 = concrete.clone();
            Ok(())
        } else {
            Err(())
        }
    }
}

fn update_crouch_enforcer(
    mut query: Query<(Entity, &mut TnuaCrouchEnforcer)>,
    sensors_query: Query<(Entity, &TnuaProximitySensor), With<TnuaSubservientSensor>>,
    mut commands: Commands,
) {
    for (owner_entity, mut crouch_enforcer) in query.iter_mut() {
        if let Some((_enforced_action, fed_this_frame)) = crouch_enforcer.enforced_action.as_mut() {
            if *fed_this_frame {
                // TODO: enforce the action
                *fed_this_frame = false;
            } else {
                crouch_enforcer.enforced_action = None;
            }
        }
        crouch_enforcer.enforced_action = None;
        if let Some((subservient_entity, subservient_sensor)) = crouch_enforcer
            .sensor_entity
            .and_then(|entity| sensors_query.get(entity).ok())
        {
            if subservient_sensor.output.is_some() {
                commands
                    .entity(subservient_entity)
                    .add(|entity, world: &mut World| {
                        if let Some(mut subservient_entity_accessor) = world.get_entity_mut(entity)
                        {
                            if let Some(subservient_sensor) =
                                subservient_entity_accessor.get_mut::<TnuaProximitySensor>()
                            {
                                info!("{:?}", subservient_sensor.cast_range);
                            }
                        }
                    });
                // crouch_enforcer.force_crouching_to_height = keep_crouching
                // .force_crouching_to_height
                // .min(controls.float_height_offset);
            } else {
                //crouch_enforcer keep_crouching.force_crouching_to_height = f32::INFINITY;
            }
        } else {
            let mut cmd = commands.spawn((
                TransformBundle {
                    ..Default::default()
                },
                TnuaSubservientSensor { owner_entity },
                TnuaProximitySensor {
                    cast_direction: Vec3::Y,
                    cast_origin: crouch_enforcer.offset,
                    cast_range: crouch_enforcer.detection_height,
                    ..Default::default()
                },
            ));
            cmd.set_parent(owner_entity);
            (crouch_enforcer.modify_sensor)(&mut cmd);
            let sensor_entity = cmd.id();
            crouch_enforcer.sensor_entity = Some(sensor_entity);
            // keep_crouching.force_crouching_to_height = f32::INFINITY;
        }
    }
}
