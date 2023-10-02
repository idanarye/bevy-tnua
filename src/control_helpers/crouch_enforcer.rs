use std::any::Any;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use crate::controller::TnuaController;
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
    modify_sensor: Box<dyn Send + Sync + Fn(&mut EntityCommands)>,
    enforced_action: Option<(Box<dyn DynamicCrouchEnforcedAction>, bool)>,
    currently_enforcing: bool,
}

impl TnuaCrouchEnforcer {
    pub fn new(
        offset: Vec3,
        modify_sensor: impl 'static + Send + Sync + Fn(&mut EntityCommands),
    ) -> Self {
        Self {
            sensor_entity: None,
            offset,
            modify_sensor: Box::new(modify_sensor),
            enforced_action: None,
            currently_enforcing: false,
        }
    }

    pub fn enforcing<A: TnuaCrouchEnforcedAction>(&mut self, mut crouch_action: A) -> A {
        if let Some((enforced_action, fed_this_frame)) = self.enforced_action.as_mut() {
            if enforced_action.overwrite(&crouch_action).is_ok() {
                *fed_this_frame = true;
                if self.currently_enforcing {
                    crouch_action.prevent_cancellation();
                }
                return crouch_action;
            }
        }
        self.enforced_action = Some((
            Box::new(BoxableCrouchEnforcedAction(crouch_action.clone())),
            true,
        ));
        if self.currently_enforcing {
            crouch_action.prevent_cancellation();
        }
        crouch_action
    }
}

pub trait TnuaCrouchEnforcedAction: TnuaAction + Clone {
    fn range_to_cast_up(&self, state: &Self::State) -> f32;
    fn prevent_cancellation(&mut self);
}

trait DynamicCrouchEnforcedAction: Send + Sync {
    fn overwrite(&mut self, value: &dyn Any) -> Result<(), ()>;
    fn feed_to_controller(&mut self, controller: &mut TnuaController);
    fn range_to_cast_up(&self, controller: &TnuaController) -> Option<f32>;
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

    fn feed_to_controller(&mut self, controller: &mut TnuaController) {
        let mut action = self.0.clone();
        action.prevent_cancellation();
        controller.action(action);
    }

    fn range_to_cast_up(&self, controller: &TnuaController) -> Option<f32> {
        if let Some((action, state)) = controller.action_and_state::<A>() {
            Some(action.range_to_cast_up(state))
        } else {
            None
        }
    }
}

fn update_crouch_enforcer(
    mut query: Query<(Entity, &mut TnuaController, &mut TnuaCrouchEnforcer)>,
    mut sensors_query: Query<&mut TnuaProximitySensor, With<TnuaSubservientSensor>>,
    mut commands: Commands,
) {
    for (owner_entity, mut controller, mut crouch_enforcer) in query.iter_mut() {
        let set_sensor: Option<f32>;
        if let Some((enforced_action, fed_this_frame)) = crouch_enforcer.enforced_action.as_mut() {
            if *fed_this_frame {
                set_sensor = enforced_action.range_to_cast_up(controller.as_mut());
                *fed_this_frame = false;
            } else {
                set_sensor = None;
                crouch_enforcer.enforced_action = None;
            }
        } else {
            set_sensor = None;
        }

        if let Some(set_sensor) = set_sensor {
            if let Some(mut subservient_sensor) = crouch_enforcer
                .sensor_entity
                .and_then(|entity| sensors_query.get_mut(entity).ok())
            {
                subservient_sensor.cast_origin = crouch_enforcer.offset;
                subservient_sensor.cast_range = set_sensor;
            } else {
                let mut cmd = commands.spawn((
                    TransformBundle {
                        ..Default::default()
                    },
                    TnuaSubservientSensor { owner_entity },
                    TnuaProximitySensor {
                        cast_direction: Vec3::Y,
                        cast_origin: crouch_enforcer.offset,
                        cast_range: set_sensor,
                        ..Default::default()
                    },
                ));
                cmd.set_parent(owner_entity);
                (crouch_enforcer.modify_sensor)(&mut cmd);
                let sensor_entity = cmd.id();
                crouch_enforcer.sensor_entity = Some(sensor_entity);
            }
        } else if let Some(mut subservient_sensor) = crouch_enforcer
            .sensor_entity
            .and_then(|entity| sensors_query.get_mut(entity).ok())
        {
            // Turn it off
            subservient_sensor.cast_range = 0.0;
        }
        if let Some((enforced_action, fed_this_frame)) =
            crouch_enforcer.sensor_entity.and_then(|entity| {
                if sensors_query.get_mut(entity).ok()?.output.is_some() {
                    crouch_enforcer.enforced_action.as_mut()
                } else {
                    None
                }
            })
        {
            enforced_action.feed_to_controller(controller.as_mut());
            *fed_this_frame = true;
            crouch_enforcer.currently_enforcing = true;
        } else {
            crouch_enforcer.currently_enforcing = false;
        }
    }
}
