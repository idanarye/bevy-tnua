use bevy::prelude::*;

use crate::basis_action_traits::{
    TnuaAction, TnuaActionContext, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus,
};

pub struct Jump {
    pub height: f32,
}

impl TnuaAction for Jump {
    type State = ();

    fn apply(
        &self,
        _state: &mut Self::State,
        _ctx: TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        _motor: &mut crate::TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        info!("Jump {:?}. lifecycle {:?}", self.height, lifecycle_status);
        lifecycle_status.directive_simple()
    }
}
