use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy::utils::{Entry, HashMap};
use bevy_tnua_physics_integration_layer::math::{AsF32, Float};

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
pub struct TnuaControllerPlugin {
    schedule: InternedScheduleLabel,
}

impl TnuaControllerPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Default for TnuaControllerPlugin {
    fn default() -> Self {
        Self::new(Update)
    }
}

impl Plugin for TnuaControllerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            self.schedule,
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
            self.schedule,
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
    action_flow_status: TnuaActionFlowStatus,
}

impl TnuaController {
    /// Feed a basis - the main movement command - with [its default name](TnuaBasis::NAME).
    pub fn basis<B: TnuaBasis>(&mut self, basis: B) {
        self.named_basis(B::NAME, basis);
    }

    /// Feed a basis - the main movement command - with a custom name.
    ///
    /// This should only be used if the same basis type needs to be used with different names to
    /// allow, for example, different animations. Otherwise prefer to use the default name with
    /// [`basis`](Self::basis).
    pub fn named_basis<B: TnuaBasis>(&mut self, name: &'static str, basis: B) {
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
    }

    /// Instruct the basis to pretend the user provided no input this frame.
    ///
    /// The exact meaning is defined in the basis' [`neutralize`](TnuaBasis::neutralize) method,
    /// but generally it means that fields that typically come from a configuration will not be
    /// touched, and only fields that are typically set by user input get nullified.
    pub fn neutralize_basis(&mut self) {
        if let Some((_, basis)) = self.current_basis.as_mut() {
            basis.neutralize();
        }
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
    pub fn dynamic_basis(&self) -> Option<&dyn DynamicBasis> {
        Some(self.current_basis.as_ref()?.1.as_ref())
    }

    /// The currently running basis, together with its state.
    ///
    /// This is mainly useful for animation. When multiple basis types are used in the game,
    /// [`basis_name`](Self::basis_name) be used to determine the type of the current basis first,
    /// to avoid having to try multiple downcasts.
    pub fn concrete_basis<B: TnuaBasis>(&self) -> Option<(&B, &B::State)> {
        let (_, basis) = self.current_basis.as_ref()?;
        let boxable_basis: &BoxableBasis<B> = basis.as_any().downcast_ref()?;
        Some((&boxable_basis.input, &boxable_basis.state))
    }

    /// Feed an action with [its default name](TnuaBasis::NAME).
    pub fn action<A: TnuaAction>(&mut self, action: A) {
        self.named_action(A::NAME, action);
    }

    /// Feed an action with a custom name.
    ///
    /// This should only be used if the same action type needs to be used with different names to
    /// allow, for example, different animations. Otherwise prefer to use the default name with
    /// [`action`](Self::action).
    pub fn named_action<A: TnuaAction>(&mut self, name: &'static str, action: A) {
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

    /// A dynamic accessor to the currently running action.
    pub fn dynamic_action(&self) -> Option<&dyn DynamicAction> {
        Some(self.current_action.as_ref()?.1.as_ref())
    }

    /// The currently running action, together with its state.
    ///
    /// This is mainly useful for animation. When multiple action types are used in the game,
    /// [`action_name`](Self::action_name) be used to determine the type of the current action
    /// first, to avoid having to try multiple downcasts.
    pub fn concrete_action<A: TnuaAction>(&self) -> Option<(&A, &A::State)> {
        let (_, action) = self.current_action.as_ref()?;
        let boxable_action: &BoxableAction<A> = action.as_any().downcast_ref()?;
        Some((&boxable_action.input, &boxable_action.state))
    }

    /// Indicator for the state and flow of movement actions.
    ///
    /// Query this every frame to keep track of the actions. For air actions,
    /// [`TnuaAirActionsTracker`](crate::control_helpers::TnuaAirActionsTracker) is easier to use
    /// (and uses this behind the scenes)
    ///
    /// The benefits of this over querying [`action_name`](Self::action_name) every frame are:
    ///
    /// * `action_flow_status` can indicate when the same action has been fed again immediately
    ///   after stopping or cancelled into itself.
    /// * `action_flow_status` shows an [`ActionEnded`](TnuaActionFlowStatus::ActionEnded) when the
    ///   action is no longer fed, even if the action is still active (termination sequence)
    pub fn action_flow_status(&self) -> &TnuaActionFlowStatus {
        &self.action_flow_status
    }

    /// Checks if the character is currently airborne.
    ///
    /// The check is done based on the basis, and is equivalent to getting the controller's
    /// [`dynamic_basis`](Self::dynamic_basis) and checking its
    /// [`is_airborne`](TnuaBasis::is_airborne) method.
    pub fn is_airborne(&self) -> Result<bool, TnuaControllerHasNoBasis> {
        match self.dynamic_basis() {
            Some(basis) => Ok(basis.is_airborne()),
            None => Err(TnuaControllerHasNoBasis),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("The Tnua controller does not have any basis set")]
pub struct TnuaControllerHasNoBasis;

/// The result of [`TnuaController::action_flow_status()`].
#[derive(Debug, Default, Clone)]
pub enum TnuaActionFlowStatus {
    /// No action is going on.
    #[default]
    NoAction,

    /// An action just started.
    ActionStarted(&'static str),

    /// An action was fed in a past frame and is still ongoing.
    ActionOngoing(&'static str),

    /// An action has stopped being fed.
    ///
    /// Note that the action may still have a termination sequence after this happens.
    ActionEnded(&'static str),

    /// An action has just been canceled into another action.
    Cancelled {
        old: &'static str,
        new: &'static str,
    },
}

impl TnuaActionFlowStatus {
    /// The name of the ongoing action, if there is an ongoing action.
    ///
    /// Will also return a value if the action has just started.
    pub fn ongoing(&self) -> Option<&'static str> {
        match self {
            TnuaActionFlowStatus::NoAction | TnuaActionFlowStatus::ActionEnded(_) => None,
            TnuaActionFlowStatus::ActionStarted(action_name)
            | TnuaActionFlowStatus::ActionOngoing(action_name)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => Some(action_name),
        }
    }

    /// The name of the action that has just started this frame.
    ///
    /// Will return `None` if there is no action, or if the ongoing action has started in a past
    /// frame.
    pub fn just_starting(&self) -> Option<&'static str> {
        match self {
            TnuaActionFlowStatus::NoAction
            | TnuaActionFlowStatus::ActionOngoing(_)
            | TnuaActionFlowStatus::ActionEnded(_) => None,
            TnuaActionFlowStatus::ActionStarted(action_name)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => Some(action_name),
        }
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
    let frame_duration = time.delta().as_secs_f64() as Float;
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

        match controller.action_flow_status {
            TnuaActionFlowStatus::NoAction | TnuaActionFlowStatus::ActionOngoing(_) => {}
            TnuaActionFlowStatus::ActionEnded(_) => {
                controller.action_flow_status = TnuaActionFlowStatus::NoAction;
            }
            TnuaActionFlowStatus::ActionStarted(action_name)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => {
                controller.action_flow_status = TnuaActionFlowStatus::ActionOngoing(action_name);
            }
        }

        if let Some((_, basis)) = controller.current_basis.as_mut() {
            let up_direction = Dir3::new(-tracker.gravity.f32()).unwrap_or(Dir3::Y);
            let basis = basis.as_mut();
            basis.apply(
                TnuaBasisContext {
                    frame_duration,
                    tracker,
                    proximity_sensor: sensor.as_ref(),
                    up_direction,
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
                        up_direction,
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
                        up_direction,
                    },
                    lifecycle_status,
                    motor.as_mut(),
                );
                if current_action.violates_coyote_time() {
                    basis.violate_coyote_time();
                }
                let reschedule_action =
                    |actions_being_fed: &mut HashMap<&'static str, FedEntry>,
                     after_seconds: Float| {
                        if let Some(fed_entry) = actions_being_fed.get_mut(name) {
                            fed_entry.rescheduled_in =
                                Some(Timer::from_seconds(after_seconds.f32(), TimerMode::Once));
                        }
                    };
                match directive {
                    TnuaActionLifecycleDirective::StillActive => {
                        if !lifecycle_status.is_active()
                            && matches!(
                                controller.action_flow_status,
                                TnuaActionFlowStatus::ActionOngoing(_)
                            )
                        {
                            controller.action_flow_status = TnuaActionFlowStatus::ActionEnded(name);
                        }
                    }
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
                                    up_direction,
                                },
                                TnuaActionLifecycleStatus::CancelledFrom,
                                motor.as_mut(),
                            );
                            if contender_action.violates_coyote_time() {
                                basis.violate_coyote_time();
                            }
                            match contender_directive {
                                TnuaActionLifecycleDirective::StillActive => {
                                    if matches!(
                                        controller.action_flow_status,
                                        TnuaActionFlowStatus::ActionOngoing(_)
                                    ) {
                                        controller.action_flow_status =
                                            TnuaActionFlowStatus::Cancelled {
                                                old: name,
                                                new: contender_name,
                                            };
                                    } else {
                                        controller.action_flow_status =
                                            TnuaActionFlowStatus::ActionStarted(contender_name);
                                    }
                                    Some((contender_name, contender_action))
                                }
                                TnuaActionLifecycleDirective::Finished => {
                                    if matches!(
                                        controller.action_flow_status,
                                        TnuaActionFlowStatus::ActionOngoing(_)
                                    ) {
                                        controller.action_flow_status =
                                            TnuaActionFlowStatus::ActionEnded(name);
                                    }
                                    None
                                }
                                TnuaActionLifecycleDirective::Reschedule { after_seconds } => {
                                    if matches!(
                                        controller.action_flow_status,
                                        TnuaActionFlowStatus::ActionOngoing(_)
                                    ) {
                                        controller.action_flow_status =
                                            TnuaActionFlowStatus::ActionEnded(name);
                                    }
                                    reschedule_action(
                                        &mut controller.actions_being_fed,
                                        after_seconds,
                                    );
                                    None
                                }
                            }
                        } else {
                            controller.action_flow_status = TnuaActionFlowStatus::ActionEnded(name);
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
                        up_direction,
                    },
                    TnuaActionLifecycleStatus::Initiated,
                    motor.as_mut(),
                );
                if contender_action.violates_coyote_time() {
                    basis.violate_coyote_time();
                }
                controller.action_flow_status = TnuaActionFlowStatus::ActionStarted(contender_name);
                controller.current_action = Some((contender_name, contender_action));
            }

            let sensor_case_range_for_action =
                if let Some((_, current_action)) = &controller.current_action {
                    current_action.proximity_sensor_cast_range()
                } else {
                    0.0
                };

            sensor.cast_range = sensor_cast_range_for_basis.max(sensor_case_range_for_action);
            sensor.cast_direction = -up_direction;
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

        if let Some((contender_name, ..)) = controller.contender_action {
            if !controller.actions_being_fed.contains_key(contender_name) {
                controller.contender_action = None;
            }
        }
    }
}
