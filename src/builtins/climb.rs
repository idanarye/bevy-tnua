use bevy::prelude::*;

use crate::{
    math::Vector3, TnuaAction, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor,
};

pub struct TnuaBuiltinClimb {
    pub climbable_entity: Option<Entity>,
    /// The direction used to initiate the climb.
    ///
    /// This field is not used by the action itself. It's purpose is to help user controller
    /// systems determine if the player input is a continuation of the motion used to initiate the
    /// climb, or if it's a motion for breaking from the climb.
    pub initiation_direction: Vector3,
}

impl TnuaAction for TnuaBuiltinClimb {
    const NAME: &'static str = "TnuaBuiltinClimb";

    type State = TnuaBuiltinClimbState;

    const VIOLATES_COYOTE_TIME: bool = true;

    fn apply(
        &self,
        _state: &mut Self::State,
        _ctx: crate::TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        _motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        lifecycle_status.directive_simple()
    }

    fn initiation_decision(
        &self,
        _ctx: crate::TnuaActionContext,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        TnuaActionInitiationDirective::Allow
    }
}

#[derive(Default, Debug)]
pub struct TnuaBuiltinClimbState {}
