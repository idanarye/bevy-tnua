use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_tnua_physics_integration_layer::math::{Float, Vector3};

use std::{any::Any, time::Duration};

use crate::{TnuaMotor, TnuaProximitySensor, TnuaRigidBodyTracker};

/// Various data passed to [`TnuaBasis::apply`].
pub struct TnuaBasisContext<'a> {
    /// The duration of the current frame.
    pub frame_duration: Float,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a TnuaRigidBodyTracker,

    /// A sensor that tracks the distance of the character's center from the ground.
    pub proximity_sensor: &'a TnuaProximitySensor,

    /// The direction considered as "up".
    pub up_direction: Dir3,
}

/// The main movement command of a character.
///
/// A basis handles the character's motion when the user is not feeding it any input, or when it
/// just moves around without doing anything special. A simple game would only need once basis -
/// [`TnuaBuiltinWalk`](crate::builtins::TnuaBuiltinWalk) - but more complex games can have bases
/// for things like swimming or driving.
///
/// The type that implements this trait is called the basis _input_, and is expected to be
/// overwritten each frame by the controller system of the game code. Configuration is considered
/// as part of the input. If the basis needs to persist data between frames it must keep it in its
/// [state](Self::State).
pub trait TnuaBasis: 'static + Send + Sync {
    /// The default name of the basis.
    ///
    /// [Once `type_name` becomes `const`](https://github.com/rust-lang/rust/issues/63084), this
    /// will default to it. For now, just set it to the name of the type.
    const NAME: &'static str;

    /// Data that the basis can persist between frames.
    ///
    /// The basis will typically update this in its [`apply`](Self::apply). It has three purposes:
    ///
    /// 1. Store data that cannot be calculated on the spot. For example - a timer for tracking
    ///    coyote time.
    ///
    /// 2. Pass data from the basis to the action (or to Tnua's internal mechanisms)
    ///
    /// 3. Inspect the basis from game code systems, like an animation controlling system that
    ///    needs to know which animation to play based on the basis' current state.
    type State: Default + Send + Sync;

    /// This is where the basis affects the character's motion.
    ///
    /// This method gets called each frame to let the basis control the [`TnuaMotor`] that will
    /// later move the character.
    ///
    /// Note that after the motor is set in this method, if there is an action going on, the
    /// action's [`apply`](TnuaAction::apply) will also run and typically change some of the things
    /// the basis did to the motor.
    ///                                                              
    /// It can also update the state.
    fn apply(&self, state: &mut Self::State, ctx: TnuaBasisContext, motor: &mut TnuaMotor);

    /// A value to configure the range of the ground proximity sensor according to the basis'
    /// needs.
    fn proximity_sensor_cast_range(&self, state: &Self::State) -> Float;

    /// The displacement of the character from where the basis wants it to be.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn displacement(&self, state: &Self::State) -> Option<Vector3>;

    /// The velocity of the character, relative the what the basis considers its frame of
    /// reference.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn effective_velocity(&self, state: &Self::State) -> Vector3;

    /// The vertical velocity the character requires to stay the same height if it wants to move in
    /// [`effective_velocity`](Self::effective_velocity).
    fn vertical_velocity(&self, state: &Self::State) -> Float;

    /// Nullify the fields of the basis that represent user input.
    fn neutralize(&mut self);

    /// Can be queried by an action to determine if the character should be considered "in the air".
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn is_airborne(&self, state: &Self::State) -> bool;

    /// If the basis is at coyote time - finish the coyote time.
    ///
    /// This will be called automatically by Tnua, if the controller runs an action that  [violated
    /// coyote time](TnuaAction::VIOLATES_COYOTE_TIME), so that a long coyote time will not allow,
    /// for example, unaccounted air jumps.
    ///
    /// If the character is fully grounded, this method must not change that.
    fn violate_coyote_time(&self, state: &mut Self::State);
}

/// Helper trait for accessing a basis and its trait with dynamic dispatch.
pub trait DynamicBasis: Send + Sync + Any + 'static {
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any;

    #[doc(hidden)]
    fn as_mut_any(&mut self) -> &mut dyn Any;

    #[doc(hidden)]
    fn apply(&mut self, ctx: TnuaBasisContext, motor: &mut TnuaMotor);

    /// Dynamically invokes [`TnuaBasis::proximity_sensor_cast_range`].
    fn proximity_sensor_cast_range(&self) -> Float;

    /// Dynamically invokes [`TnuaBasis::displacement`].
    fn displacement(&self) -> Option<Vector3>;

    /// Dynamically invokes [`TnuaBasis::effective_velocity`].
    fn effective_velocity(&self) -> Vector3;

    /// Dynamically invokes [`TnuaBasis::vertical_velocity`].
    fn vertical_velocity(&self) -> Float;

    /// Dynamically invokes [`TnuaBasis::neutralize`].
    fn neutralize(&mut self);

    /// Dynamically invokes [`TnuaBasis::is_airborne`].
    fn is_airborne(&self) -> bool;

    #[doc(hidden)]
    fn violate_coyote_time(&mut self);
}

pub(crate) struct BoxableBasis<B: TnuaBasis> {
    pub(crate) input: B,
    pub(crate) state: B::State,
}

