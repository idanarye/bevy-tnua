// use bevy::prelude::*;

use std::any::Any;
use std::time::Duration;

use crate::{TnuaMotor, TnuaProximitySensor, TnuaRigidBodyTracker};

pub struct TnuaBasisContext<'a> {
    pub frame_duration: Duration,
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
