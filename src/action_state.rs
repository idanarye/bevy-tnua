use bevy_tnua_physics_integration_layer::data_for_backends::TnuaMotor;

use crate::{TnuaAction, TnuaActionContext, TnuaBasis};
use crate::{TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, TnuaBasisContext};

pub struct TnuaActionState<A: TnuaAction<B>, B: TnuaBasis> {
    pub input: A,
    pub config: A::Config,
    pub memory: A::Memory,
}

impl<A: TnuaAction<B>, B: TnuaBasis> TnuaActionState<A, B> {
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

pub trait TnuaActionStateInterface<B: TnuaBasis> {
    fn apply(
        &mut self,
        ctx: TnuaActionContext<B>,
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

impl<A: TnuaAction<B>, B: TnuaBasis> TnuaActionStateInterface<B> for TnuaActionState<A, B> {
    fn apply(
        &mut self,
        ctx: TnuaActionContext<B>,
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
        basis_config: &<B as TnuaBasis>::Config,
        basis_memory: &mut <B as TnuaBasis>::Memory,
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
