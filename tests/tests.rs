use bevy::prelude::*;
use bevy_tnua::builtins::{
    TnuaBuiltinClimb, TnuaBuiltinCrouch, TnuaBuiltinDash, TnuaBuiltinKnockback,
    TnuaBuiltinWalkConfig, TnuaBuiltinWallSlide,
};
use bevy_tnua::math::*;
use bevy_tnua::prelude::*;

#[test]
fn scheme_with_all_actions() {
    #[derive(TnuaScheme)]
    #[scheme(basis = TnuaBuiltinWalk)]
    #[cfg_attr(
        feature = "serialize",
        scheme(serde),
        derive(serde::Serialize, serde::Deserialize)
    )]
    #[allow(dead_code)]
    enum ControlScheme {
        Jump(TnuaBuiltinJump),
        Crouch(TnuaBuiltinCrouch),
        Dash(TnuaBuiltinDash),
        Knockback(TnuaBuiltinKnockback),
        WallSlide(TnuaBuiltinWallSlide),
        Climb(TnuaBuiltinClimb),
    }

    let mut controller = TnuaController::<ControlScheme>::default();

    controller.initiate_action_feeding();
    controller.basis = TnuaBuiltinWalk {
        desired_motion: Vector3::X,
        desired_forward: Some(Dir3::Z),
    };
    controller.action(ControlScheme::Jump(Default::default()));

    #[cfg(feature = "serialize")]
    {
        use bevy_tnua::TnuaGhostOverwrites;

        let serialized =
            ron::to_string(&(controller, TnuaGhostOverwrites::<ControlScheme>::default()))
                .expect("Unable to serialize");

        let (deserialized_controller, _): (
            TnuaController<ControlScheme>,
            TnuaGhostOverwrites<ControlScheme>,
        ) = ron::from_str(&serialized).expect(&format!("Could not deserialize {serialized}"));

        assert_eq!(deserialized_controller.basis.desired_motion, Vector3::X);
        assert_eq!(deserialized_controller.basis.desired_forward, Some(Dir3::Z));
    }

    // The config should always be serializable

    let config = ControlSchemeConfig {
        basis: TnuaBuiltinWalkConfig {
            float_height: 42.0,
            ..Default::default()
        },
        jump: Default::default(),
        crouch: Default::default(),
        dash: Default::default(),
        knockback: Default::default(),
        wall_slide: Default::default(),
        climb: Default::default(),
    };

    let serialized_config = ron::to_string(&config).expect("Unable to serialize the configuration");

    let deserialized_config: ControlSchemeConfig =
        ron::from_str(&serialized_config).expect("Unable to deserialize the configuration");

    assert_eq!(deserialized_config.basis.float_height, 42.0);
}

#[test]
fn scheme_with_no_actions() {
    #[derive(TnuaScheme)]
    #[scheme(basis = TnuaBuiltinWalk)]
    enum ControlScheme {}

    let mut controller = TnuaController::<ControlScheme>::default();

    controller.basis = TnuaBuiltinWalk {
        desired_motion: Vector3::new(1.0, 0.0, 0.0),
        desired_forward: None,
    };
}
