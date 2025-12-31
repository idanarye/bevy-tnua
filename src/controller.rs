use std::marker::PhantomData;

use crate::basis_capabilities::TnuaBasisWithGround;
use crate::sensor_sets::TnuaSensors;
use crate::{
    TnuaActionInitiationDirective, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, math::*,
};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy::time::Stopwatch;

use crate::basis_action_traits::{
    TnuaActionContext, TnuaActionDiscriminant, TnuaActionState, TnuaBasis, TnuaBasisAccess,
    TnuaScheme, TnuaSchemeConfig, TnuaUpdateInActionStateResult,
};
use crate::{
    TnuaBasisContext, TnuaMotor, TnuaPipelineSystems, TnuaProximitySensor, TnuaRigidBodyTracker,
    TnuaSystems, TnuaToggle, TnuaUserControlsSystems,
};

pub struct TnuaControllerPlugin<S: TnuaScheme> {
    schedule: InternedScheduleLabel,
    _phantom: PhantomData<S>,
}

/// The main for supporting Tnua character controller.
///
/// Will not work without a physics backend plugin (like `TnuaRapier2dPlugin` or
/// `TnuaRapier3dPlugin`)
///
/// Make sure the schedule for this plugin, the physics backend plugin, and the physics backend
/// itself are all using the same timestep. This usually means that the physics backend is in e.g.
/// `FixedPostUpdate` and the Tnua plugins are at `PostUpdate`.
///
/// **DO NOT mix `Update` with `FixedUpdate`!** This will mess up Tnua's calculations, resulting in
/// very unstable character motion.
impl<S: TnuaScheme> TnuaControllerPlugin<S> {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<S: TnuaScheme> Plugin for TnuaControllerPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_asset::<S::Config>();
        app.configure_sets(
            self.schedule,
            (
                TnuaPipelineSystems::Sensors,
                TnuaPipelineSystems::SubservientSensors,
                TnuaUserControlsSystems,
                TnuaPipelineSystems::Logic,
                TnuaPipelineSystems::Motors,
            )
                .chain()
                .in_set(TnuaSystems),
        );
        app.add_systems(
            self.schedule,
            apply_controller_system::<S>.in_set(TnuaPipelineSystems::Logic),
        );
    }
}

struct ContenderAction<S: TnuaScheme> {
    action: S,
    being_fed_for: Stopwatch,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum FedStatus {
    #[default]
    Not,
    Lingering,
    Fresh,
    Interrupt,
}

impl FedStatus {
    fn considered_fed(&self) -> bool {
        match self {
            FedStatus::Not => false,
            FedStatus::Lingering => true,
            FedStatus::Fresh => true,
            FedStatus::Interrupt => true,
        }
    }
}

#[derive(Default, Debug)]
struct FedEntry {
    status: FedStatus,
    rescheduled_in: Option<Timer>,
}

/// The main component used for interaction with the controls and animation code.
///
/// Every frame, the game code should invoke
/// [`initiate_action_feeding`](Self::initiate_action_feeding) and then feed input this component
/// on every controlled entity. What should be fed is:
///
/// * A basis - this is the main movement command - usually
///   [`TnuaBuiltinWalk`](crate::builtins::TnuaBuiltinWalk), but there can be others. The
///   controller's basis is takens from the scheme (the generic argument). Controlling it is done
///   by modifying the [`basis`](Self::basis) field of the controller.
///
///   Refer to the documentation of [the implementors of
///   `TnuaBasis`](crate::TnuaBasis#implementors) for more information.
///
/// * Zero or more actions - these are movements like jumping, dashing, crouching, etc. Multiple
///   actions can be fed, but only one can be active at any given moment. Unlike basis, there is a
///   smart mechanism for deciding which action to use and which to discard, so it is safe to feed
///   many actions at the same frame. Actions are also defined in the scheme, and can be fed using
///   the [`action`](Self::action) method.
///
///   Refer to the documentation of [the implementors of
///   `TnuaAction`](crate::TnuaAction#implementors) for more information.
///
/// Without [`TnuaControllerPlugin`] of the same scheme this component will not do anything.
#[derive(Component)]
#[require(TnuaMotor, TnuaRigidBodyTracker)]
pub struct TnuaController<S: TnuaScheme> {
    /// Input for the basis - the main movement command.
    pub basis: S::Basis,
    pub basis_memory: <S::Basis as TnuaBasis>::Memory,
    pub basis_config: Option<<S::Basis as TnuaBasis>::Config>,
    pub sensors_entities:
        <<S::Basis as TnuaBasis>::Sensors<'static> as TnuaSensors<'static>>::Entities,
    pub config: Handle<S::Config>,
    // TODO: If ever possible, make this a fixed size array:
    actions_being_fed: Vec<FedEntry>,
    contender_action: Option<ContenderAction<S>>,
    action_flow_status: TnuaActionFlowStatus<S::ActionDiscriminant>,
    up_direction: Option<Dir3>,
    action_feeding_initiated: bool,
    pub current_action: Option<S::ActionState>,
}

/// The result of [`TnuaController::action_flow_status()`].
#[derive(Debug, Clone)]
pub enum TnuaActionFlowStatus<D: TnuaActionDiscriminant> {
    /// No action is going on.
    NoAction,

