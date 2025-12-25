use std::marker::PhantomData;

use crate::{
    TnuaActionInitiationDirective, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, math::*,
};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;
use bevy::time::Stopwatch;

use crate::schemes_traits::{
    Tnua2ActionContext, Tnua2ActionDiscriminant, Tnua2ActionStateEnum, Tnua2Basis,
    Tnua2BasisAccess, TnuaScheme, TnuaSchemeConfig, UpdateInActionStateEnumResult,
};
use crate::{
    TnuaBasisContext, TnuaMotor, TnuaPipelineSystems, TnuaProximitySensor, TnuaRigidBodyTracker,
    TnuaSystems, TnuaToggle, TnuaUserControlsSystems,
};

pub struct Tnua2ControllerPlugin<S: TnuaScheme> {
    schedule: InternedScheduleLabel,
    _phantom: PhantomData<S>,
}

impl<S: TnuaScheme> Tnua2ControllerPlugin<S> {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<S: TnuaScheme> Plugin for Tnua2ControllerPlugin<S> {
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
}

impl FedStatus {
    fn considered_fed(&self) -> bool {
        match self {
            FedStatus::Not => false,
            FedStatus::Lingering => true,
            FedStatus::Fresh => true,
        }
    }
}

#[derive(Default, Debug)]
struct FedEntry {
    status: FedStatus,
    rescheduled_in: Option<Timer>,
}

#[derive(Component)]
#[require(TnuaMotor, TnuaRigidBodyTracker, TnuaProximitySensor)]
pub struct Tnua2Controller<S: TnuaScheme> {
    pub basis: S::Basis,
    pub basis_memory: <S::Basis as Tnua2Basis>::Memory,
    pub config: Handle<S::Config>,
    // TODO: If ever possible, make this a fixed size array:
    actions_being_fed: Vec<FedEntry>,
    contender_action: Option<ContenderAction<S>>,
    action_flow_status: Tnua2ActionFlowStatus<S::ActionDiscriminant>,
    action_feeding_initiated: bool,
    pub current_action: Option<S::ActionStateEnum>,
}

/// The result of [`TnuaController::action_flow_status()`].
#[derive(Debug, Clone)]
pub enum Tnua2ActionFlowStatus<D: Tnua2ActionDiscriminant> {
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

impl<D: Tnua2ActionDiscriminant> Tnua2ActionFlowStatus<D> {
    /// The discriminant of the ongoing action, if there is an ongoing action.
    ///
    /// Will also return a value if the action has just started.
    pub fn ongoing(&self) -> Option<D> {
        match self {
            Tnua2ActionFlowStatus::NoAction | Tnua2ActionFlowStatus::ActionEnded(_) => None,
            Tnua2ActionFlowStatus::ActionStarted(discriminant)
            | Tnua2ActionFlowStatus::ActionOngoing(discriminant)
            | Tnua2ActionFlowStatus::Cancelled {
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
            Tnua2ActionFlowStatus::NoAction
            | Tnua2ActionFlowStatus::ActionOngoing(_)
            | Tnua2ActionFlowStatus::ActionEnded(_) => None,
            Tnua2ActionFlowStatus::ActionStarted(discriminant)
            | Tnua2ActionFlowStatus::Cancelled {
                old: _,
                new: discriminant,
            } => Some(*discriminant),
        }
    }
}

impl<S: TnuaScheme> Tnua2Controller<S> {
    pub fn new(config: Handle<S::Config>) -> Self {
        Self {
            basis: Default::default(),
            basis_memory: Default::default(),
            config,
            actions_being_fed: (0..S::NUM_VARIANTS).map(|_| Default::default()).collect(),
            contender_action: None,
            action_flow_status: Tnua2ActionFlowStatus::NoAction,
            action_feeding_initiated: false,
            current_action: None,
        }
    }

    pub fn initiate_action_feeding(&mut self) {
        self.action_feeding_initiated = true;
    }

