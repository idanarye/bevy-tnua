use bevy::prelude::*;
use bevy_tnua::builtins::{TnuaBuiltinClimbMemory, TnuaBuiltinJumpMemory};
use bevy_tnua::math::{AdjustPrecision, Float, Vector3};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaAnimatingState, TnuaAnimatingStateDirective};

use crate::character_control_systems::platformer_control_scheme::{
    DemoControlScheme, DemoControlSchemeActionState,
};
use crate::util::animating::AnimationsHandler;

#[derive(Debug)]
pub enum AnimationState {
    Standing,
    Running(Float),
    Jumping,
    Falling,
    Crouching,
    Crawling(Float),
    Dashing,
    KnockedBack,
    WallSliding,
    WallJumping,
    Climbing(Float),
}

#[allow(clippy::unnecessary_cast)]
pub fn animate_platformer_character(
    mut animations_handlers_query: Query<(
        // `TnuaAnimatingState` is a helper for controlling the animations. The user system is
        // expected to provide it with an enum on every frame that describes the state of the
        // character. The helper then tells the user system if the enum variant changed - which
        // usually means the system should start a new animation - or remained the same, which
        // means that the system should not change the animation (but maybe change its speed based
        // on the enum's payload)
        &mut TnuaAnimatingState<AnimationState>,
        // The controller can be used to determine the state of the character - information crucial
        // for deciding which animation to play.
        &TnuaController<DemoControlScheme>,
        &AnimationsHandler,
    )>,
    mut animation_players_query: Query<&mut AnimationPlayer>,
) {
    for (mut animating_state, controller, handler) in animations_handlers_query.iter_mut() {
        let Ok(mut player) = animation_players_query.get_mut(handler.player_entity) else {
            continue;
        };
        // We need to determine the animating status of the character on each frame, and feed it to
        // `update_by_discriminant` which will decide whether or not we need to switch the
        // animation.
        match animating_state.update_by_discriminant({
            // We use the action name because it's faster than trying to cast into each action
            // type. We'd still have to cast into the action type later though, to get
            // action-specific data.
            match controller.current_action.as_ref() {
                // For builtin actions, prefer using the `NAME` const from the `TnuaAction` trait.
                Some(DemoControlSchemeActionState::Jump(state)) => {
                    // Depending on the state of the jump, we need to decide if we want to play the
                    // jump animation or the fall animation.
                    match state.memory {
                        TnuaBuiltinJumpMemory::NoJump => continue,
                        TnuaBuiltinJumpMemory::StartingJump { .. } => AnimationState::Jumping,
                        TnuaBuiltinJumpMemory::SlowDownTooFastSlopeJump { .. } => {
                            AnimationState::Jumping
                        }
                        TnuaBuiltinJumpMemory::MaintainingJump { .. } => AnimationState::Jumping,
                        TnuaBuiltinJumpMemory::StoppedMaintainingJump => AnimationState::Jumping,
                        TnuaBuiltinJumpMemory::FallSection => AnimationState::Falling,
                    }
                }
                Some(DemoControlSchemeActionState::Crouch(..)) => {
                    // In case of crouch, we need the state of the basis to determine - based on
                    // the speed - if the charcter is just crouching or also crawling.
                    let speed = Some(controller.basis_memory.running_velocity.length())
                        .filter(|speed| 0.01 < *speed);
                    let is_crouching = controller.basis_memory.standing_offset.dot(
                        controller
                            .up_direction()
                            .unwrap_or(Dir3::Y)
                            .adjust_precision(),
                    ) < -0.4;
                    match (speed, is_crouching) {
                        (None, false) => AnimationState::Standing,
                        (None, true) => AnimationState::Crouching,
                        (Some(speed), false) => AnimationState::Running(0.1 * speed),
                        (Some(speed), true) => AnimationState::Crawling(0.1 * speed),
                    }
                }
                // For the dash, we don't need the internal state of the dash action to determine
                // the action - so there is no need to downcast.
                Some(DemoControlSchemeActionState::Dash(_)) => AnimationState::Dashing,
                Some(DemoControlSchemeActionState::Knockback(_)) => AnimationState::KnockedBack,
                Some(DemoControlSchemeActionState::WallSlide(..)) => AnimationState::WallSliding,
                Some(DemoControlSchemeActionState::WallJump(_)) => AnimationState::WallJumping,
                Some(DemoControlSchemeActionState::Climb(state, ..)) => {
                    let TnuaBuiltinClimbMemory::Climbing { climbing_velocity } = state.memory
                    else {
                        continue;
                    };
                    AnimationState::Climbing(0.3 * climbing_velocity.dot(Vector3::Y))
                }
                None => {
                    // If there is no action going on, we'll base the animation on the state of the
                    // basis.
                    if controller.basis_memory.standing_on_entity().is_none() {
                        AnimationState::Falling
                    } else {
                        let speed = controller.basis_memory.running_velocity.length();
                        if 0.01 < speed {
                            AnimationState::Running(0.1 * speed)
                        } else {
                            AnimationState::Standing
                        }
                    }
                }
            }
        }) {
            // `Maintain` means that the same animation state continues from the previous frame, so
            // we shouldn't switch the animation.
            TnuaAnimatingStateDirective::Maintain { state } => match state {
                // Some animation states have parameters, that we may want to use to control the
                // animation (without necessarily replacing it). In this case - control the speed
                // of the animation based on the speed of the movement.
                AnimationState::Running(speed)
                | AnimationState::Crawling(speed)
                | AnimationState::Climbing(speed) => {
                    for (_, active_animation) in player.playing_animations_mut() {
                        active_animation.set_speed(*speed as f32);
                    }
                }
                // Jumping and dashing can be chained, we want to start a new jump/dash animation
                // when one jump/dash is chained to another.
                AnimationState::Jumping | AnimationState::Dashing => {
                    if controller.action_flow_status().just_starting().is_some() {
                        player.seek_all_by(0.0);
                    }
                }
                // For other animations we don't have anything special to do - so we just let them
                // continue.
                _ => {}
            },
            // `Alter` means that the character animation state has changed, and thus we need to
            // start a new animation. The actual implementation for each possible animation state
            // is straightforward - we start the animation, set its speed if the state has a
            // variable speed, and set it to repeat if it's something that needs to repeat.
            TnuaAnimatingStateDirective::Alter {
                old_state: _,
                state,
            } => {
                player.stop_all();
                match state {
                    AnimationState::Standing => {
                        player
                            .start(handler.animations["Standing"])
                            .set_speed(1.0)
                            .repeat();
                    }
                    AnimationState::Running(speed) => {
                        player
                            .start(handler.animations["Running"])
                            .set_speed(*speed as f32)
                            .repeat();
                    }
                    AnimationState::Jumping => {
                        player.start(handler.animations["Jumping"]).set_speed(2.0);
                    }
                    AnimationState::Falling => {
                        player.start(handler.animations["Falling"]).set_speed(1.0);
                    }
                    AnimationState::Crouching => {
                        player
                            .start(handler.animations["Crouching"])
                            .set_speed(1.0)
                            .repeat();
                    }
                    AnimationState::Crawling(speed) => {
                        player
                            .start(handler.animations["Crawling"])
                            .set_speed(*speed as f32)
                            .repeat();
                    }
                    AnimationState::Dashing => {
                        player.start(handler.animations["Dashing"]).set_speed(10.0);
                    }
                    AnimationState::KnockedBack => {
                        player
                            .start(handler.animations["KnockedBack"])
                            .set_speed(1.0);
                    }
                    AnimationState::WallSliding => {
                        player
                            .start(handler.animations["WallSliding"])
                            .set_speed(1.0)
                            .repeat();
                    }
                    AnimationState::WallJumping => {
                        player
                            .start(handler.animations["WallJumping"])
                            .set_speed(2.0);
                    }
                    AnimationState::Climbing(speed) => {
                        player
                            .start(handler.animations["VineClimbing"])
                            .set_speed(*speed as f32)
                            .repeat()
                            .set_speed(1.0);
                    }
                }
            }
        }
    }
}