    /// An action just started.
    ActionStarted(D),

    /// An action was fed in a past frame and is still ongoing.
    ActionOngoing(D),

    /// An action has stopped being fed.
    ///
    /// Note that the action may still have a termination sequence after this happens.
    ActionEnded(D),

    /// An action has just been canceled into another action.
    Cancelled { old: D, new: D },
}

impl<D: TnuaActionDiscriminant> TnuaActionFlowStatus<D> {
    /// The discriminant of the ongoing action, if there is an ongoing action.
    ///
    /// Will also return a value if the action has just started.
    pub fn ongoing(&self) -> Option<D> {
        match self {
            TnuaActionFlowStatus::NoAction | TnuaActionFlowStatus::ActionEnded(_) => None,
            TnuaActionFlowStatus::ActionStarted(discriminant)
            | TnuaActionFlowStatus::ActionOngoing(discriminant)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: discriminant,
            } => Some(*discriminant),
        }
    }

    /// The discriminant of the action that has just started this frame.
    ///
    /// Will return `None` if there is no action, or if the ongoing action has started in a past
    /// frame.
    pub fn just_starting(&self) -> Option<D> {
        match self {
            TnuaActionFlowStatus::NoAction
            | TnuaActionFlowStatus::ActionOngoing(_)
            | TnuaActionFlowStatus::ActionEnded(_) => None,
            TnuaActionFlowStatus::ActionStarted(discriminant)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: discriminant,
            } => Some(*discriminant),
        }
    }
}

impl<S: TnuaScheme> TnuaController<S> {
    pub fn new(config: Handle<S::Config>) -> Self {
        Self {
            basis: Default::default(),
            basis_memory: Default::default(),
            basis_config: None,
            sensors_entities: Default::default(),
            config,
            actions_being_fed: (0..S::NUM_VARIANTS).map(|_| Default::default()).collect(),
            contender_action: None,
            action_flow_status: TnuaActionFlowStatus::NoAction,
            up_direction: None,
            action_feeding_initiated: false,
            current_action: None,
        }
    }

