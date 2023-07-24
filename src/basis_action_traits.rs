// use bevy::prelude::*;

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
}

pub(crate) trait DynamicBasis: Send + Sync + Any + 'static {
    fn apply(&mut self, ctx: TnuaBasisContext, motor: &mut TnuaMotor);
    fn proximity_sensor_cast_range(&self) -> f32;
    fn as_mut_any(&mut self) -> &mut dyn Any;
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
}

pub struct TnuaActionContext<'a> {
    pub frame_duration: f32,
    pub tracker: &'a TnuaRigidBodyTracker,
    pub proximity_sensor: &'a TnuaProximitySensor,
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
    pub(crate) fn directive_simple(&self) -> TnuaActionLifecycleDirective {
        match self {
            TnuaActionLifecycleStatus::Initiated => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::CancelledFrom => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::StillFed => TnuaActionLifecycleDirective::StillActive,
            TnuaActionLifecycleStatus::NoLongerFed => TnuaActionLifecycleDirective::Finished,
            TnuaActionLifecycleStatus::CancelledInto => TnuaActionLifecycleDirective::Finished,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TnuaActionLifecycleDirective {
    StillActive,
    Finished,
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
}

pub(crate) trait DynamicAction: Send + Sync + Any + 'static {
    fn apply(
        &mut self,
        ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;
    fn proximity_sensor_cast_range(&self) -> f32;
    fn as_mut_any(&mut self) -> &mut dyn Any;
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

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}
