use std::cmp::Ordering;

use bevy::{
    app::{FixedMain, RunFixedMainLoop},
    prelude::*,
};
#[cfg(feature = "egui")]
use bevy_egui::{egui, EguiContexts};
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter, TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::math::{AdjustPrecision, AsF32, Float, Vector3};
use bevy_tnua::radar_lens::{TnuaBlipSpatialRelation, TnuaRadarLens};
use bevy_tnua::{
    builtins::{
        TnuaBuiltinClimb, TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash,
        TnuaBuiltinKnockback, TnuaBuiltinWallSlide,
    },
    control_helpers::TnuaBlipReuseAvoidance,
};
use bevy_tnua::{prelude::*, TnuaObstacleRadar};
use bevy_tnua::{TnuaGhostSensor, TnuaProximitySensor};

use crate::ui::tuning::UiTunable;

use super::querying_helpers::ObstacleQueryHelper;
use super::spatial_ext_facade::SpatialExtFacade;
use super::Dimensionality;

#[allow(clippy::type_complexity)]
#[allow(clippy::useless_conversion)]
pub fn apply_platformer_controls(
    #[cfg(feature = "egui")] mut egui_context: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut just_pressed: ResMut<JustPressedCache>,
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
        // This is used to avoid re-initiating actions on the same obstacles until we return to
        // them.
        &mut TnuaBlipReuseAvoidance,
    )>,
    // This is used to run spatial queries on the physics backend. Note that `SpatialExtFacade` is
    // defined in the demos crates, and actual games that use Tnua should instead use the
    // appropriate type from the physics backend integration crate they use - e.g.
    // `TnuaSpatialExtAvian2d` or `TnuaSpatialExtRapier3d`.
    spatial_ext: SpatialExtFacade,
    // This is used to determine the qualities of the obstacles (e.g. whether or not they are
    // climbable)
    obstacle_query: Query<ObstacleQueryHelper>,
) {
    #[cfg(feature = "egui")]
    if egui_context.ctx_mut().unwrap().wants_keyboard_input() {
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
        mut blip_reuse_avoidance,
    ) in query.iter_mut()
    {
        // This part is just keyboard input processing. In a real game this would probably be done
        // with a third party plugin.
        let mut direction = Vector3::ZERO;

        let is_climbing = controller.action_name() == Some(TnuaBuiltinClimb::NAME);

        if config.dimensionality == Dimensionality::Dim3 || is_climbing {
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

        let screen_space_direction = direction.clamp_length_max(1.0);

        let transform_for_controls = calculate_transform_for_controls(
            forward_from_camera
                .and_then(|ffc| Dir3::new(ffc.forward.f32()).ok())
                .unwrap_or(Dir3::NEG_Z),
            Dir3::Y, // TOOD: does this change in shooter?
            controller.up_direction().unwrap_or(Dir3::Y),
        );

        let direction = transform_for_controls
            .transform_point(screen_space_direction.f32())
            .adjust_precision();

        let jump = match (config.dimensionality, is_climbing) {
            (Dimensionality::Dim2, true) => keyboard.any_pressed([KeyCode::Space]),
            (Dimensionality::Dim2, false) => {
                keyboard.any_pressed([KeyCode::Space, KeyCode::ArrowUp, KeyCode::KeyW])
            }
            (Dimensionality::Dim3, _) => keyboard.any_pressed([KeyCode::Space]),
        };
        let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        let turn_in_place = forward_from_camera.is_none()
            && keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let crouch_buttons = match (config.dimensionality, is_climbing) {
            (Dimensionality::Dim2, true) => CROUCH_BUTTONS_3D.iter().copied(),
            (Dimensionality::Dim2, false) => CROUCH_BUTTONS_2D.iter().copied(),
            (Dimensionality::Dim3, _) => CROUCH_BUTTONS_3D.iter().copied(),
        };
        let crouch_pressed = keyboard.any_pressed(crouch_buttons);
        let crouch_just_pressed = just_pressed.crouch;
        just_pressed.was_read = true;

        // This needs to be called once per frame. It lets the air actions counter know about the
        // air status of the character. Specifically:
        // * Is it grounded or is it midair?
        // * Did any air action just start?
        // * Did any air action just finished?
        // * Is any air action currently ongoing?
        air_actions_counter.update(controller.as_ref());

        // This also needs to be called once per frame. It checks which obstacles needs to be
        // blocked - e.g. because we've just finished an action on them and we don't want to
        // reinitiate that action.
        blip_reuse_avoidance.update(controller.as_ref(), obstacle_radar);

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
                    .filter(|entity| obstacle_radar.has_blip(*entity))
            });

        let already_climbing_on =
            controller
                .concrete_action::<TnuaBuiltinClimb>()
                .and_then(|(action, _)| {
                    let entity = action
                        .climbable_entity
                        .filter(|entity| obstacle_radar.has_blip(*entity))?;
                    Some((entity, action.clone()))
                });

        let mut walljump_candidate = None;

        'blips_loop: for blip in radar_lens.iter_blips() {
            if !blip_reuse_avoidance.should_avoid(blip.entity())
                && obstacle_query
                    .get(blip.entity())
                    .expect("ObstacleQueryHelper has nothing that could fail when missing")
                    .climbable
            {
                if let Some((climbable_entity, action)) = already_climbing_on.as_ref() {
                    if *climbable_entity != blip.entity() {
                        continue 'blips_loop;
                    }
                    let dot_initiation = direction.dot(action.initiation_direction);
                    let initiation_direction = if 0.5 < dot_initiation {
                        action.initiation_direction
                    } else {
                        Vector3::ZERO
                    };
                    if initiation_direction == Vector3::ZERO {
                        let right_left = screen_space_direction.dot(Vector3::X);
                        if 0.5 <= right_left.abs() {
                            continue 'blips_loop;
                        }
                    }

                    let mut action = TnuaBuiltinClimb {
                        climbable_entity: Some(blip.entity()),
                        anchor: blip.closest_point().get(),
                        desired_climb_velocity: config.climb_speed
                            * screen_space_direction.dot(Vector3::NEG_Z)
                            * Vector3::Y,
                        initiation_direction,
                        desired_vec_to_anchor: action.desired_vec_to_anchor,
                        desired_forward: action.desired_forward,
                        ..config.climb.clone()
                    };

                    const LOOK_ABOVE_OR_BELOW: Float = 5.0;
                    match action
                        .desired_climb_velocity
                        .dot(Vector3::Y)
                        .partial_cmp(&0.0)
                        .unwrap()
                    {
                        Ordering::Less => {
                            if controller.is_airborne().unwrap() {
                                let extent = blip
                                    .probe_extent_from_closest_point(-Dir3::Y, LOOK_ABOVE_OR_BELOW);
                                if extent < 0.9 * LOOK_ABOVE_OR_BELOW {
                                    action.hard_stop_down =
                                        Some(blip.closest_point().get() - extent * Vector3::Y);
                                }
                            } else if initiation_direction == Vector3::ZERO {
                                continue 'blips_loop;
                            } else {
                                action.desired_climb_velocity = Vector3::ZERO;
                            }
                        }
                        Ordering::Equal => {}
                        // Climbing up
                        Ordering::Greater => {
                            let extent =
                                blip.probe_extent_from_closest_point(Dir3::Y, LOOK_ABOVE_OR_BELOW);
                            if extent < 0.9 * LOOK_ABOVE_OR_BELOW {
                                action.hard_stop_up =
                                    Some(blip.closest_point().get() + extent * Vector3::Y);
                            }
                        }
                    }

                    controller.action(action);
                } else if let TnuaBlipSpatialRelation::Aeside(blip_direction) =
                    blip.spatial_relation(0.5)
                {
                    if 0.5 < direction.dot(blip_direction.adjust_precision()) {
                        let direction_to_anchor = match config.dimensionality {
                            Dimensionality::Dim2 => Vector3::ZERO,
                            Dimensionality::Dim3 => -blip
                                .normal_from_closest_point()
                                .reject_from_normalized(Vector3::Y),
                        };
                        controller.action(TnuaBuiltinClimb {
                            climbable_entity: Some(blip.entity()),
                            anchor: blip.closest_point().get(),
                            desired_vec_to_anchor: 0.5 * direction_to_anchor,
                            desired_forward: Dir3::new(direction_to_anchor.f32()).ok(),
                            initiation_direction: direction.normalize_or_zero(),
                            ..config.climb.clone()
                        });
                    }
                }
            }
            if !blip.is_interactable() {
                continue;
            }
            match blip.spatial_relation(0.5) {
                TnuaBlipSpatialRelation::Invalid => {}
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
                        if dot_threshold < dot_direction
                            && 0.8 < blip.flat_wall_score(Dir3::Y, &[-1.0, 1.0])
                        {
                            let Ok(normal) = Dir3::new(blip.normal_from_closest_point().f32())
                            else {
                                continue;
                            };
                            controller.action(TnuaBuiltinWallSlide {
                                wall_entity: Some(blip.entity()),
                                contact_point_with_wall: blip.closest_point().get(),
                                normal,
                                force_forward: Some(blip_direction),
                                maintain_distance: Some(0.7),
                                ..config.wall_slide.clone()
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
            let action_flow_status = controller.action_flow_status().clone();
            if matches!(
                action_flow_status.ongoing(),
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
                let current_action_name = controller.action_name();
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
                        <= config.actions_in_air
                        // We also want to be able to jump from a climb.
                        || current_action_name == Some(TnuaBuiltinClimb::NAME),
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
    pub wall_slide: TnuaBuiltinWallSlide,
    pub climb_speed: Float,
    pub climb: TnuaBuiltinClimb,
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
        ui.collapsing("Wall Slide:", |ui| {
            self.wall_slide.tune(ui);
        });
        ui.collapsing("Climb", |ui| {
            ui.add(egui::Slider::new(&mut self.climb_speed, 0.0..=30.0).text("Climb Speed"));
            self.climb.tune(ui);
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

pub fn calculate_transform_for_controls(
    camera_forward: Dir3,
    camera_up: Dir3,
    controller_up: Dir3,
) -> Transform {
    let dot = camera_forward.dot(controller_up.into());
    let quat = if dot <= 0.0 {
        Quat::from_rotation_arc(*camera_up, *controller_up)
    } else {
        Quat::from_rotation_arc(-*camera_up, *controller_up)
    };
    Transform::default().with_rotation(quat)
}

/// Since the fixed timestep schedule does not cache just pressed states that happened
/// in a frame with no fixed updates, we need to cache them ourselves in order to not miss them.
/// Note that if you use a smarter input manager like LWIM, this is handled for you.
/// If the demo is running with a variable timestep, this will just report the current frame's
/// state as expected.
pub struct JustPressedCachePlugin;

impl Plugin for JustPressedCachePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JustPressedCache>();
        app.add_systems(
            RunFixedMainLoop,
            (
                collect_just_pressed_cache.before(FixedMain::run_fixed_main),
                clear_just_pressed_cache.after(FixedMain::run_fixed_main),
            ),
        );
    }
}

fn collect_just_pressed_cache(
    query: Query<&CharacterMotionConfigForPlatformerDemo>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut just_pressed: ResMut<JustPressedCache>,
) {
    for config in &query {
        let crouch_buttons = match config.dimensionality {
            Dimensionality::Dim2 => CROUCH_BUTTONS_2D.iter().copied(),
            Dimensionality::Dim3 => CROUCH_BUTTONS_3D.iter().copied(),
        };
        just_pressed.crouch = keyboard.any_just_pressed(crouch_buttons);
    }
}

fn clear_just_pressed_cache(mut just_pressed: ResMut<JustPressedCache>) {
    if just_pressed.was_read {
        *just_pressed = default()
    }
}

#[derive(Resource, Default)]
pub struct JustPressedCache {
    crouch: bool,
    was_read: bool,
}

const CROUCH_BUTTONS_2D: &[KeyCode] = &[
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::ArrowDown,
    KeyCode::KeyS,
];

const CROUCH_BUTTONS_3D: &[KeyCode] = &[KeyCode::ControlLeft, KeyCode::ControlRight];
