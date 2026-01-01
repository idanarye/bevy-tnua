use std::time::Duration;

use crate::TnuaMotor;
use crate::action_state::TnuaActionStateInterface;
use crate::ghost_overrides::TnuaGhostOverwrite;
use crate::sensor_sets::TnuaSensors;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaGhostSensor;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaProximitySensor;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaRigidBodyTracker;

use crate::math::*;

pub trait TnuaScheme: 'static + Send + Sync + Sized {
    type Basis: TnuaBasis;
    type Config: TnuaSchemeConfig<Scheme = Self> + Asset;
    type ActionDiscriminant: TnuaActionDiscriminant;
    type ActionState: TnuaActionState<Basis = Self::Basis, Discriminant = Self::ActionDiscriminant>;

    const NUM_VARIANTS: usize;

    fn discriminant(&self) -> Self::ActionDiscriminant;

    fn variant_idx(&self) -> usize {
        self.discriminant().variant_idx()
    }

    fn is_same_action_as(&self, other: &Self) -> bool {
        self.variant_idx() == other.variant_idx()
    }

    fn into_action_state_variant(self, config: &Self::Config) -> Self::ActionState;

    fn update_in_action_state(
        self,
        action_state_enum: &mut Self::ActionState,
    ) -> TnuaUpdateInActionStateResult<Self>;

    fn initiation_decision(
        &self,
        config: &Self::Config,
        sensors: &<Self::Basis as TnuaBasis>::Sensors<'_>,
        ctx: TnuaActionContext<Self::Basis>,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective;
}

pub enum TnuaUpdateInActionStateResult<S: TnuaScheme> {
    Success,
    WrongVariant(S),
}

pub trait TnuaSchemeConfig {
    type Scheme: TnuaScheme<Config = Self>;

    fn basis_config(&self) -> &<<Self::Scheme as TnuaScheme>::Basis as TnuaBasis>::Config;
}

/// Various data passed to [`TnuaBasis::apply`].
#[derive(Debug)]
pub struct TnuaBasisContext<'a> {
    /// The duration of the current frame.
    pub frame_duration: Float,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a TnuaRigidBodyTracker,

    /// The direction considered as "up".
    pub up_direction: Dir3,
}

/// The main movement command of a character.
///
/// A basis handles the character's motion when the user is not feeding it any input, or when it
/// just moves around without doing anything special. A simple game would only need one basis -
/// [`TnuaBuiltinWalk`](crate::builtins::TnuaBuiltinWalk) - but more complex games can have bases
/// for things like swimming or driving.
///
/// The type that implements this trait is called the basis _input_, and is expected to be
/// overwritten each frame by the controller system of the game code. Configuration is considered
/// as part of the input. Configuration is stored in an asset, as part of a struct implementing
/// [`TnuaSchemeConfig`] which also holds the configuration for all the actions. If the basis needs
/// to persist data between frames it must keep it in its [memory](TnuaBasis::Memory).
pub trait TnuaBasis: Default + 'static + Send + Sync {
    type Config: Send + Sync + Clone;
    type Memory: Send + Sync + Default;
    type Sensors<'a>: TnuaSensors<'a>;

    /// This is where the basis affects the character's motion.
    ///
    /// This method gets called each frame to let the basis control the [`TnuaMotor`] that will
    /// later move the character.
    ///
    /// Note that after the motor is set in this method, if there is an action going on, the
    /// action's [`apply`](TnuaAction::apply) will also run and typically change some of the things
    /// the basis did to the motor.
    ///                                                              
    /// It can also update the memory.
    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        sensors: &Self::Sensors<'_>,
        ctx: TnuaBasisContext,
        motor: &mut TnuaMotor,
    );

    /// A value to configure the range of the ground proximity sensor according to the basis'
    /// needs.
    fn proximity_sensor_cast_range(&self, config: &Self::Config, memory: &Self::Memory) -> Float;

    fn get_or_create_sensors<'a: 'b, 'b>(
        up_direction: Dir3,
        config: &'a Self::Config,
        memory: &Self::Memory,
        entities: &'a mut <Self::Sensors<'static> as TnuaSensors<'static>>::Entities,
        proximity_sensors_query: &'b Query<(&TnuaProximitySensor, Has<TnuaGhostSensor>)>,
        controller_entity: Entity,
        commands: &mut Commands,
        has_ghost_overwrites: bool,
    ) -> Option<Self::Sensors<'b>>;

    fn ghost_sensor_overwrites<'a>(
        ghost_overwrites: &'a mut <Self::Sensors<'static> as TnuaSensors<'static>>::GhostOverwrites,
        entities: &<Self::Sensors<'static> as TnuaSensors<'static>>::Entities,
    ) -> impl Iterator<Item = (&'a mut TnuaGhostOverwrite, Entity)>;
}

