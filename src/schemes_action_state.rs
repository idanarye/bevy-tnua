use bevy_tnua_physics_integration_layer::data_for_backends::TnuaMotor;

use crate::schemes_traits::{Tnua2Action, Tnua2ActionContext, Tnua2Basis};
use crate::{TnuaActionLifecycleDirective, TnuaActionLifecycleStatus};

pub struct Tnua2ActionState<A: Tnua2Action<B>, B: Tnua2Basis> {
    input: A,
    config: A::Config,
    memory: A::Memory,
}

impl<A: Tnua2Action<B>, B: Tnua2Basis> Tnua2ActionState<A, B> {
    pub fn new(input: A, config: &A::Config) -> Self {
        Self {
            input,
            config: config.clone(),
            memory: Default::default(),
        }
    }
}

pub trait Tnua2ActionStateInterface<B: Tnua2Basis> {
    fn apply(
        &mut self,
        ctx: Tnua2ActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;
}

impl<A: Tnua2Action<B>, B: Tnua2Basis> Tnua2ActionStateInterface<B> for Tnua2ActionState<A, B> {
    fn apply(
        &mut self,
        ctx: Tnua2ActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        self.input
            .apply(&self.config, &mut self.memory, ctx, lifecycle_status, motor)
    }
}
