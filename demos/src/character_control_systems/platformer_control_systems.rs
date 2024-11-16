use bevy::prelude::*;
#[cfg(feature = "egui")]
use bevy_egui::{egui, EguiContexts};
use bevy_tnua::builtins::{
    TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash, TnuaBuiltinKnockback,
    TnuaBuiltinWallSlide,
};
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter, TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::math::{AdjustPrecision, AsF32, Float, Vector3};
use bevy_tnua::radar_lens::{TnuaBlipSpatialRelation, TnuaRadarLens};
use bevy_tnua::{prelude::*, TnuaObstacleRadar};
use bevy_tnua::{TnuaGhostSensor, TnuaProximitySensor};

use crate::ui::tuning::UiTunable;

use super::spatial_ext_facade::SpatialExtFacade;
use super::Dimensionality;

#[allow(clippy::type_complexity)]
#[allow(clippy::useless_conversion)]
pub fn apply_platformer_controls(
    #[cfg(feature = "egui")] mut egui_context: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &CharacterMotionConfigForPlatformerDemo,
        // This is the main component used for interacting with Tnua. It is used for both issuing
        // commands and querying the character's state.
        &mut TnuaController,
        // This is an helper for preventing the character from standing up while under an
        // obstacle, since this will make it slam into the obstacle, causing weird physics
        // behavior.
        // Most of the job is done by TnuaCrouchEnforcerPlugin - the control system only
        // needs to "let it know" about the crouch action.
        &mut TnuaCrouchEnforcer,
        // The proximity sensor usually works behind the scenes, but we need it here because
        // manipulating the proximity sensor using data from the ghost sensor is how one-way
        // platforms work in Tnua.
        &mut TnuaProximitySensor,
        // The ghost sensor detects ghost platforms - which are pass-through platforms marked with
        // the `TnuaGhostPlatform` component. Left alone it does not actually affect anything - a
        // user control system (like this very demo here) has to use the data from it and
        // manipulate the proximity sensor.
        &TnuaGhostSensor,
        // This is an helper for implementing one-way platforms.
        &mut TnuaSimpleFallThroughPlatformsHelper,
        // This is an helper for implementing air actions. It counts all the air actions using a
        // single counter, so it cannot be used to implement, for example, one double jump and one
        // air dash per jump - only a single "pool" of air action "energy" shared by all air
        // actions.
        &mut TnuaSimpleAirActionsCounter,
        // This is used in the shooter-like demo to control the forward direction of the
        // character.
        Option<&ForwardFromCamera>,
        // This is used to detect all the colliders in a small area around the character.
        &TnuaObstacleRadar,
    )>,
    // This is used to run spatial queries on the physics backend. Note that `SpatialExtFacade` is
    // defined in the demos crates, and actual games that use Tnua should instead use the
    // appropriate type from the physics backend integration crate they use - e.g.
    // `TnuaSpatialExtAvian2d` or `TnuaSpatialExtRapier3d`.
    spatial_ext: SpatialExtFacade,
) {
    #[cfg(feature = "egui")]
    if egui_context.ctx_mut().wants_keyboard_input() {
        for (_, mut controller, ..) in query.iter_mut() {
            // The basis remembers its last frame status, so if we cannot feed it proper input this
            // frame (for example - because the GUI takes the input focus) we need to neutralize
            // it.
            controller.neutralize_basis();
        }
        return;
    }

    for (
        config,
        mut controller,
        mut crouch_enforcer,
        mut sensor,
        ghost_sensor,
        mut fall_through_helper,
        mut air_actions_counter,
        forward_from_camera,
        obstacle_radar,
    ) in query.iter_mut()
    {
        // This part is just keyboard input processing. In a real game this would probably be done
        // with a third party plugin.
        let mut direction = Vector3::ZERO;

        if config.dimensionality == Dimensionality::Dim3 {
            if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
                direction -= Vector3::Z;
            }
            if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
                direction += Vector3::Z;
            }
        }
        if keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
            direction -= Vector3::X;
        }
        if keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
            direction += Vector3::X;
        }

        direction = direction.clamp_length_max(1.0);

        if let Some(forward_from_camera) = forward_from_camera {
            direction = Transform::default()
                .looking_to(forward_from_camera.forward.f32(), Vec3::Y)
                .transform_point(direction.f32())
                .adjust_precision();
        }

        let jump = match config.dimensionality {
            Dimensionality::Dim2 => {
                keyboard.any_pressed([KeyCode::Space, KeyCode::ArrowUp, KeyCode::KeyW])
            }
            Dimensionality::Dim3 => keyboard.any_pressed([KeyCode::Space]),
        };
        let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        let turn_in_place = forward_from_camera.is_none()
            && keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let crouch_pressed: bool;
        let crouch_just_pressed: bool;
        match config.dimensionality {
            Dimensionality::Dim2 => {
                let crouch_buttons = [
                    KeyCode::ControlLeft,
                    KeyCode::ControlRight,
                    KeyCode::ArrowDown,
                    KeyCode::KeyS,
                ];
                crouch_pressed = keyboard.any_pressed(crouch_buttons);
                crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);
            }
            Dimensionality::Dim3 => {
                let crouch_buttons = [KeyCode::ControlLeft, KeyCode::ControlRight];
                crouch_pressed = keyboard.any_pressed(crouch_buttons);
                crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);
            }
        }

        // This needs to be called once per frame. It lets the air actions counter know about the
        // air status of the character. Specifically:
        // * Is it grounded or is it midair?
        // * Did any air action just start?
        // * Did any air action just finished?
        // * Is any air action currently ongoing?
        air_actions_counter.update(controller.as_mut());

        // Here we will handle one-way platforms. It looks long and complex, but it's actual
        // several schemes with observable changes in behavior, and each implementation is rather
        // short and simple.
        let crouch;
        match config.falling_through {
            // With this scheme, the player cannot make their character fall through by pressing
            // the crouch button - the platforms are jump-through only.
            FallingThroughControlScheme::JumpThroughOnly => {
                crouch = crouch_pressed;
                // To achieve this, we simply take the first platform detected by the ghost sensor,
                // and treat it like a "real" platform.
                for ghost_platform in ghost_sensor.iter() {
                    // Because the ghots platforms don't interact with the character through the
                    // physics engine, and because the ray that detected them starts from the
                    // center of the character, we usually want to only look at platforms that
                    // are at least a certain distance lower than that - to limit the point from
                    // which the character climbs when they collide with the platform.
                    if config.one_way_platforms_min_proximity <= ghost_platform.proximity {
                        // By overriding the sensor's output, we make it pretend the ghost platform
                        // is a real one - which makes Tnua make the character stand on it even
                        // though the physics engine will not consider them colliding with each
                        // other.
                        sensor.output = Some(ghost_platform.clone());
                        break;
                    }
                }
            }
            // With this scheme, the player can drop down one-way platforms by pressing the crouch
            // button. Because it does not use `TnuaSimpleFallThroughPlatformsHelper`, it has
            // certain limitations:
            //
            // 1. If the player releases the crouch button before the character has passed a
            //    certain distance it'll climb back up on the platform.
            // 2. If a ghost platform is too close above another platform (either ghost or solid),
            //    such that when the character floats above the lower platform the higher platform
            //    is detected at above-minimal proximity, the character will climb up to the higher
            //    platform - even after explicitly dropping down from it to the lower one.
            //
            // Both limitations are greatly affected by the min proximity, but setting it tightly
            // to minimize them may cause the character to sometimes fall through a ghost platform
            // without explicitly being told to. To properly overcome these limitations - use
            // `TnuaSimpleFallThroughPlatformsHelper`.
            FallingThroughControlScheme::WithoutHelper => {
                // With this scheme we only care about the first ghost platform the ghost sensor
                // finds with a proximity higher than the defined minimum. We either treat it as a
                // real platform, or ignore it and any other platform the sensor has found.
                let relevant_platform = ghost_sensor.iter().find(|ghost_platform| {
                    config.one_way_platforms_min_proximity <= ghost_platform.proximity
                });
                if crouch_pressed {
                    // If there is a ghost platform, it means the player wants to fall through it -
                    // so we "cancel" the crouch, and we don't pass any ghots platform to the
                    // proximity sensor (because we want to character to fall through)
                    //
                    // If there is no ghost platform, it means the character is standing on a real
                    // platform - so we make it crouch. We don't pass any ghost platform to the
                    // proximity sensor here either - because there aren't any.
                    crouch = relevant_platform.is_none();
                } else {
                    crouch = false;
                    if let Some(ghost_platform) = relevant_platform {
                        // Ghost platforms can only be detected _before_ fully solid platforms, so
                        // if we detect one we can safely replace the proximity sensor's output
                        // with it.
                        //
                        // Do take care to only do this when there is a ghost platform though -
                        // otherwise it could replace an actual solid platform detection with a
                        // `None`.
                        sensor.output = Some(ghost_platform.clone());
                    }
                }
            }
            // This scheme uses `TnuaSimpleFallThroughPlatformsHelper` to properly handle fall
            // through:
            //
            // * Pressing the crouch button while standing on a ghost platform will make the
            //   character fall through it.
            // * Even if the button is released immediately, the character will not climb back up.
            //   It'll continue the fall.
            // * Even if the button is held and there is another ghost platform below, the
            //   character will only drop one "layer" of ghost platforms.
            // * If the player drops from a ghost platform to a platform too close to it - the
            //   character will not climb back up. The player can still climb back up by jumping,
            //   of course.
            FallingThroughControlScheme::SingleFall => {
                // The fall through helper is operated by creating an handler.
                let mut handler = fall_through_helper.with(
                    &mut sensor,
                    ghost_sensor,
                    config.one_way_platforms_min_proximity,
                );
                if crouch_pressed {
                    // Use `try_falling` to fall through the first ghost platform. It'll return
                    // `true` if there really was a ghost platform to fall through - in which case
                    // we want to cancel the crouch. If there was no ghost platform to fall
                    // through, it returns `false` - in which case we do want to crouch.
                    //
                    // The boolean argument to `try_falling` determines if the character should
                    // fall through "new" ghost platforms. When the player have just pressed the
                    // crouch button, we pass `true` so that the fall can begin. But in the
                    // following frames we pass `false` so that if there are more ghost platforms
                    // below the character will not fall through them.
                    crouch = !handler.try_falling(crouch_just_pressed);
                } else {
                    crouch = false;
                    // Use `dont_fall` to not fall. If there are platforms that the character
                    // already stared falling through, it'll continue the fall through and not
                    // climb back up (like it would with the `WithoutHelper` scheme). Otherwise, it
                    // will just copy the first ghost platform (above the min proximity) from the
                    // ghost sensor to the proximity sensor.
                    handler.dont_fall();
                }
            }
            // This scheme is similar to `SingleFall`, with the exception that as long as the
            // crouch button is pressed the character will keep falling through ghost platforms.
            FallingThroughControlScheme::KeepFalling => {
                let mut handler = fall_through_helper.with(
                    &mut sensor,
                    ghost_sensor,
                    config.one_way_platforms_min_proximity,
                );
                if crouch_pressed {
                    // This is done by passing `true` to `try_falling`, allowing it to keep falling
                    // through new platforms even if the button was not _just_ pressed.
                    crouch = !handler.try_falling(true);
                } else {
                    crouch = false;
                    handler.dont_fall();
                }
            }
        };

        let speed_factor =
            // `TnuaController::concrete_action` can be used to determine if an action is currently
            // running, and query its status. Here, we use it to check if the character is
            // currently crouching, so that we can limit its speed.
            if let Some((_, state)) = controller.concrete_action::<TnuaBuiltinCrouch>() {
                // If the crouch is finished (last stages of standing up) we don't need to slow the
                // character down.
                if matches!(state, TnuaBuiltinCrouchState::Rising) {
                    1.0
                } else {
                    0.2
                }
            } else {
                1.0
            };

        // The basis is Tnua's most fundamental control command, governing over the character's
        // regular movement. The basis (and, to some extent, the actions as well) contains both
        // configuration - which in this case we copy over from `config.walk` - and controls like
        // `desired_velocity` or `desired_forward` which we compute here based on the current
        // frame's input.
        controller.basis(TnuaBuiltinWalk {
            desired_velocity: if turn_in_place {
                Vector3::ZERO
            } else {
                direction * speed_factor * config.speed
            },
            desired_forward: if let Some(forward_from_camera) = forward_from_camera {
                // With shooters, we want the character model to follow the camera.
                Dir3::new(forward_from_camera.forward.f32()).ok()
            } else {
                // For platformers, we only want ot change direction when the character tries to
                // moves (or when the player explicitly wants to set the direction)
                Dir3::new(direction.f32()).ok()
            },
            ..config.walk.clone()
        });

        let radar_lens = TnuaRadarLens::new(obstacle_radar, &spatial_ext);

        let already_sliding_on = controller
            .concrete_action::<TnuaBuiltinWallSlide>()
            .and_then(|(action, _)| {
                action
                    .wall_entity
                    .filter(|wall_entity| obstacle_radar.has_blip(*wall_entity))
            });

        let mut walljump_candidate = None;

        for blip in radar_lens.iter_blips() {
            if !blip.is_interactable() {
                continue;
            }
            match blip.spatial_relation(0.5) {
                TnuaBlipSpatialRelation::Clipping => {}
                TnuaBlipSpatialRelation::Above => {}
                TnuaBlipSpatialRelation::Below => {}
                TnuaBlipSpatialRelation::Aeside(blip_direction) => {
                    let dot_threshold = if already_sliding_on == Some(blip.entity()) {
                        -0.1
                    } else {
                        0.0
                    };
                    if controller.is_airborne().unwrap() {
                        let dot_direction = direction.dot(blip_direction.adjust_precision());
                        if dot_direction <= -0.7 {
                            if let Some((best_entity, best_dot, best_direction)) =
                                walljump_candidate.as_mut()
                            {
                                if *best_dot < dot_direction {
                                    *best_entity = blip.entity();
                                    *best_dot = dot_direction;
                                    *best_direction = blip_direction;
                                }
                            } else {
                                walljump_candidate =
                                    Some((blip.entity(), dot_direction, blip_direction));
                            }
                        }
                        if dot_threshold < dot_direction {
                            let Ok(normal) = Dir3::new(blip.normal_from_closest_point().f32())
                            else {
                                continue;
                            };
                            controller.action(TnuaBuiltinWallSlide {
                                wall_entity: Some(blip.entity()),
                                contact_point_with_wall: blip.closest_point(),
                                normal,
                                force_forward: Some(blip_direction),
                                max_fall_speed: 2.0,
                                maintain_distance: Some(0.7),
                                max_sideways_speed: 1.0,
                                max_sideways_acceleration: 60.0,
                            });
                        }
                    }
                }
            }
        }
        let walljump_candidate =
            walljump_candidate.map(|(entity, _, blip_direction)| (entity, -blip_direction));

        if crouch {
            // Crouching is an action. We either feed it or we don't - other than that there is
            // nothing to set from the current frame's input. We do pass it through the crouch
            // enforcer though, which makes sure the character does not stand up if below an
            // obstacle.
            controller.action(crouch_enforcer.enforcing(config.crouch.clone()));
        }

        if jump {
            if matches!(
                controller.action_flow_status().ongoing(),
                Some(TnuaBuiltinJump::NAME | "walljump")
            ) {
                controller.prolong_action();
            } else if let Some((_, walljump_direction)) = walljump_candidate {
                controller.named_action(
                    "walljump",
                    TnuaBuiltinJump {
                        vertical_displacement: Some(2.0 * walljump_direction.adjust_precision()),
                        allow_in_air: true,
                        takeoff_extra_gravity: 3.0 * config.jump.takeoff_extra_gravity,
                        takeoff_above_velocity: 0.0,
                        force_forward: Some(-walljump_direction),
                        ..config.jump.clone()
                    },
                );
            } else {
                controller.action(TnuaBuiltinJump {
                    // Jumping, like crouching, is an action that we either feed or don't. However,
                    // because it can be used in midair, we want to set its `allow_in_air`. The air
                    // counter helps us with that.
                    //
                    // The air actions counter is used to decide if the action is allowed midair by
                    // determining how many actions were performed since the last time the character
                    // was considered "grounded" - including the first jump (if it was done from the
                    // ground) or the initiation of a free fall.
                    //
                    // `air_count_for` needs the name of the action to be performed (in this case
                    // `TnuaBuiltinJump::NAME`) because if the player is still holding the jump button,
                    // we want it to be considered as the same air action number. So, if the player
                    // performs an air jump, before the air jump `air_count_for` will return 1 for any
                    // action, but after it it'll return 1 only for `TnuaBuiltinJump::NAME`
                    // (maintaining the jump) and 2 for any other action. Of course, if the player
                    // releases the button and presses it again it'll return 2.
                    allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinJump::NAME)
                        <= config.actions_in_air,
                    ..config.jump.clone()
                });
            }
        }

        if dash {
            controller.action(TnuaBuiltinDash {
                // Dashing is also an action, but because it has directions we need to provide said
                // directions. `displacement` is a vector that determines where the jump will bring
                // us. Note that even after reaching the displacement, the character may still have
                // some leftover velocity (configurable with the other parameters of the action)
                //
                // The displacement is "frozen" when the action starts - user code does not have to
                // worry about storing the original direction.
                displacement: direction.normalize() * config.dash_distance,
                // When set, the `desired_forward` of the dash action "overrides" the
                // `desired_forward` of the walk basis. Like the displacement, it gets "frozen" -
                // allowing to easily maintain a forward direction during the dash.
                desired_forward: if forward_from_camera.is_none() {
                    Dir3::new(direction.f32()).ok()
                } else {
                    // For shooters, we want to allow rotating mid-dash if the player moves the
                    // mouse.
                    None
                },
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinDash::NAME)
                    <= config.actions_in_air,
                ..config.dash.clone()
            });
        }
    }
}

