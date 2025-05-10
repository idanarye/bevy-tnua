use std::any::Any;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::math::{Float, Vector3};

use crate::controller::TnuaController;
use crate::subservient_sensors::TnuaSubservientSensor;
use crate::{TnuaAction, TnuaPipelineStages, TnuaProximitySensor};

pub struct TnuaCrouchEnforcerPlugin {
    schedule: InternedScheduleLabel,
}

impl TnuaCrouchEnforcerPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Default for TnuaCrouchEnforcerPlugin {
    fn default() -> Self {
        Self::new(Update)
    }
}

/// A plugin required for making [`TnuaCrouchEnforcer`] work.
impl Plugin for TnuaCrouchEnforcerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            self.schedule,
            update_crouch_enforcer.in_set(TnuaPipelineStages::SubservientSensors),
        );
    }
}

/// Prevents the character from standing up if the player stops feeding a crouch action (like
/// [`TnuaBuiltinCrouch`](crate::builtins::TnuaBuiltinCrouch)) while under an obstacle.
///
/// This will create a child entity with a proximity sensor pointed upward. When that sensor senses
/// a ceiling, it will force feed the action even if it is no longer fed by the game code into the
/// controller (which would happen if the player releases the crouch button)
///
/// Using it requires three things:
///
/// 1. Adding the plugin [`TnuaCrouchEnforcerPlugin`].
/// 2. Adding [`TnuaCrouchEnforcer`] as a component to the character entity.
/// 2. Passing the crouch action through the component's
///    [`enforcing`](TnuaCrouchEnforcer::enforcing) method:
///     ```no_run
///     # use bevy_tnua::prelude::*;
///     # use bevy_tnua::builtins::TnuaBuiltinCrouch;
///     # use bevy_tnua::control_helpers::TnuaCrouchEnforcer;
///     # let mut controller = TnuaController::default();
///     # let mut crouch_enforcer = TnuaCrouchEnforcer::new(Default::default(), |_| {});
///     controller.action(crouch_enforcer.enforcing(TnuaBuiltinCrouch {
///         float_offset: -0.9,
///         ..Default::default()
///     }));
///     ```
#[derive(Component)]
pub struct TnuaCrouchEnforcer {
    sensor_entity: Option<Entity>,
    offset: Vector3,
    modify_sensor: Box<dyn Send + Sync + Fn(&mut EntityCommands)>,
    enforced_action: Option<(Box<dyn DynamicCrouchEnforcedAction>, bool)>,
    currently_enforcing: bool,
}

impl TnuaCrouchEnforcer {
    /// Create a new crouch enforcer, to be added as a component to the entity that the crouch
    /// action will be fed to.
    ///
    /// # Arguments:
    ///
    /// * `offset` - the origin of the proximity sensor used to determine if the character needs to
    ///   crouch. Should be placed at the top of the collider. The sensor is always pointed
    ///   upwards.
    /// * `modify_sensor` - a function called with the command that creates the sensor. This
    ///   function has the opportunity to add things to the sensor entity - mostly cast-shape
    ///   components.
    pub fn new(
        offset: Vector3,
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

/// An action that can be enforced by [`TnuaCrouchEnforcer`].
pub trait TnuaCrouchEnforcedAction: TnuaAction + Clone {
    /// The range, from the sensor's offset (as set by [`TnuaCrouchEnforcer::new`]), to check for a
    /// ceiling. If the sensor finds anything within that range - the crouch will be enforced.
    fn range_to_cast_up(&self, state: &Self::State) -> Float;

    /// Modify the action so that it won't be cancellable by another action.
    fn prevent_cancellation(&mut self);
}

trait DynamicCrouchEnforcedAction: Send + Sync {
    fn overwrite(&mut self, value: &dyn Any) -> Result<(), ()>;
    fn feed_to_controller(&mut self, controller: &mut TnuaController);
    fn range_to_cast_up(&self, controller: &TnuaController) -> Option<Float>;
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

    fn range_to_cast_up(&self, controller: &TnuaController) -> Option<Float> {
        if let Some((action, state)) = controller.concrete_action::<A>() {
            Some(action.range_to_cast_up(state))
        } else {
            None
        }
    }
}

fn update_crouch_enforcer(
    mut query: Query<(Entity, &mut TnuaController, &mut TnuaCrouchEnforcer)>,
    mut sensors_query: Query<(&mut TnuaProximitySensor, Has<TnuaSubservientSensor>)>,
    mut commands: Commands,
) {
    for (owner_entity, mut controller, mut crouch_enforcer) in query.iter_mut() {
        struct SetSensor {
            cast_direction: Dir3,
            cast_range: Float,
        }
        let set_sensor: Option<SetSensor>;
        if let Some((enforced_action, fed_this_frame)) = crouch_enforcer.enforced_action.as_mut() {
            if *fed_this_frame {
                set_sensor = enforced_action
                    .range_to_cast_up(controller.as_mut())
                    .and_then(|cast_range| {
                        let (main_sensor, _) = sensors_query.get(owner_entity).ok()?;
                        Some(SetSensor {
                            cast_direction: -main_sensor.cast_direction,
                            cast_range,
                        })
                    });
                *fed_this_frame = false;
            } else {
                set_sensor = None;
                crouch_enforcer.enforced_action = None;
            }
        } else {
            set_sensor = None;
        }

        if let Some(SetSensor {
            cast_direction,
            cast_range,
        }) = set_sensor
        {
            if let Some((mut subservient_sensor, true)) = crouch_enforcer
                .sensor_entity
                .and_then(|entity| sensors_query.get_mut(entity).ok())
            {
                subservient_sensor.cast_origin = crouch_enforcer.offset;
                subservient_sensor.cast_direction = cast_direction;
                subservient_sensor.cast_range = cast_range;
            } else {
                let mut cmd = commands.spawn((
                    Transform::default(),
                    TnuaSubservientSensor { owner_entity },
                    TnuaProximitySensor {
                        cast_origin: crouch_enforcer.offset,
                        cast_direction,
                        cast_range,
                        ..Default::default()
                    },
                ));
                cmd.insert(ChildOf(owner_entity));
                (crouch_enforcer.modify_sensor)(&mut cmd);
                let sensor_entity = cmd.id();
                crouch_enforcer.sensor_entity = Some(sensor_entity);
            }
        } else if let Some((mut subservient_sensor, true)) = crouch_enforcer
            .sensor_entity
            .and_then(|entity| sensors_query.get_mut(entity).ok())
        {
            // Turn it off
            subservient_sensor.cast_range = 0.0;
        }
        if let Some((enforced_action, fed_this_frame)) =
            crouch_enforcer.sensor_entity.and_then(|entity| {
                let Ok((sensor, true)) = sensors_query.get_mut(entity) else {
                    return None;
                };
                if sensor.output.is_some() {
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