impl<B: TnuaBasis> BoxableBasis<B> {
    pub(crate) fn new(basis: B) -> Self {
        Self {
            input: basis,
            state: Default::default(),
        }
    }
}

impl<B: TnuaBasis> DynamicBasis for BoxableBasis<B> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(&mut self, ctx: TnuaBasisContext, motor: &mut TnuaMotor) {
        self.input.apply(&mut self.state, ctx, motor);
    }

    fn proximity_sensor_cast_range(&self) -> Float {
        self.input.proximity_sensor_cast_range(&self.state)
    }

    fn displacement(&self) -> Option<Vector3> {
        self.input.displacement(&self.state)
    }

    fn effective_velocity(&self) -> Vector3 {
        self.input.effective_velocity(&self.state)
    }

    fn vertical_velocity(&self) -> Float {
        self.input.vertical_velocity(&self.state)
    }

    fn neutralize(&mut self) {
        self.input.neutralize();
    }

    fn is_airborne(&self) -> bool {
        self.input.is_airborne(&self.state)
    }

    fn violate_coyote_time(&mut self) {
        self.input.violate_coyote_time(&mut self.state)
    }
}

/// Various data passed to [`TnuaAction::apply`].
pub struct TnuaActionContext<'a> {
    /// The duration of the current frame.
    pub frame_duration: Float,

    /// A sensor that collects data about the rigid body from the physics backend.
    pub tracker: &'a TnuaRigidBodyTracker,

    /// A sensor that tracks the distance of the character's center from the ground.
    pub proximity_sensor: &'a TnuaProximitySensor,

    /// The direction considered as "up".
    pub up_direction: Dir3,

    /// An accessor to the currently active basis.
    pub basis: &'a dyn DynamicBasis,
}

impl<'a> TnuaActionContext<'a> {
    /// Can be used to get the concrete basis.
    ///
    /// Use with care - actions that use it will only be usable with one basis.
    pub fn concrete_basis<B: TnuaBasis>(&self) -> Option<(&B, &B::State)> {
        let boxable_basis: &BoxableBasis<B> = self.basis.as_any().downcast_ref()?;
        Some((&boxable_basis.input, &boxable_basis.state))
    }

    /// "Downgrade" to a basis context.
    ///
    /// This is useful for some helper methods of [the concrete basis and its
    /// state](Self::concrete_basis) that require a basis context.
    pub fn as_basis_context(&self) -> TnuaBasisContext<'a> {
        TnuaBasisContext {
            frame_duration: self.frame_duration,
            tracker: self.tracker,
            proximity_sensor: self.proximity_sensor,
            up_direction: self.up_direction,
        }
    }

    pub fn frame_duration_as_duration(&self) -> Duration {
        Duration::from_secs_f64(self.frame_duration.into())
    }
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
/// input will probably be the exact same. Configuration is considered as part of the input. If the
/// action needs to persist data between frames it must keep it in its [state](Self::State).
pub trait TnuaAction: 'static + Send + Sync {
    /// The default name of the action.
    ///
    /// [Once `type_name` becomes `const`](https://github.com/rust-lang/rust/issues/63084), this
    /// will default to it. For now, just set it to the name of the type.
    const NAME: &'static str;

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
    type State: Default + Send + Sync;

    /// Set this to true for actions that may launch the character into the air.
    const VIOLATES_COYOTE_TIME: bool;

    /// This is where the action affects the character's motion.
    ///
    /// This method gets called each frame to let the action control the [`TnuaMotor`] that will
    /// later move the character. Note that this happens the motor was set by the basis'
    /// [`apply`](TnuaBasis::apply). Here the action can modify some aspects of or even completely
    /// overwrite what the basis did.
    ///                                                              
    /// It can also update the state.
    ///
    /// The returned value of this action determines whether or not the action will continue in the
    /// next frame.
    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;

    /// A value to configure the range of the ground proximity sensor according to the action's
    /// needs.
    fn proximity_sensor_cast_range(&self) -> Float {
        0.0
    }

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
        ctx: TnuaActionContext,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective;
}

pub trait DynamicAction: Send + Sync + Any + 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn apply(
        &mut self,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;
    fn proximity_sensor_cast_range(&self) -> Float;
    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective;
    fn violates_coyote_time(&self) -> bool;
}

pub(crate) struct BoxableAction<A: TnuaAction> {
    pub(crate) input: A,
    pub(crate) state: A::State,
}

impl<A: TnuaAction> BoxableAction<A> {
    pub(crate) fn new(basis: A) -> Self {
        Self {
            input: basis,
            state: Default::default(),
        }
    }
}

impl<A: TnuaAction> DynamicAction for BoxableAction<A> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(
        &mut self,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        self.input
            .apply(&mut self.state, ctx, lifecycle_status, motor)
    }

    fn proximity_sensor_cast_range(&self) -> Float {
        self.input.proximity_sensor_cast_range()
    }

    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective {
        self.input.initiation_decision(ctx, being_fed_for)
    }

    fn violates_coyote_time(&self) -> bool {
        A::VIOLATES_COYOTE_TIME
    }
}