    pub fn basis_access(
        &'_ self,
    ) -> Result<TnuaBasisAccess<'_, S::Basis>, TnuaControllerHasNotPulledConfiguration> {
        Ok(TnuaBasisAccess {
            input: &self.basis,
            config: self
                .basis_config
                .as_ref()
                .ok_or(TnuaControllerHasNotPulledConfiguration)?,
            memory: &self.basis_memory,
        })
    }

    pub fn initiate_action_feeding(&mut self) {
        self.action_feeding_initiated = true;
    }

    /// Feed an action.
    pub fn action(&mut self, action: S) {
        assert!(
            self.action_feeding_initiated,
            "Feeding action without invoking `initiate_action_feeding()`"
        );
        let fed_entry = &mut self.actions_being_fed[action.variant_idx()];

        match fed_entry.status {
            FedStatus::Lingering | FedStatus::Fresh | FedStatus::Interrupt => {
                fed_entry.status = FedStatus::Fresh;
                if let Some(current_action) = self.current_action.as_mut() {
                    match action.update_in_action_state(current_action) {
                        TnuaUpdateInActionStateResult::Success => {
                            // Do nothing farther
                        }
                        TnuaUpdateInActionStateResult::WrongVariant(_) => {
                            // different action is running - will not override because button was
                            // already pressed.
                        }
                    }
                } else if self.contender_action.is_none()
                    && fed_entry
                        .rescheduled_in
                        .as_ref()
                        .is_some_and(|timer| timer.is_finished())
                {
                    // no action is running - but this action is rescheduled and there is no
                    // already-existing contender that would have taken priority
                    self.contender_action = Some(ContenderAction {
                        action,
                        being_fed_for: Stopwatch::new(),
                    });
                } else {
                    // no action is running - will not set because button was already pressed.
                }
            }
            FedStatus::Not => {
                *fed_entry = FedEntry {
                    status: FedStatus::Fresh,
                    rescheduled_in: None,
                };
                if let Some(contender_action) = self.contender_action.as_mut()
                    && action.discriminant() == contender_action.action.discriminant()
                {
                    contender_action.action = action;
                } else if let Some(contender_action) = self.contender_action.as_ref()
                    && self.actions_being_fed[contender_action.action.discriminant().variant_idx()]
                        .status
                        == FedStatus::Interrupt
                {
                    // If the existing condender is an interrupt, we will not overwrite it.
                } else {
                    self.contender_action = Some(ContenderAction {
                        action,
                        being_fed_for: Stopwatch::new(),
                    });
                }
            }
        }
    }

    /// TODO: documentation
    pub fn action_interrupt(&mut self, action: S) {
        // Because this is an interrupt, we ignore the old fed status - but we still care not to
        // set the contender if we are the current action.
        self.actions_being_fed[action.variant_idx()] = FedEntry {
            status: FedStatus::Interrupt,
            rescheduled_in: None,
        };

        let action = if let Some(current_action) = self.current_action.as_mut() {
            match action.update_in_action_state(current_action) {
                TnuaUpdateInActionStateResult::Success => {
                    return;
                }
                TnuaUpdateInActionStateResult::WrongVariant(action) => {
                    // different action is running - we'll have to
                    action
                }
            }
        } else {
            action
        };
        // Overwrite the condender action even if there already was a contender action.
        self.contender_action = Some(ContenderAction {
            action,
            being_fed_for: Stopwatch::new(),
        });
    }

    /// Re-feed the same action that is currently active.
    ///
    /// This is useful when matching on [`current_action`](Self::current_action) and wanting to
    /// continue feeding the **exact same** action with the **exact same** input without having to
    pub fn prolong_action(&mut self) {
        if let Some(current_action) = self.action_discriminant() {
            self.actions_being_fed[current_action.variant_idx()].status = FedStatus::Fresh;
        }
    }

    /// The discriminant of the currently running action.
    pub fn action_discriminant(&self) -> Option<S::ActionDiscriminant> {
        Some(self.current_action.as_ref()?.discriminant())
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
    pub fn action_flow_status(&self) -> &TnuaActionFlowStatus<S::ActionDiscriminant> {
        &self.action_flow_status
    }

    /// Returns the direction considered as up.
    ///
    /// Note that the up direction is based on gravity, as reported by
    /// [`TnuaRigidBodyTracker::gravity`], and that it'd typically be one frame behind since it
    /// gets updated in the same system that applies the controller logic. If this is unacceptable,
    /// consider using [`TnuaRigidBodyTracker::gravity`] directly or deducing the up direction via
    /// different means.
    pub fn up_direction(&self) -> Option<Dir3> {
        self.up_direction
    }
}

