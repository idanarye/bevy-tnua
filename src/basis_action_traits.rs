use bevy::prelude::*;
use bevy::time::Stopwatch;

use std::any::Any;

use crate::{TnuaMotor, TnuaProximitySensor, TnuaRigidBodyTracker};

pub struct TnuaBasisContext<'a> {
    pub frame_duration: f32,
    pub tracker: &'a TnuaRigidBodyTracker,
    pub proximity_sensor: &'a TnuaProximitySensor,
}

pub trait TnuaBasis: 'static + Send + Sync {
    type State: Default + Send + Sync;

    fn apply(&self, state: &mut Self::State, ctx: TnuaBasisContext, motor: &mut TnuaMotor);
    fn proximity_sensor_cast_range(&self) -> f32 {
        0.0
    }

    fn up_direction(&self, _state: &Self::State) -> Vec3;
    fn displacement(&self, _state: &Self::State) -> Option<Vec3>;
}

pub trait DynamicBasis: Send + Sync + Any + 'static {
    fn apply(&mut self, ctx: TnuaBasisContext, motor: &mut TnuaMotor);
    fn proximity_sensor_cast_range(&self) -> f32;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn up_direction(&self) -> Vec3;
    fn displacement(&self) -> Option<Vec3>;
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
    fn apply(&mut self, ctx: TnuaBasisContext, motor: &mut TnuaMotor) {
        self.input.apply(&mut self.state, ctx, motor);
    }

    fn proximity_sensor_cast_range(&self) -> f32 {
        self.input.proximity_sensor_cast_range()
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn up_direction(&self) -> Vec3 {
        self.input.up_direction(&self.state)
    }

    fn displacement(&self) -> Option<Vec3> {
        self.input.displacement(&self.state)
    }
}

pub struct TnuaActionContext<'a> {
    pub frame_duration: f32,
    pub tracker: &'a TnuaRigidBodyTracker,
    pub proximity_sensor: &'a TnuaProximitySensor,
    pub basis: &'a dyn DynamicBasis,
}

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
    pub fn directive_simple(&self) -> TnuaActionLifecycleDirective {
        match self {
            TnuaActionLifecycleStatus::Initiated => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::CancelledFrom => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::StillFed => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::NoLongerFed => TnuaActionLifecycleDirective::Finished,
            TnuaActionLifecycleStatus::CancelledInto => TnuaActionLifecycleDirective::Finished,
        }
    }

    pub fn just_started(&self) -> bool {
        match self {
            TnuaActionLifecycleStatus::Initiated => true,
            TnuaActionLifecycleStatus::CancelledFrom => true,
            TnuaActionLifecycleStatus::StillFed => false,
            TnuaActionLifecycleStatus::NoLongerFed => false,
            TnuaActionLifecycleStatus::CancelledInto => false,
        }
    }

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

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TnuaActionLifecycleDirective {
    StillActive,
    Finished,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TnuaActionInitiationDirective {
    Reject,
    Delay,
    Allow,
}

pub trait TnuaAction: 'static + Send + Sync {
    type State: Default + Send + Sync;

    fn apply(
        &self,
        state: &mut Self::State,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;

    fn proximity_sensor_cast_range(&self) -> f32 {
        0.0
    }

    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective {
        if 0.2 < being_fed_for.elapsed().as_secs_f32() {
            TnuaActionInitiationDirective::Reject
        } else if ctx.proximity_sensor.output.is_some() {
            TnuaActionInitiationDirective::Allow
        } else {
            TnuaActionInitiationDirective::Delay
        }
    }
}

pub(crate) trait DynamicAction: Send + Sync + Any + 'static {
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn apply(
        &mut self,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;
    fn proximity_sensor_cast_range(&self) -> f32;
    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective;
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

    fn proximity_sensor_cast_range(&self) -> f32 {
        self.input.proximity_sensor_cast_range()
    }

    fn initiation_decision(
        &self,
        ctx: TnuaActionContext,
        being_fed_for: &Stopwatch,
    ) -> TnuaActionInitiationDirective {
        self.input.initiation_decision(ctx, being_fed_for)
    }
}
