use bevy::ecs::query::QueryData;
use bevy::prelude::*;
use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinDash, TnuaBuiltinJumpState};
use bevy_tnua::math::Float;
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaAnimatingState, TnuaAnimatingStateDirective};

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
    KnockedBack(Dir3),
}

impl AnimationState {
    fn force_forward(&self) -> Option<Dir3> {
        match self {
            AnimationState::Standing => None,
            AnimationState::Running(_) => None,
            AnimationState::Jumping => None,
            AnimationState::Falling => None,
            AnimationState::Crouching => None,
            AnimationState::Crawling(_) => None,
            AnimationState::Dashing => None,
            AnimationState::KnockedBack(direction) => Some(-*direction),
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct ForwardForcing {
    pub transform: &'static mut Transform,
    #[cfg(feature = "rapier3d")]
    rapier3d_locked_axes: &'static mut bevy_rapier3d::prelude::LockedAxes,
    #[cfg(feature = "avian3d")]
    avian3d_locked_axes: &'static mut avian3d::prelude::LockedAxes,
}

impl ForwardForcingItem<'_> {
    fn lock_rotation(&mut self) {
        #[cfg(feature = "rapier3d")]
        self.rapier3d_locked_axes
            .insert(bevy_rapier3d::prelude::LockedAxes::ROTATION_LOCKED_Y);
        #[cfg(feature = "avian3d")]
        {
            *self.avian3d_locked_axes = self.avian3d_locked_axes.lock_rotation_y();
        }
    }

    fn unlock_rotation(&mut self) {
        #[cfg(feature = "rapier3d")]
        self.rapier3d_locked_axes
            .remove(bevy_rapier3d::prelude::LockedAxes::ROTATION_LOCKED_Y);
        #[cfg(feature = "avian3d")]
        {
            *self.avian3d_locked_axes = self.avian3d_locked_axes.unlock_rotation_y();
        }
    }
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
        &TnuaController,
        &AnimationsHandler,
        ForwardForcing,
    )>,
    mut animation_players_query: Query<&mut AnimationPlayer>,
) {
    for (mut animating_state, controller, handler, mut forward_forcing) in
        animations_handlers_query.iter_mut()
    {
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
            match controller.action_name() {
                // For builtin actions, prefer using the `NAME` const from the `TnuaAction` trait.
                Some(TnuaBuiltinJump::NAME) => {
                    // In case of jump, we want to cast it so that we can get the concrete jump
                    // state.
                    let (_, jump_state) = controller
                        .concrete_action::<TnuaBuiltinJump>()
                        .expect("action name mismatch");
                    // Depending on the state of the jump, we need to decide if we want to play the
                    // jump animation or the fall animation.
                    match jump_state {
                        TnuaBuiltinJumpState::NoJump => continue,
                        TnuaBuiltinJumpState::StartingJump { .. } => AnimationState::Jumping,
                        TnuaBuiltinJumpState::SlowDownTooFastSlopeJump { .. } => {
                            AnimationState::Jumping
                        }
                        TnuaBuiltinJumpState::MaintainingJump => AnimationState::Jumping,
                        TnuaBuiltinJumpState::StoppedMaintainingJump => AnimationState::Jumping,
                        TnuaBuiltinJumpState::FallSection => AnimationState::Falling,
                    }
                }
                Some(TnuaBuiltinCrouch::NAME) => {
                    // In case of crouch, we need the state of the basis to determine - based on
                    // the speed - if the charcter is just crouching or also crawling.
                    let Some((_, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>()
                    else {
                        continue;
                    };
                    let speed =
                        Some(basis_state.running_velocity.length()).filter(|speed| 0.01 < *speed);
                    let is_crouching = basis_state.standing_offset.y < -0.4;
                    match (speed, is_crouching) {
                        (None, false) => AnimationState::Standing,
                        (None, true) => AnimationState::Crouching,
                        (Some(speed), false) => AnimationState::Running(0.1 * speed),
                        (Some(speed), true) => AnimationState::Crawling(0.1 * speed),
                    }
                }
                // For the dash, we don't need the internal state of the dash action to determine
                // the action - so there is no need to downcast.
                Some(TnuaBuiltinDash::NAME) => AnimationState::Dashing,
                Some("TODO") => AnimationState::KnockedBack(Dir3::Y), // use fake dir for now
                Some(other) => panic!("Unknown action {other}"),
                None => {
                    // If there is no action going on, we'll base the animation on the state of the
                    // basis.
                    let Some((_, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>()
                    else {
                        continue;
                    };
                    if basis_state.standing_on_entity().is_none() {
                        AnimationState::Falling
                    } else {
                        let speed = basis_state.running_velocity.length();
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
                AnimationState::Running(speed) | AnimationState::Crawling(speed) => {
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
                    AnimationState::KnockedBack(_) => {
                        player
                            .start(handler.animations["KnockedBack"])
                            .set_speed(1.0);
                    }
                }
            }
        }

        // The knockback animation needs the character to be in a specific direciton. We want the
        // character to face that way immediately (a gradual turn would look weird here) and we
        // also want to lock the rotation so that the physics engine won't be able to slightly
        // change it between frames (which Tnua will try to do)
        if let Some(forward) = animating_state.get().and_then(|s| s.force_forward()) {
            forward_forcing.lock_rotation();
            forward_forcing.transform.look_to(forward, Dir3::Y);
        } else {
            forward_forcing.unlock_rotation();
        }
    }
}