/// Input for [`TnuaAction::apply`] that informs it about the long-term feeding of the input.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TnuaActionLifecycleStatus {
    /// There was no action in the previous frame
    Initiated,
    /// There was a different action in the previous frame
    CancelledFrom,
    /// This action was already active in the previous frame, and it keeps getting fed
    StillFed,
    /// This action was fed up until the previous frame, and now no action is fed
    NoLongerFed,
    /// This action was fed up until the previous frame, and now a different action tries to override it
    CancelledInto,
}

impl TnuaActionLifecycleStatus {
    /// Continue if the action is still fed, finish if its not fed or if some other action gets
    /// fed.
    pub fn directive_simple(&self) -> TnuaActionLifecycleDirective {
        match self {
            TnuaActionLifecycleStatus::Initiated => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::CancelledFrom => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::StillFed => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::NoLongerFed => TnuaActionLifecycleDirective::Finished,
            TnuaActionLifecycleStatus::CancelledInto => TnuaActionLifecycleDirective::Finished,
        }
    }

    /// Similar to [`directive_simple`](Self::directive_simple), but if some other action gets fed
    /// and this action is still being fed, reschedule this action once the other action finishes,
    /// as long as more time than `after_seconds` has passed.
    pub fn directive_simple_reschedule(
        &self,
        after_seconds: Float,
    ) -> TnuaActionLifecycleDirective {
        match self {
            TnuaActionLifecycleStatus::Initiated => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::CancelledFrom => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::StillFed => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::NoLongerFed => {
                // The rescheduling will probably go away, but in case things happen too fast and
                // it doesn't - pass it anyway.
                TnuaActionLifecycleDirective::Reschedule { after_seconds }
            }
            TnuaActionLifecycleStatus::CancelledInto => {
                TnuaActionLifecycleDirective::Reschedule { after_seconds }
            }
        }
    }

    /// Continue - unless the action is cancelled into another action.
    pub fn directive_linger(&self) -> TnuaActionLifecycleDirective {
        match self {
            TnuaActionLifecycleStatus::Initiated => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::CancelledFrom => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::StillFed => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::NoLongerFed => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::CancelledInto => TnuaActionLifecycleDirective::Finished,
        }
    }

    /// Determine if the action just started, whether from no action or to replace another action.
    pub fn just_started(&self) -> bool {
        match self {
            TnuaActionLifecycleStatus::Initiated => true,
            TnuaActionLifecycleStatus::CancelledFrom => true,
            TnuaActionLifecycleStatus::StillFed => false,
            TnuaActionLifecycleStatus::NoLongerFed => false,
            TnuaActionLifecycleStatus::CancelledInto => false,
        }
    }

    /// Determine if the action is currently active - still fed and not replaced by another.
    pub fn is_active(&self) -> bool {
        match self {
            TnuaActionLifecycleStatus::Initiated => true,
            TnuaActionLifecycleStatus::CancelledFrom => true,
            TnuaActionLifecycleStatus::StillFed => true,
            TnuaActionLifecycleStatus::NoLongerFed => false,
            TnuaActionLifecycleStatus::CancelledInto => false,
        }
    }
}

/// A decision by [`TnuaAction::apply`] that determines if the action should be continued or not.
///
/// Note that an action may continue (probably with different state) after no longer being fed, or
/// stopped while still being fed. It's up to the action, and it should be responsible with it.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum TnuaActionLifecycleDirective {
    /// The action should continue in the next frame.
    StillActive,

    /// The action should not continue in the next frame.
    ///
    /// If another action is pending, it will run in this frame. This means that two actions can
    /// run in the same frame, as long as the first is finished (or
    /// [rescheduled](Self::Reschedule))
    ///
    /// If [`TnuaAction::apply`] returns this but the action is still being fed, it will not run
    /// again unless it stops being fed for one frame and then gets fed again. If this is not the
    /// desired behavior, [`TnuaActionLifecycleDirective::Reschedule`] should be used instead.
    Finished,

    /// The action should not continue in the next frame, but if its still being fed it run again
    /// later. The rescheduled action will be considered a new action.
    ///
    /// If another action is pending, it will run in this frame. This means that two actions can
    /// run in the same frame, as long as the first is rescheduled (or [finished](Self::Finished))
    Reschedule {
        /// Only reschedule the action after this much time has passed.
        after_seconds: Float,
    },
}

/// A decision by [`TnuaAction::initiation_decision`] that determines if the action can start.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TnuaActionInitiationDirective {
    /// The action will not start as long as the input is still fed. In order to start it, the
    /// input must be released for at least one frame and then start being fed again.
    Reject,

    /// The action will not start this frame, but if the input is still fed next frame
    /// [`TnuaAction::initiation_decision`] will be checked again.
    Delay,

    /// The action can start this frame.
    Allow,
}

