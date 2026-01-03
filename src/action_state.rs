use bevy_tnua_physics_integration_layer::data_for_backends::TnuaMotor;
use serde::{Deserialize, Serialize};

use crate::{TnuaAction, TnuaActionContext, TnuaBasis};
use crate::{TnuaActionLifecycleDirective, TnuaActionLifecycleStatus, TnuaBasisContext};

/// The full state of a single [`TnuaAction`].
///
/// These are used in the variants of the [action state enum](crate::TnuaScheme::ActionState),
/// created automatically by [the `TnuaScheme` derive](bevy_tnua_macros::TnuaScheme).
#[derive(Serialize, Deserialize)]
pub struct TnuaActionState<A: TnuaAction<B>, B: TnuaBasis> {
    /// The data that the user control system feeds to the
    /// [`TnuaController`](crate::TnuaController).
    pub input: A,
    /// The action configuration, retrieved from an asset.
    pub config: A::Config,
    /// Initiated when the action starts, and gets updated by the action itself.
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
}

#[doc(hidden)]
pub trait TnuaActionStateInterface<B: TnuaBasis> {
    fn apply(
        &mut self,
        sensors: &B::Sensors<'_>,
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
        sensors: &B::Sensors<'_>,
        ctx: TnuaActionContext<B>,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        self.input.apply(
            &self.config,
            &mut self.memory,
            sensors,
            ctx,
            lifecycle_status,
            motor,
        )
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
