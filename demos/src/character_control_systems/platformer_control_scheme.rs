use bevy_tnua::builtins::{
    TnuaBuiltinClimb, TnuaBuiltinClimbConfig, TnuaBuiltinCrouch, TnuaBuiltinCrouchConfig,
    TnuaBuiltinDash, TnuaBuiltinDashConfig, TnuaBuiltinJump, TnuaBuiltinJumpConfig,
    TnuaBuiltinKnockback, TnuaBuiltinWalk, TnuaBuiltinWalkConfig, TnuaBuiltinWalkHeadroom,
    TnuaBuiltinWallSlide, TnuaBuiltinWallSlideConfig,
};
use bevy_tnua::control_helpers::{TnuaAirActionDefinition, TnuaHasTargetEntity};
use bevy_tnua::math::*;
use bevy_tnua::{TnuaConfigModifier, TnuaScheme};

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum DemoControlScheme {
    Jump(TnuaBuiltinJump),
    Crouch(
        TnuaBuiltinCrouch,
        #[scheme(modify_basis_config)] SlowDownWhileCrouching,
    ),
    Dash(TnuaBuiltinDash),
    Knockback(TnuaBuiltinKnockback),
    WallSlide(TnuaBuiltinWallSlide),
    WallJump(TnuaBuiltinJump),
    Climb(TnuaBuiltinClimb),
}

pub struct SlowDownWhileCrouching(pub bool);

impl TnuaConfigModifier<TnuaBuiltinWalkConfig> for SlowDownWhileCrouching {
    fn modify_config(&self, config: &mut TnuaBuiltinWalkConfig) {
        if self.0 {
            config.speed *= 0.2;
        }
    }
}

impl TnuaAirActionDefinition for DemoControlScheme {
    fn is_air_action(action: Self::ActionDiscriminant) -> bool {
        match action {
            DemoControlSchemeActionDiscriminant::Jump => true,
            DemoControlSchemeActionDiscriminant::Crouch => false,
            DemoControlSchemeActionDiscriminant::Dash => true,
            DemoControlSchemeActionDiscriminant::Knockback => true,
            DemoControlSchemeActionDiscriminant::WallSlide => true,
            DemoControlSchemeActionDiscriminant::WallJump => true,
            DemoControlSchemeActionDiscriminant::Climb => true,
        }
    }
}

impl TnuaHasTargetEntity for DemoControlScheme {
    fn target_entity(action_state: &Self::ActionState) -> Option<bevy::ecs::entity::Entity> {
        match action_state {
            DemoControlSchemeActionState::Jump(_) => None,
            DemoControlSchemeActionState::Crouch(_, _) => None,
            DemoControlSchemeActionState::Dash(_) => None,
            DemoControlSchemeActionState::Knockback(_) => None,
            DemoControlSchemeActionState::WallSlide(_) => None, // maybe it should have?
            DemoControlSchemeActionState::WallJump(_) => None,  // maybe it should have?
            DemoControlSchemeActionState::Climb(state) => state.input.climbable_entity,
        }
    }
}

impl DemoControlSchemeConfig {
    pub fn new_with_speed(walk_speed: Float) -> Self {
        Self {
            basis: TnuaBuiltinWalkConfig {
                speed: walk_speed,
                float_height: 2.0,
                headroom: Some(TnuaBuiltinWalkHeadroom {
                    distance_to_collider_top: 1.0,
                    ..Default::default()
                }),
                max_slope: float_consts::FRAC_PI_4,
                ..Default::default()
            },
            jump: TnuaBuiltinJumpConfig {
                height: 4.0,
                ..Default::default()
            },
            crouch: TnuaBuiltinCrouchConfig {
                float_offset: -0.9,
                ..Default::default()
            },
            dash: TnuaBuiltinDashConfig {
                // TODO: add dash distance of 10.0?
                ..Default::default()
            },
            knockback: Default::default(),
            wall_slide: TnuaBuiltinWallSlideConfig {
                maintain_distance: Some(0.7),
                ..Default::default()
            },
            wall_jump: TnuaBuiltinJumpConfig {
                height: 4.0,
                takeoff_extra_gravity: 90.0, // 3 times the default
                takeoff_above_velocity: 0.0,
                ..Default::default()
            },
            climb: TnuaBuiltinClimbConfig {
                climb_speed: 10.0,
                ..Default::default()
            },
        }
    }
}
