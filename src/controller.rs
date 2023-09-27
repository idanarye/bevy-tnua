use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy::utils::{Entry, HashMap};

use crate::basis_action_traits::{
    BoxableAction, BoxableBasis, DynamicAction, DynamicBasis, TnuaAction, TnuaActionContext,
    TnuaActionInitiationDirective, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus,
    TnuaBasisContext,
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
    contender_action: Option<(&'static str, Box<dyn DynamicAction>, Stopwatch)>,
}

impl TnuaController {
    pub fn named_basis<B: TnuaBasis>(&mut self, name: &'static str, basis: B) -> &mut Self {
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

    pub fn basis<B: TnuaBasis>(&mut self, basis: B) -> &mut Self {
        self.named_basis(B::NAME, basis)
    }

    pub fn basis_name(&self) -> &'static str {
        if let Some((basis_name, _)) = self.current_basis {
            basis_name
        } else {
            ""
        }
    }

    pub fn basis_and_state<B: TnuaBasis>(&self) -> Option<(&B, &B::State)> {
        let (_, basis) = self.current_basis.as_ref()?;
        let boxable_basis: &BoxableBasis<B> = basis.as_any().downcast_ref()?;
        Some((&boxable_basis.input, &boxable_basis.state))
    }

    pub fn named_action<A: TnuaAction>(&mut self, name: &'static str, action: A) -> &mut Self {
        match self.actions_being_fed.entry(name) {
            Entry::Occupied(mut entry) => {
                *entry.get_mut() = true;
                if let Some((current_name, current_action)) = self.current_action.as_mut() {
                    if *current_name == name {
                        let Some(current_action) = current_action
                            .as_mut_any()
                            .downcast_mut::<BoxableAction<A>>()
                        else {
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
                if let Some(contender_action) = self.contender_action.as_mut().and_then(
                    |(contender_name, contender_action, _)| {
                        if *contender_name == name {
                            let Some(contender_action) = contender_action
                                .as_mut_any()
                                .downcast_mut::<BoxableAction<A>>()
                            else {
                                panic!("Multiple action types registered with same name {name:?}");
                            };
                            Some(contender_action)
                        } else {
                            None
                        }
                    },
                ) {
                    contender_action.input = action;
                } else {
                    self.contender_action =
                        Some((name, Box::new(BoxableAction::new(action)), Stopwatch::new()));
                }
            }
        }
        self
    }

    pub fn action<A: TnuaAction>(&mut self, action: A) -> &mut Self {
        self.named_action(A::NAME, action)
    }

    pub fn action_name(&self) -> &'static str {
        if let Some((action_name, _)) = self.current_action {
            action_name
        } else {
            ""
        }
    }

    pub fn action_and_state<A: TnuaAction>(&self) -> Option<(&A, &A::State)> {
        let (_, action) = self.current_action.as_ref()?;
        let boxable_action: &BoxableAction<A> = action.as_any().downcast_ref()?;
        Some((&boxable_action.input, &boxable_action.state))
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

            // To streamline TnuaActionContext creation
            let proximity_sensor = sensor.as_ref();
            let basis = basis.as_ref();

            let has_valid_contender = if let Some((_, contender_action, being_fed_for)) =
                &mut controller.contender_action
            {
                let initiation_decision = contender_action.initiation_decision(
                    TnuaActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor,
                        basis,
                    },
                    being_fed_for,
                );
                being_fed_for.tick(time.delta());
                match initiation_decision {
                    TnuaActionInitiationDirective::Reject => {
                        controller.contender_action = None;
                        false
                    }
                    TnuaActionInitiationDirective::Delay => false,
                    TnuaActionInitiationDirective::Allow => true,
                }
            } else {
                false
            };

            if let Some((name, current_action)) = controller.current_action.as_mut() {
                let lifecycle_status = if has_valid_contender {
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
                        proximity_sensor,
                        basis,
                    },
                    lifecycle_status,
                    motor.as_mut(),
                );
                match directive {
                    TnuaActionLifecycleDirective::StillActive => {}
                    TnuaActionLifecycleDirective::Finished => {
                        controller.current_action = if has_valid_contender {
                            let (contender_name, mut contender_action, _) = controller.contender_action.take().expect("has_valid_contender can only be true if contender_action is Some");
                            let contender_directive = contender_action.apply(
                                TnuaActionContext {
                                    frame_duration,
                                    tracker,
                                    proximity_sensor,
                                    basis,
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
            } else if has_valid_contender {
                let (contender_name, mut contender_action, _) = controller
                    .contender_action
                    .take()
                    .expect("has_valid_contender can only be true if contender_action is Some");
                contender_action.apply(
                    TnuaActionContext {
                        frame_duration,
                        tracker,
                        proximity_sensor,
                        basis,
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