impl<S: TnuaScheme> TnuaController<S>
where
    S::Basis: TnuaBasisWithGround,
{
    /// Checks if the character is currently airborne.
    ///
    /// The check is done based on the basis, and is equivalent to getting the controller's
    /// [`basis_access`](Self::basis_access) and using [`TnuaBasisWithGround::is_airborne`] on it.
    pub fn is_airborne(&self) -> Result<bool, TnuaControllerHasNotPulledConfiguration> {
        Ok(S::Basis::is_airborne(&self.basis_access()?))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("The Tnua controller did not pull the configuration asset yet")]
pub struct TnuaControllerHasNotPulledConfiguration;

#[allow(clippy::type_complexity)]
fn apply_controller_system<S: TnuaScheme>(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut TnuaController<S>,
        &TnuaRigidBodyTracker,
        &mut TnuaMotor,
        Option<&TnuaToggle>,
    )>,
    proximity_sensors_query: Query<&TnuaProximitySensor>,
    config_assets: Res<Assets<S::Config>>,
    mut commands: Commands,
) {
    let frame_duration = time.delta().as_secs_f64() as Float;
    if frame_duration == 0.0 {
        return;
    }
    for (controller_entity, mut controller, tracker, mut motor, tnua_toggle) in query.iter_mut() {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }
        let controller = controller.as_mut();

        let Some(config) = config_assets.get(&controller.config) else {
            continue;
        };
        controller.basis_config = Some({
            let mut basis_config = config.basis_config().clone();
            if let Some(current_action) = controller.current_action.as_ref() {
                current_action.modify_basis_config(&mut basis_config);
            }
            basis_config
        });
        let basis_config = controller
            .basis_config
            .as_ref()
            .expect("We just set it to Some");

        let up_direction = Dir3::new(-tracker.gravity.f32()).ok();
        controller.up_direction = up_direction;
        // TODO: support the case where there is no up direction at all?
        let up_direction = up_direction.unwrap_or(Dir3::Y);

        let basis_clone = basis_config.clone();
        let Some((proximity_sensor, _sensors)) = S::Basis::get_or_create_sensors(
            up_direction,
            &basis_clone,
            &mut controller.sensors_entities,
            &proximity_sensors_query,
            controller_entity,
            &mut commands,
        ) else {
            continue;
        };

        match controller.action_flow_status {
            TnuaActionFlowStatus::NoAction | TnuaActionFlowStatus::ActionOngoing(_) => {}
            TnuaActionFlowStatus::ActionEnded(_) => {
                controller.action_flow_status = TnuaActionFlowStatus::NoAction;
            }
            TnuaActionFlowStatus::ActionStarted(discriminant)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: discriminant,
            } => {
                controller.action_flow_status = TnuaActionFlowStatus::ActionOngoing(discriminant);
            }
        }

        controller.basis.apply(
            basis_config,
            &mut controller.basis_memory,
            TnuaBasisContext {
                frame_duration,
                tracker,
                // sensors,
                proximity_sensor,
                up_direction,
            },
            &mut motor,
        );

        if controller.action_feeding_initiated {
            controller.action_feeding_initiated = false;
            for fed_entry in controller.actions_being_fed.iter_mut() {
                match fed_entry.status {
                    FedStatus::Not => {}
                    FedStatus::Lingering => {
                        *fed_entry = Default::default();
                    }
                    FedStatus::Fresh | FedStatus::Interrupt => {
                        fed_entry.status = FedStatus::Lingering;
                        if let Some(rescheduled_in) = &mut fed_entry.rescheduled_in {
                            rescheduled_in.tick(time.delta());
                        }
                    }
                }
            }
        }

        let has_valid_contender =
            if let Some(contender_action) = controller.contender_action.as_mut() {
                if controller.actions_being_fed[contender_action.action.variant_idx()]
                    .status
                    .considered_fed()
                {
                    let initiation_decision = contender_action.action.initiation_decision(
                        config,
                        TnuaActionContext {
                            frame_duration,
                            tracker,
                            // sensors,
                            proximity_sensor,
                            up_direction,
                            basis: &TnuaBasisAccess {
                                input: &controller.basis,
                                config: basis_config,
                                memory: &controller.basis_memory,
                            },
                        },
                        &contender_action.being_fed_for,
                    );
                    contender_action.being_fed_for.tick(time.delta());
                    match initiation_decision {
                        TnuaActionInitiationDirective::Reject => {
                            controller.contender_action = None;
                            false
                        }
                        TnuaActionInitiationDirective::Delay => false,
                        TnuaActionInitiationDirective::Allow => true,
                    }
                } else {
                    controller.contender_action = None;
                    false
                }
            } else {
                false
            };

        if let Some(action_state) = controller.current_action.as_mut() {
            let lifecycle_status = if has_valid_contender {
                TnuaActionLifecycleStatus::CancelledInto
            } else if controller.actions_being_fed[action_state.variant_idx()]
                .status
                .considered_fed()
            {
                TnuaActionLifecycleStatus::StillFed
            } else {
                TnuaActionLifecycleStatus::NoLongerFed
            };

            let directive = action_state.interface_mut().apply(
                TnuaActionContext {
                    frame_duration,
                    tracker,
                    // sensors,
                    proximity_sensor,
                    basis: &TnuaBasisAccess {
                        input: &controller.basis,
                        config: basis_config,
                        memory: &controller.basis_memory,
                    },
                    up_direction,
                },
                lifecycle_status,
                motor.as_mut(),
            );
            action_state.interface_mut().influence_basis(
                TnuaBasisContext {
                    frame_duration,
                    tracker,
                    // sensors,
                    proximity_sensor,
                    up_direction,
                },
                &controller.basis,
                basis_config,
                &mut controller.basis_memory,
            );
            match directive {
                TnuaActionLifecycleDirective::StillActive => {
                    if !lifecycle_status.is_active()
                        && let TnuaActionFlowStatus::ActionOngoing(action_discriminant) =
                            controller.action_flow_status
                    {
                        controller.action_flow_status =
                            TnuaActionFlowStatus::ActionEnded(action_discriminant);
                    }
                }
                TnuaActionLifecycleDirective::Finished
                | TnuaActionLifecycleDirective::Reschedule { .. } => {
                    if let TnuaActionLifecycleDirective::Reschedule { after_seconds } = directive {
                        controller.actions_being_fed[action_state.variant_idx()].rescheduled_in =
                            Some(Timer::from_seconds(after_seconds.f32(), TimerMode::Once));
                    }
                    controller.current_action = if has_valid_contender {
                        let contender_action = controller.contender_action.take().expect(
                            "has_valid_contender can only be true if contender_action is Some",
                        );
                        let mut contender_action_state =
                            contender_action.action.into_action_state_variant(config);

                        controller.actions_being_fed[contender_action_state.variant_idx()]
                            .rescheduled_in = None;

                        let contender_directive = contender_action_state.interface_mut().apply(
                            TnuaActionContext {
                                frame_duration,
                                tracker,
                                proximity_sensor,
                                basis: &TnuaBasisAccess {
                                    input: &controller.basis,
                                    config: basis_config,
                                    memory: &controller.basis_memory,
                                },
                                up_direction,
                            },
                            TnuaActionLifecycleStatus::CancelledFrom,
                            motor.as_mut(),
                        );
                        contender_action_state.interface_mut().influence_basis(
                            TnuaBasisContext {
                                frame_duration,
                                tracker,
                                // sensors,
                                proximity_sensor,
                                up_direction,
                            },
                            &controller.basis,
                            basis_config,
                            &mut controller.basis_memory,
                        );
                        match contender_directive {
                            TnuaActionLifecycleDirective::StillActive => {
                                controller.action_flow_status =
                                    if let TnuaActionFlowStatus::ActionOngoing(discriminant) =
                                        controller.action_flow_status
                                    {
                                        TnuaActionFlowStatus::Cancelled {
                                            old: discriminant,
                                            new: contender_action_state.discriminant(),
                                        }
                                    } else {
                                        TnuaActionFlowStatus::ActionStarted(
                                            contender_action_state.discriminant(),
                                        )
                                    };
                                Some(contender_action_state)
                            }
                            TnuaActionLifecycleDirective::Finished
                            | TnuaActionLifecycleDirective::Reschedule { after_seconds: _ } => {
                                if let TnuaActionLifecycleDirective::Reschedule { after_seconds } =
                                    contender_directive
                                {
                                    controller.actions_being_fed
                                        [contender_action_state.variant_idx()]
                                    .rescheduled_in = Some(Timer::from_seconds(
                                        after_seconds.f32(),
                                        TimerMode::Once,
                                    ));
                                }
                                if let TnuaActionFlowStatus::ActionOngoing(discriminant) =
                                    controller.action_flow_status
                                {
                                    controller.action_flow_status =
                                        TnuaActionFlowStatus::ActionEnded(discriminant);
                                }
                                None
                            }
                        }
                    } else {
                        controller.action_flow_status =
                            TnuaActionFlowStatus::ActionEnded(action_state.discriminant());
                        None
                    };
                }
            }
        } else if has_valid_contender {
            let contender_action = controller
                .contender_action
                .take()
                .expect("has_valid_contender can only be true if contender_action is Some");
            let mut contender_action_state =
                contender_action.action.into_action_state_variant(config);

            contender_action_state.interface_mut().apply(
                TnuaActionContext {
                    frame_duration,
                    tracker,
                    // sensors,
                    proximity_sensor,
                    basis: &TnuaBasisAccess {
                        input: &controller.basis,
                        config: basis_config,
                        memory: &controller.basis_memory,
                    },
                    up_direction,
                },
                TnuaActionLifecycleStatus::Initiated,
                motor.as_mut(),
            );
            contender_action_state.interface_mut().influence_basis(
                TnuaBasisContext {
                    frame_duration,
                    tracker,
                    // sensors,
                    proximity_sensor,
                    up_direction,
                },
                &controller.basis,
                basis_config,
                &mut controller.basis_memory,
            );
            controller.action_flow_status =
                TnuaActionFlowStatus::ActionStarted(contender_action_state.discriminant());
            controller.current_action = Some(contender_action_state);
        }
    }
}
