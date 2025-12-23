use bevy_tnua_physics_integration_layer::data_for_backends::TnuaMotor;

use crate::schemes_traits::{Tnua2Action, Tnua2ActionContext, Tnua2Basis};
use crate::{TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, TnuaBasisContext};

pub struct Tnua2ActionState<A: Tnua2Action<B>, B: Tnua2Basis> {
    pub input: A,
    pub config: A::Config,
    pub memory: A::Memory,
}

impl<A: Tnua2Action<B>, B: Tnua2Basis> Tnua2ActionState<A, B> {
    pub fn new(input: A, config: &A::Config) -> Self {
        Self {
            input,
            config: config.clone(),
            memory: Default::default(),
        }
    }

    pub fn update_input(&mut self, input: A) {
        self.input = input;
    }
}

pub trait Tnua2ActionStateInterface<B: Tnua2Basis> {
    fn apply(
        &mut self,
        ctx: Tnua2ActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective;

    fn influence_basis(
        &self,
        ctx: TnuaBasisContext,
        basis_input: &B,
        basis_config: &B::Config,
        basis_memory: &mut B::Memory,
    );
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

    fn influence_basis(
        &self,
        ctx: TnuaBasisContext,
        basis_input: &B,
        basis_config: &<B as Tnua2Basis>::Config,
        basis_memory: &mut <B as Tnua2Basis>::Memory,
    ) {
        self.input.influence_basis(
            &self.config,
            &self.memory,
            ctx,
            basis_input,
            basis_config,
            basis_memory,
        );
    }
}
