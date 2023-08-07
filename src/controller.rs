use bevy::prelude::*;
use bevy::utils::{Entry, HashMap};

use crate::basis_action_traits::{
    BoxableAction, BoxableBasis, DynamicAction, DynamicBasis, TnuaAction, TnuaActionContext,
    TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, TnuaBasisContext,
};
use crate::{
    TnuaBasis, TnuaMotor, TnuaPipelineStages, TnuaProximitySensor, TnuaRigidBodyTracker,
    TnuaSystemSet, TnuaUserControlsSystemSet,
};

pub struct TnuaPlatformerPlugin2;

impl Plugin for TnuaPlatformerPlugin2 {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                TnuaPipelineStages::Sensors,
                TnuaPipelineStages::SubservientSensors,
                TnuaUserControlsSystemSet,
                TnuaPipelineStages::Logic,
                TnuaPipelineStages::Motors,
            )
                .chain()
                .in_set(TnuaSystemSet),
        );
        app.add_systems(
            Update,
            apply_controller_system.in_set(TnuaPipelineStages::Logic),
        );
        //app.add_systems(
        //Update,
        //handle_keep_crouching_below_obstacles.in_set(TnuaPipelineStages::SubservientSensors),
        //);
    }
}

#[derive(Component, Default)]
pub struct TnuaController {
    current_basis: Option<(&'static str, Box<dyn DynamicBasis>)>,
    actions_being_fed: HashMap<&'static str, bool>,
    current_action: Option<(&'static str, Box<dyn DynamicAction>)>,
    contender_action: Option<(&'static str, Box<dyn DynamicAction>)>,
}

impl TnuaController {
    pub fn basis<B: TnuaBasis>(&mut self, name: &'static str, basis: B) -> &mut Self {
        if let Some((existing_name, existing_basis)) =
            self.current_basis.as_mut().and_then(|(n, b)| {
                let b = b.as_mut_any().downcast_mut::<BoxableBasis<B>>()?;
                Some((n, b))
            })
        {
            *existing_name = name;
            existing_basis.input = basis;
        } else {
            self.current_basis = Some((name, Box::new(BoxableBasis::new(basis))));
        }
        self
    }

    pub fn action<A: TnuaAction>(&mut self, name: &'static str, action: A) -> &mut Self {
        match self.actions_being_fed.entry(name) {
            Entry::Occupied(mut entry) => {
                *entry.get_mut() = true;
                if let Some((current_name, current_action)) = self.current_action.as_mut() {
                    if *current_name == name {
                        let Some(current_action) = current_action.as_mut_any().downcast_mut::<BoxableAction<A>>() else {
                            panic!("Multiple action types registered with same name {name:?}");
                        };
                        current_action.input = action;
                    } else {
                        // different action is running - will not override because button was
                        // already pressed.
                    }
                } else {
                    // different action is running - will not set because button was already
                    // pressed.
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(true);
                if let Some(contender_action) = self.contender_action.as_mut().and_then(|(contender_name, contender_action)| {
                    if *contender_name == name {
                        let Some(contender_action) = contender_action.as_mut_any().downcast_mut::<BoxableAction<A>>() else {
                            panic!("Multiple action types registered with same name {name:?}");
                        };
                        Some(contender_action)
                    } else {
                        None
                    }
                }) {
                    contender_action.input = action;
                } else {
                    self.contender_action = Some((name, Box::new(BoxableAction::new(action))));
                }
            }
        }
        self
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system(
    time: Res<Time>,
    mut query: Query<(
        &mut TnuaController,
        &TnuaRigidBodyTracker,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
    )>,
) {
    let frame_duration = time.delta().as_secs_f32();
    if frame_duration == 0.0 {
        return;
    }
    for (mut controller, tracker, mut sensor, mut motor) in query.iter_mut() {
        let controller = controller.as_mut();

        if let Some((_, basis)) = controller.current_basis.as_mut() {
            basis.apply(
                TnuaBasisContext {
                    frame_duration,
                    tracker,
                    proximity_sensor: sensor.as_ref(),
                },
                motor.as_mut(),
            );
            let sensor_cast_range = basis.proximity_sensor_cast_range();
            sensor.cast_range = sensor_cast_range;

            if let Some((name, current_action)) = controller.current_action.as_mut() {
                let lifecycle_status = if controller.contender_action.is_some() {
                    TnuaActionLifecycleStatus::CancelledInto
                } else if controller
                    .actions_being_fed
                    .get(name)
                    .copied()
                    .unwrap_or(false)
                {
                    TnuaActionLifecycleStatus::StillFed
                } else {
                    TnuaActionLifecycleStatus::NoLongerFed
                };

                let directive = current_action.apply(
                    TnuaActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor: sensor.as_ref(),
                        basis: basis.as_ref(),
                    },
                    lifecycle_status,
                    motor.as_mut(),
                );
                match directive {
                    TnuaActionLifecycleDirective::StillActive => {}
                    TnuaActionLifecycleDirective::Finished => {
                        controller.current_action =
                            if let Some((contender_name, mut contender_action)) =
                                controller.contender_action.take()
                            {
                                let contender_directive = contender_action.apply(
                                    TnuaActionContext {
                                        frame_duration,
                                        tracker,
                                        proximity_sensor: sensor.as_ref(),
                                        basis: basis.as_ref(),
                                    },
                                    TnuaActionLifecycleStatus::CancelledFrom,
                                    motor.as_mut(),
                                );
                                match contender_directive {
                                    TnuaActionLifecycleDirective::StillActive => {
                                        Some((contender_name, contender_action))
                                    }
                                    TnuaActionLifecycleDirective::Finished => None,
                                }
                            } else {
                                None
                            };
                    }
                }
            } else if let Some((contender_name, mut contender_action)) =
                controller.contender_action.take()
            {
                contender_action.apply(
                    TnuaActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor: sensor.as_ref(),
                        basis: basis.as_ref(),
                    },
                    TnuaActionLifecycleStatus::Initiated,
                    motor.as_mut(),
                );
                controller.current_action = Some((contender_name, contender_action));
            }
        }

        // Cycle actions_being_fed
        controller
            .actions_being_fed
            .retain(|_, triggered_this_frame| {
                if *triggered_this_frame {
                    *triggered_this_frame = false;
                    true
                } else {
                    false
                }
            });
    }
}
