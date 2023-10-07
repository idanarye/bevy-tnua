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
    TnuaSystemSet, TnuaToggle, TnuaUserControlsSystemSet,
};

/// The main for supporting Tnua character controller.
///
/// Will not work without a physics backend plugin (like `TnuaRapier2dPlugin` or
/// `TnuaRapier3dPlugin`)
pub struct TnuaControllerPlugin;

impl Plugin for TnuaControllerPlugin {
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
    }
}

/// All the Tnua components needed to run a floating character controller.
///
/// Note that this bundle only contains components defined by Tnua. The components of the physics
/// backend that turn the entity into a dynamic rigid body must be added separately.
#[derive(Default, Bundle)]
pub struct TnuaControllerBundle {
    pub controller: TnuaController,
    pub motor: TnuaMotor,
    pub rigid_body_tracker: TnuaRigidBodyTracker,
    pub proximity_sensor: TnuaProximitySensor,
}

struct FedEntry {
    fed_this_frame: bool,
    rescheduled_in: Option<Timer>,
}

/// The main component used for interaction with the controls and animation code.
///
/// Every frame, the game code should feed input this component on every controlled entity. What
/// should be fed is:
///
/// * A basis - this is the main movement command - usually
///   [`TnuaBuiltinWalk`](crate::builtins::TnuaBuiltinWalk), but there can be others. It is the
///   game code's responsibility to ensure only one basis is fed at any given time, because basis
///   can hold state and replacing the basis type restarts the state.
///
///   Refer to the documentation of [the implementors of
///   `TnuaBasis`](crate::TnuaBasis#implementors) for more information.
///
/// * Zero or more actions - these are movements like jumping, dashing, crouching, etc. Multiple
///   actions can be fed, but only one can be active at any given moment. Unlike basis, there is a
///   smart mechanism for deciding which action to use and which to discard, so it is safe to feed
///   many actions at the same frame.
///
///   Refer to the documentation of [the implementors of
///   `TnuaAction`](crate::TnuaAction#implementors) for more information.
///
/// Without [`TnuaControllerPlugin`] this component will not do anything.
#[derive(Component, Default)]
pub struct TnuaController {
    current_basis: Option<(&'static str, Box<dyn DynamicBasis>)>,
    actions_being_fed: HashMap<&'static str, FedEntry>,
    current_action: Option<(&'static str, Box<dyn DynamicAction>)>,
    contender_action: Option<(&'static str, Box<dyn DynamicAction>, Stopwatch)>,
}

impl TnuaController {
    /// Feed a basis - the main movement command - with [its default name](TnuaBasis::NAME).
    pub fn basis<B: TnuaBasis>(&mut self, basis: B) -> &mut Self {
        self.named_basis(B::NAME, basis)
    }

    /// Feed a basis - the main movement command - with a custom name.
    ///
    /// This should only be used if the same basis type needs to be used with different names to
    /// allow, for example, different animations. Otherwise prefer to use the default name with
    /// [`basis`](Self::basis).
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

    /// Instruct the basis to pretend the user provided no input this frame.
    ///
    /// The exact meaning is defined in the basis' [`neutralize`](TnuaBasis::neutralize) method,
    /// but generally it means that fields that typically come from a configuration will not be
    /// touched, and only fields that are typically set by user input get nullified.
    pub fn neutralize_basis(&mut self) -> &mut Self {
        if let Some((_, basis)) = self.current_basis.as_mut() {
            basis.neutralize();
        }
        self
    }

    /// The name of the currently running basis.
    ///
    /// When using the basis with it's default name, prefer to match this against
    /// [`TnuaBasis::NAME`] and not against a string literal.
    pub fn basis_name(&self) -> Option<&'static str> {
        self.current_basis
            .as_ref()
            .map(|(basis_name, _)| *basis_name)
    }

    /// A dynamic accessor to the currently running basis.
    pub fn dynaimc_basis(&self) -> Option<&dyn DynamicBasis> {
        Some(self.current_basis.as_ref()?.1.as_ref())
    }

    /// The currently running basis, together with its state.
    ///
    /// This is mainly useful for animation. When multiple basis types are used in the game,
    /// [`basis_name`](Self::basis_name) be used to determine the type of the current basis first,
    /// to avoid having to try multiple downcasts.
    pub fn basis_and_state<B: TnuaBasis>(&self) -> Option<(&B, &B::State)> {
        let (_, basis) = self.current_basis.as_ref()?;
        let boxable_basis: &BoxableBasis<B> = basis.as_any().downcast_ref()?;
        Some((&boxable_basis.input, &boxable_basis.state))
    }

    /// Feed an action with [its default name](TnuaBasis::NAME).
    pub fn action<A: TnuaAction>(&mut self, action: A) -> &mut Self {
        self.named_action(A::NAME, action)
    }

    /// Feed an action with a custom name.
    ///
    /// This should only be used if the same action type needs to be used with different names to
    /// allow, for example, different animations. Otherwise prefer to use the default name with
    /// [`action`](Self::action).
    pub fn named_action<A: TnuaAction>(&mut self, name: &'static str, action: A) -> &mut Self {
        match self.actions_being_fed.entry(name) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().fed_this_frame = true;
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
                } else if self.contender_action.is_none()
                    && entry
                        .get()
                        .rescheduled_in
                        .as_ref()
                        .map_or(false, |timer| timer.finished())
                {
                    // no action is running - but this action is rescheduled and there is no
                    // already-existing contender that would have taken priority
                    self.contender_action =
                        Some((name, Box::new(BoxableAction::new(action)), Stopwatch::new()));
                } else {
                    // no action is running - will not set because button was already pressed.
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(FedEntry {
                    fed_this_frame: true,
                    rescheduled_in: None,
                });
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

    /// The name of the currently running action.
    ///
    /// When using an action with it's default name, prefer to match this against
    /// [`TnuaAction::NAME`] and not against a string literal.
    pub fn action_name(&self) -> Option<&'static str> {
        self.current_action
            .as_ref()
            .map(|(action_name, _)| *action_name)
    }

    /// The currently running action, together with its state.
    ///
    /// This is mainly useful for animation. When multiple action types are used in the game,
    /// [`action_name`](Self::action_name) be used to determine the type of the current action
    /// first, to avoid having to try multiple downcasts.
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
        Option<&TnuaToggle>,
    )>,
) {
    let frame_duration = time.delta().as_secs_f32();
    if frame_duration == 0.0 {
        return;
    }
    for (mut controller, tracker, mut sensor, mut motor, tnua_toggle) in query.iter_mut() {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }

        let controller = controller.as_mut();

        if let Some((_, basis)) = controller.current_basis.as_mut() {
            let basis = basis.as_mut();
            basis.apply(
                TnuaBasisContext {
                    frame_duration,
                    tracker,
                    proximity_sensor: sensor.as_ref(),
                },
                motor.as_mut(),
            );
            let sensor_cast_range_for_basis = basis.proximity_sensor_cast_range();

            // To streamline TnuaActionContext creation
            let proximity_sensor = sensor.as_ref();

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
                    .map(|fed_entry| fed_entry.fed_this_frame)
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
                if current_action.violates_coyote_time() {
                    basis.violate_coyote_time();
                }
                let reschedule_action =
                    |actions_being_fed: &mut HashMap<&'static str, FedEntry>,
                     after_seconds: f32| {
                        if let Some(fed_entry) = actions_being_fed.get_mut(name) {
                            fed_entry.rescheduled_in =
                                Some(Timer::from_seconds(after_seconds, TimerMode::Once));
                        }
                    };
                match directive {
                    TnuaActionLifecycleDirective::StillActive => {}
                    TnuaActionLifecycleDirective::Finished
                    | TnuaActionLifecycleDirective::Reschedule { .. } => {
                        if let TnuaActionLifecycleDirective::Reschedule { after_seconds } =
                            directive
                        {
                            reschedule_action(&mut controller.actions_being_fed, after_seconds);
                        }
                        controller.current_action = if has_valid_contender {
                            let (contender_name, mut contender_action, _) = controller.contender_action.take().expect("has_valid_contender can only be true if contender_action is Some");
                            if let Some(contender_fed_entry) =
                                controller.actions_being_fed.get_mut(contender_name)
                            {
                                contender_fed_entry.rescheduled_in = None;
                            }
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
                            if contender_action.violates_coyote_time() {
                                basis.violate_coyote_time();
                            }
                            match contender_directive {
                                TnuaActionLifecycleDirective::StillActive => {
                                    Some((contender_name, contender_action))
                                }
                                TnuaActionLifecycleDirective::Finished => None,
                                TnuaActionLifecycleDirective::Reschedule { after_seconds } => {
                                    reschedule_action(
                                        &mut controller.actions_being_fed,
                                        after_seconds,
                                    );
                                    None
                                }
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
                if contender_action.violates_coyote_time() {
                    basis.violate_coyote_time();
                }
                controller.current_action = Some((contender_name, contender_action));
            }

            let sensor_case_range_for_action =
                if let Some((_, current_action)) = &controller.current_action {
                    current_action.proximity_sensor_cast_range()
                } else {
                    0.0
                };

            sensor.cast_range = sensor_cast_range_for_basis.max(sensor_case_range_for_action);
        }

        // Cycle actions_being_fed
        controller.actions_being_fed.retain(|_, fed_entry| {
            if fed_entry.fed_this_frame {
                fed_entry.fed_this_frame = false;
                if let Some(rescheduled_in) = &mut fed_entry.rescheduled_in {
                    rescheduled_in.tick(time.delta());
                }
                true
            } else {
                false
            }
        });
    }
}