    pub fn action(&mut self, action: S) {
        assert!(
            self.action_feeding_initiated,
            "Feeding action without invoking `initiate_action_feeding()`"
        );
        let fed_entry = &mut self.actions_being_fed[action.variant_idx()];
        let orig_status = fed_entry.status;
        fed_entry.status = FedStatus::Fresh;
        let action = if orig_status != FedStatus::Not
            && let Some(current_action) = self.current_action.as_mut()
        {
            match action.update_in_action_state_enum(current_action) {
                UpdateInActionStateEnumResult::Success => {
                    return;
                }
                UpdateInActionStateEnumResult::WrongVariant(action) => action,
            }
        } else {
            action
        };
        if let Some(ContenderAction {
            action: existing_action,
            being_fed_for: _,
        }) = self.contender_action.as_mut()
            && action.is_same_action_as(existing_action)
        {
            *existing_action = action;
        } else if orig_status == FedStatus::Not
            // no action is running - but this action is rescheduled and there is no
            // already-existing contender that would have taken priority:
            || fed_entry
                .rescheduled_in
                .as_ref()
                .is_some_and(|timer| timer.is_finished())
        {
            self.contender_action = Some(ContenderAction {
                action,
                being_fed_for: Stopwatch::new(),
            });
            // Set the rescheduling to None so that we won't tick it anymore.
            fed_entry.rescheduled_in = None;
        }
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
    pub fn action_flow_status(&self) -> &Tnua2ActionFlowStatus<S::ActionDiscriminant> {
        &self.action_flow_status
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system<S: TnuaScheme>(
    time: Res<Time>,
    mut query: Query<(
        &mut Tnua2Controller<S>,
        &TnuaRigidBodyTracker,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
        Option<&TnuaToggle>,
    )>,
    config_assets: Res<Assets<S::Config>>,
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

        let Some(config) = config_assets.get(&controller.config) else {
            continue;
        };
        let basis_config = config.basis_config();

        let up_direction = Dir3::new(-tracker.gravity.f32()).ok();
        let up_direction = up_direction.unwrap_or(Dir3::Y);

        let proximity_sensor = sensor.as_mut();

        match controller.action_flow_status {
            Tnua2ActionFlowStatus::NoAction | Tnua2ActionFlowStatus::ActionOngoing(_) => {}
            Tnua2ActionFlowStatus::ActionEnded(_) => {
                controller.action_flow_status = Tnua2ActionFlowStatus::NoAction;
            }
            Tnua2ActionFlowStatus::ActionStarted(discriminant)
            | Tnua2ActionFlowStatus::Cancelled {
                old: _,
                new: discriminant,
            } => {
                controller.action_flow_status = Tnua2ActionFlowStatus::ActionOngoing(discriminant);
            }
        }

        controller.basis.apply(
            basis_config,
            &mut controller.basis_memory,
            TnuaBasisContext {
                frame_duration,
                tracker,
                proximity_sensor,
                up_direction,
            },
            &mut motor,
        );
        let sensor_cast_range_for_basis = controller
            .basis
            .proximity_sensor_cast_range(basis_config, &controller.basis_memory);

        if controller.action_feeding_initiated {
            controller.action_feeding_initiated = false;
            for fed_entry in controller.actions_being_fed.iter_mut() {
                match fed_entry.status {
                    FedStatus::Not => {}
                    FedStatus::Lingering => {
                        *fed_entry = Default::default();
                    }
                    FedStatus::Fresh => {
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
                        Tnua2ActionContext {
                            frame_duration,
                            tracker,
                            proximity_sensor,
                            up_direction,
                            basis: &Tnua2BasisAccess {
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
                Tnua2ActionContext {
                    frame_duration,
                    tracker,
                    proximity_sensor,
                    basis: &Tnua2BasisAccess {
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
                    proximity_sensor,
                    up_direction,
                },
                &controller.basis,
                basis_config,
                &mut controller.basis_memory,
            );
            match directive {
                TnuaActionLifecycleDirective::StillActive => {
                    // TOOD: update flow status in case the action is ending
                }
                TnuaActionLifecycleDirective::Finished
                | TnuaActionLifecycleDirective::Reschedule { .. } => {
                    if let TnuaActionLifecycleDirective::Reschedule { after_seconds } = directive {
                        controller.actions_being_fed[action_state.variant_idx()].rescheduled_in =
                            Some(Timer::from_seconds(after_seconds.f32(), TimerMode::Once));
                    }
                    (controller.current_action, controller.action_flow_status) =
                        if has_valid_contender {
                            // TODO - run contender. Remember to:
                            // * Handle scheduling
                            // * Set the discriminant to Cancel
                            (
                                None,
                                Tnua2ActionFlowStatus::ActionEnded(action_state.discriminant()),
                            )
                        } else {
                            (
                                None,
                                Tnua2ActionFlowStatus::ActionEnded(action_state.discriminant()),
                            )
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
                Tnua2ActionContext {
                    frame_duration,
                    tracker,
                    proximity_sensor,
                    basis: &Tnua2BasisAccess {
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
                    proximity_sensor,
                    up_direction,
                },
                &controller.basis,
                basis_config,
                &mut controller.basis_memory,
            );
            controller.action_flow_status =
                Tnua2ActionFlowStatus::ActionStarted(contender_action_state.discriminant());
            controller.current_action = Some(contender_action_state);
        }

        let sensor_cast_range_for_action = 0.0; // TODO - base this on the action if there is one

        proximity_sensor.cast_range = sensor_cast_range_for_basis.max(sensor_cast_range_for_action);
        proximity_sensor.cast_direction = -up_direction;
        // TODO: Maybe add the horizontal rotation as well somehow?
        proximity_sensor.cast_shape_rotation =
            Quaternion::from_rotation_arc(Vector3::Y, up_direction.adjust_precision());
    }
}
