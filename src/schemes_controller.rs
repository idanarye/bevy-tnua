use std::marker::PhantomData;

use crate::{TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, math::*};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use crate::schemes_traits::{
    Tnua2ActionContext, Tnua2ActionStateEnum, Tnua2Basis, Tnua2BasisAccess, TnuaScheme,
    TnuaSchemeConfig, UpdateInActionStateEnumResult,
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
}

#[derive(Default, Debug)]
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
    action_feeding_initiated: bool,
    pub current_action: Option<S::ActionStateEnum>,
}

impl<S: TnuaScheme> Tnua2Controller<S> {
    pub fn new(config: Handle<S::Config>) -> Self {
        Self {
            basis: Default::default(),
            basis_memory: Default::default(),
            config,
            actions_being_fed: (0..S::NUM_VARIANTS).map(|_| Default::default()).collect(),
            contender_action: None,
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
        self.actions_being_fed[action.variant_idx()].status = FedStatus::Fresh;
        let action = if let Some(current_action) = self.current_action.as_mut() {
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
        }) = self.contender_action.as_mut()
            && action.is_same_action_as(existing_action)
        {
            *existing_action = action;
        } else {
            self.contender_action = Some(ContenderAction { action });
        }
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
                    // info!("Action contender is active");
                    // TODO: also check the contender's initiation_decision
                    true
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
            // TODO: reschedule action
            // info!("{lifecycle_status:?} -> Existing action - {directive:?}");
            match directive {
                TnuaActionLifecycleDirective::StillActive => {
                    // TOOD: update flow status in case the action is ending
                }
                TnuaActionLifecycleDirective::Finished
                | TnuaActionLifecycleDirective::Reschedule { .. } => {
                    // TODO: handle rescheduling
                    controller.current_action = if has_valid_contender {
                        // TODO - run contender
                        None
                    } else {
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
            // TODO: set action flow status
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