#[derive(Component)]
pub struct CharacterMotionConfigForPlatformerDemo {
    pub dimensionality: Dimensionality,
    pub speed: Float,
    pub walk: TnuaBuiltinWalk,
    pub actions_in_air: usize,
    pub jump: TnuaBuiltinJump,
    pub crouch: TnuaBuiltinCrouch,
    pub dash_distance: Float,
    pub dash: TnuaBuiltinDash,
    pub one_way_platforms_min_proximity: Float,
    pub falling_through: FallingThroughControlScheme,
    pub knockback: TnuaBuiltinKnockback,
}

impl UiTunable for CharacterMotionConfigForPlatformerDemo {
    #[cfg(feature = "egui")]
    fn tune(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Walking:", |ui| {
            ui.add(egui::Slider::new(&mut self.speed, 0.0..=60.0).text("Speed"));
            self.walk.tune(ui);
        });
        ui.add(egui::Slider::new(&mut self.actions_in_air, 0..=8).text("Max Actions in Air"));
        ui.collapsing("Jumping:", |ui| {
            self.jump.tune(ui);
        });
        ui.collapsing("Dashing:", |ui| {
            ui.add(egui::Slider::new(&mut self.dash_distance, 0.0..=40.0).text("Dash Distance"));
            self.dash.tune(ui);
        });
        ui.collapsing("Crouching:", |ui| {
            self.crouch.tune(ui);
        });
        ui.collapsing("One-way Platforms", |ui| {
            ui.add(
                egui::Slider::new(&mut self.one_way_platforms_min_proximity, 0.0..=2.0)
                    .text("Min Proximity"),
            );
            self.falling_through.tune(ui);
        });
        ui.collapsing("Knockback:", |ui| {
            self.knockback.tune(ui);
        });
    }
}

#[derive(Component, Debug, PartialEq, Default)]
pub enum FallingThroughControlScheme {
    JumpThroughOnly,
    WithoutHelper,
    #[default]
    SingleFall,
    KeepFalling,
}

impl UiTunable for FallingThroughControlScheme {
    #[cfg(feature = "egui")]
    fn tune(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_label("Falling Through Control Scheme")
            .selected_text(format!("{:?}", self))
            .show_ui(ui, |ui| {
                for variant in [
                    FallingThroughControlScheme::JumpThroughOnly,
                    FallingThroughControlScheme::WithoutHelper,
                    FallingThroughControlScheme::SingleFall,
                    FallingThroughControlScheme::KeepFalling,
                ] {
                    if ui
                        .selectable_label(*self == variant, format!("{:?}", variant))
                        .clicked()
                    {
                        *self = variant;
                    }
                }
            });
    }
}

#[derive(Component)]
pub struct ForwardFromCamera {
    pub forward: Vector3,
    pub pitch_angle: Float,
}

impl Default for ForwardFromCamera {
    fn default() -> Self {
        Self {
            forward: Vector3::NEG_Z,
            pitch_angle: 0.0,
        }
    }
}