/// A character movement command for performing special actions.
///
/// "Special" does not necessarily mean **that** special - even
/// [jumping](crate::builtins::TnuaBuiltinJump) or [crouching](crate::builtins::TnuaBuiltinCrouch)
/// are considered [`TnuaAction`]s. Unlike basis - which is something constant - an action is
/// usually something more momentarily that has a flow.
///
/// The type that implements this trait is called the action _input_, and is expected to be
/// overwritten each frame by the controller system of the game code - although unlike basis the
/// input will probably be the exact same. Configuration is stored in an asset, as part of a struct
/// implementing [`TnuaSchemeConfig`] which holds the configuration for the basis and all the
/// actions. If the action needs to persist data between frames it must keep it in its
/// [memory](TnuaAction::Memory).
pub trait TnuaAction<B: TnuaBasis>: 'static + Send + Sync {
    type Config: Clone;

    /// Data that the action can persist between frames.
    ///
    /// The action will typically update this in its [`apply`](Self::apply). It has three purposes:
    ///
    /// 1. Store data that cannot be calculated on the spot. For example - the part of the jump
    ///    the character is currently at.
    ///
    /// 2. Pass data from the action to Tnua's internal mechanisms.
    ///
    /// 3. Inspect the action from game code systems, like an animation controlling system that
    ///    needs to know which animation to play based on the action's current state.
    type Memory: Send + Sync + Default;

    /// Decides whether the action can start.
    ///
    /// The difference between rejecting the action here with
    /// [`TnuaActionInitiationDirective::Reject`] or [`TnuaActionInitiationDirective::Delay`] and
    /// approving it with [`TnuaActionInitiationDirective::Allow`] only to do nothing in it and
    /// terminate with [`TnuaActionLifecycleDirective::Finished`] on the first frame, is that if
    /// some other action is currently running, in the former that action will continue to be
    /// active, while in the latter it'll be cancelled into this new action - which, having being
    /// immediately finished, will leave the controller with no active action, or with some third
    /// action if there is one.
    fn initiation_decision(
        &self,
        config: &Self::Config,
        sensors: &B::Sensors<'_>,
        ctx: TnuaActionContext<B>,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective;

    /// This is where the action affects the character's motion.
    ///
    /// This method gets called each frame to let the action control the [`TnuaMotor`] that will
    /// later move the character. Note that this happens the motor was set by the basis'
    /// [`apply`](TnuaBasis::apply). Here the action can modify some aspects of or even completely
    /// overwrite what the basis did.
    ///                                                              
    /// It can also update the memory.
    ///
    /// The returned value of this action determines whether or not the action will continue in the
    /// next frame.
    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        sensors: &B::Sensors<'_>,
        ctx: TnuaActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;

    /// An action can use this method to send information back to the basis' memory.
    ///
    /// For example - a jump action can use that to violate the basis' coyote time.
    #[allow(unused_variables)]
    fn influence_basis(
        &self,
        config: &Self::Config,
        memory: &Self::Memory,
        ctx: TnuaBasisContext,
        basis_input: &B,
        basis_config: &B::Config,
        basis_memory: &mut B::Memory,
    ) {
    }
}

pub trait TnuaActionDiscriminant:
    'static + Send + Sync + Copy + Clone + PartialEq + Eq + core::fmt::Debug
{
    fn variant_idx(&self) -> usize;
}

pub trait TnuaActionState: 'static + Send + Sync {
    type Basis: TnuaBasis;
    type Discriminant: TnuaActionDiscriminant;

    fn discriminant(&self) -> Self::Discriminant;

    fn variant_idx(&self) -> usize {
        self.discriminant().variant_idx()
    }

    fn interface(&self) -> &dyn TnuaActionStateInterface<Self::Basis>;
    fn interface_mut(&mut self) -> &mut dyn TnuaActionStateInterface<Self::Basis>;

    fn modify_basis_config(&self, basis_config: &mut <Self::Basis as TnuaBasis>::Config);
}

#[derive(Clone)]
pub struct TnuaBasisAccess<'a, B: TnuaBasis> {
    pub input: &'a B,
    pub config: &'a B::Config,
    pub memory: &'a B::Memory,
}

/// Various data passed to [`TnuaAction::apply`].
pub struct TnuaActionContext<'a, B: TnuaBasis> {
    /// The duration of the current frame.
    pub frame_duration: Float,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a TnuaRigidBodyTracker,

    /// The direction considered as "up".
    pub up_direction: Dir3,

    /// An accessor to the basis.
    pub basis: &'a TnuaBasisAccess<'a, B>,
}

impl<'a, B: TnuaBasis> TnuaActionContext<'a, B> {
    /// "Downgrade" to a basis context.
    ///
    /// This is useful for some helper methods of [the concrete basis and its
    /// state](Self::concrete_basis) that require a basis context.
    pub fn as_basis_context(&self) -> TnuaBasisContext<'a> {
        TnuaBasisContext {
            frame_duration: self.frame_duration,
            tracker: self.tracker,
            up_direction: self.up_direction,
        }
    }

    pub fn frame_duration_as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.frame_duration.into())
    }
}

pub trait TnuaConfigModifier<C> {
    fn modify_config(&self, config: &mut C);
}
