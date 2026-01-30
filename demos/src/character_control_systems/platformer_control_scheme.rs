use bevy::prelude::*;

use bevy_tnua::builtins::{
    TnuaBuiltinClimb, TnuaBuiltinClimbConfig, TnuaBuiltinCrouch, TnuaBuiltinCrouchConfig,
    TnuaBuiltinDash, TnuaBuiltinDashConfig, TnuaBuiltinJump, TnuaBuiltinJumpConfig,
    TnuaBuiltinKnockback, TnuaBuiltinWalk, TnuaBuiltinWalkConfig, TnuaBuiltinWalkHeadroom,
    TnuaBuiltinWallSlide, TnuaBuiltinWallSlideConfig,
};
use bevy_tnua::control_helpers::{TnuaActionSlots, TnuaAirActionDefinition, TnuaHasTargetEntity};
use bevy_tnua::math::*;
use bevy_tnua::{TnuaConfigModifier, TnuaScheme};
use serde::{Deserialize, Serialize};

use super::Dimensionality;
use super::platformer_control_systems::FallingThroughControlScheme;

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk, config_ext = CharacterMotionConfigForPlatformerDemo)]
pub enum DemoControlScheme {
    Jump(TnuaBuiltinJump),
    Crouch(
        TnuaBuiltinCrouch,
        #[scheme(modify_basis_config)] SlowDownWhileCrouching,
    ),
    Dash(TnuaBuiltinDash),
    Knockback(TnuaBuiltinKnockback),
    WallSlide(TnuaBuiltinWallSlide, Entity),
    WallJump(TnuaBuiltinJump),
    Climb(
        TnuaBuiltinClimb,
        Entity,
        // Initiation direction:
        Vector3,
    ),
}

#[derive(Serialize, Deserialize)]
pub struct CharacterMotionConfigForPlatformerDemo {
    pub dimensionality: Dimensionality,
    pub jumps_in_air: usize,
    pub dashes_in_air: usize,
    pub one_way_platforms_min_proximity: Float,
    pub falling_through: FallingThroughControlScheme,
}

impl Default for CharacterMotionConfigForPlatformerDemo {
    fn default() -> Self {
        Self {
            dimensionality: Dimensionality::Dim3,
            jumps_in_air: 1,
            dashes_in_air: 1,
            one_way_platforms_min_proximity: 1.0,
            falling_through: FallingThroughControlScheme::SingleFall,
        }
    }
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

#[derive(Debug, TnuaActionSlots)]
#[slots(scheme = DemoControlScheme, ending(WallSlide, WallJump, Climb))]
pub struct DemoControlSchemeAirActions {
    #[slots(Jump)]
    jump: usize,
    #[slots(Dash)]
    dash: usize,
}

impl TnuaHasTargetEntity for DemoControlScheme {
    fn target_entity(action_state: &Self::ActionState) -> Option<bevy::ecs::entity::Entity> {
        match action_state {
            DemoControlSchemeActionState::Jump(_) => None,
            DemoControlSchemeActionState::Crouch(_, _) => None,
            DemoControlSchemeActionState::Dash(_) => None,
            DemoControlSchemeActionState::Knockback(_) => None,
            DemoControlSchemeActionState::WallSlide(_, entity) => Some(*entity),
            DemoControlSchemeActionState::WallJump(_) => None,
            DemoControlSchemeActionState::Climb(_, entity, _) => Some(*entity),
        }
    }
}

impl Default for DemoControlSchemeConfig {
    fn default() -> Self {
        Self {
            basis: TnuaBuiltinWalkConfig {
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
                horizontal_distance: 10.0,
                vertical_distance: 0.0,
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
                horizontal_distance: 2.0,
                ..Default::default()
            },
            climb: TnuaBuiltinClimbConfig {
                climb_speed: 10.0,
                ..Default::default()
            },
            ext: CharacterMotionConfigForPlatformerDemo {
                dimensionality: Dimensionality::Dim3,
                jumps_in_air: 1,
                dashes_in_air: 1,
                one_way_platforms_min_proximity: 1.0,
                falling_through: FallingThroughControlScheme::SingleFall,
            },
        }
    }
}
