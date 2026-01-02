use bevy::prelude::*;
use bevy_tnua::builtins::{
    TnuaBuiltinClimb, TnuaBuiltinCrouch, TnuaBuiltinDash, TnuaBuiltinKnockback,
    TnuaBuiltinWallSlide,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaGhostOverwrites, math::*};
use serde::{Deserialize, Serialize};

#[test]
fn scheme_with_all_actions() {
    #[derive(TnuaScheme)]
    #[scheme(basis = TnuaBuiltinWalk)]
    #[derive(Serialize, Deserialize)]
    #[allow(dead_code)]
    enum ControlScheme {
        Jump(TnuaBuiltinJump),
        Crouch(TnuaBuiltinCrouch),
        Dash(TnuaBuiltinDash),
        Knockback(TnuaBuiltinKnockback),
        WallSlide(TnuaBuiltinWallSlide),
        Climb(TnuaBuiltinClimb),
    }

    let mut controller = TnuaController::<ControlScheme>::new(Default::default());

    controller.initiate_action_feeding();
    controller.basis = TnuaBuiltinWalk {
        desired_motion: Vector3::X,
        desired_forward: Some(Dir3::Z),
    };
    controller.action(ControlScheme::Jump(Default::default()));

    let ghost_overrides = TnuaGhostOverwrites::<ControlScheme>::default();

    let serialized =
        bevy::asset::ron::to_string(&(controller, ghost_overrides)).expect("Unable to serialize");

    let (deserialized_controller, _): (
        TnuaController<ControlScheme>,
        TnuaGhostOverwrites<ControlScheme>,
    ) = bevy::asset::ron::from_str(&serialized)
        .expect(&format!("Could not deserialize {serialized}"));

    assert_eq!(deserialized_controller.basis.desired_motion, Vector3::X);
    assert_eq!(deserialized_controller.basis.desired_forward, Some(Dir3::Z));
}

#[test]
fn scheme_with_no_actions() {
    #[derive(TnuaScheme)]
    #[scheme(basis = TnuaBuiltinWalk)]
    enum ControlScheme {}

    let mut controller = TnuaController::<ControlScheme>::new(Default::default());

    controller.basis = TnuaBuiltinWalk {
        desired_motion: Vector3::new(1.0, 0.0, 0.0),
        desired_forward: None,
    };
}
