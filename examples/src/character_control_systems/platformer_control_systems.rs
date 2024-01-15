use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash};
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter, TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaGhostSensor, TnuaProximitySensor};

use crate::ui::tuning::UiTunable;
use crate::FallingThroughControlScheme;

use super::Dimensionality;

#[allow(clippy::type_complexity)]
pub fn apply_platformer_controls(
    mut egui_context: EguiContexts,
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<(
        &CharacterMotionConfigForPlatformerExample,
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
        // user control system (like this example you are reading right now) has to use the data
        // from it and manipulate the proximity sensor.
        &TnuaGhostSensor,
        // This is an helper for implementing one-way platforms.
        &mut TnuaSimpleFallThroughPlatformsHelper,
        &FallingThroughControlScheme,
        // This is an helper for implementing air actions. It counts all the air actions using a
        // single counter, so it cannot be used to implement, for example, one double jump and one
        // air dash per jump - only a single "pool" of air action "energy" shared by all air
        // actions.
        &mut TnuaSimpleAirActionsCounter,
    )>,
) {
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
        falling_through_control_scheme,
        mut air_actions_counter,
    ) in query.iter_mut()
    {
        // This part is just keyboard input processing. In a real game this would probably be done
        // with a third party plugin.
        let mut direction = Vec3::ZERO;

        if config.dimensionality == Dimensionality::Dim3 {
            if keyboard.pressed(KeyCode::Up) {
                direction -= Vec3::Z;
            }
            if keyboard.pressed(KeyCode::Down) {
                direction += Vec3::Z;
            }
        }
        if keyboard.pressed(KeyCode::Left) {
            direction -= Vec3::X;
        }
        if keyboard.pressed(KeyCode::Right) {
            direction += Vec3::X;
        }

        direction = direction.clamp_length_max(1.0);

        let jump = match config.dimensionality {
            Dimensionality::Dim2 => keyboard.any_pressed([KeyCode::Space, KeyCode::Up]),
            Dimensionality::Dim3 => keyboard.any_pressed([KeyCode::Space]),
        };
        let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        let turn_in_place = keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let crouch: bool;
        let crouch_just_pressed: bool;
        match config.dimensionality {
            Dimensionality::Dim2 => {
                let crouch_buttons = [KeyCode::Down, KeyCode::ControlLeft, KeyCode::ControlRight];
                crouch = keyboard.any_pressed(crouch_buttons);
                crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);
            }
            Dimensionality::Dim3 => {
                let crouch_buttons = [KeyCode::ControlLeft, KeyCode::ControlRight];
                crouch = keyboard.any_pressed(crouch_buttons);
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

        // TODO: move the various implementations to here, and document them.
        let crouch = falling_through_control_scheme.perform_and_check_if_still_crouching(
            crouch,
            crouch_just_pressed,
            fall_through_helper.as_mut(),
            sensor.as_mut(),
            ghost_sensor,
            1.0,
        );

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
                Vec3::ZERO
            } else {
                direction * speed_factor * config.speed
            },
            desired_forward: direction.normalize_or_zero(),
            ..config.walk.clone()
        });

        if crouch {
            // Crouching is an action. We either feed it or we don't - other than that there is
            // nothing to set from the current frame's input. We do pass it through the crouch
            // enforcer though, which makes sure the character does not stand up if below an
            // obstacle.
            controller.action(crouch_enforcer.enforcing(config.crouch.clone()));
        }

        if jump {
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
                desired_forward: direction.normalize(),
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinDash::NAME)
                    <= config.actions_in_air,
                ..config.dash.clone()
            });
        }
    }
}

#[derive(Component)]
pub struct CharacterMotionConfigForPlatformerExample {
    pub dimensionality: Dimensionality,
    pub speed: f32,
    pub walk: TnuaBuiltinWalk,
    pub actions_in_air: usize,
    pub jump: TnuaBuiltinJump,
    pub crouch: TnuaBuiltinCrouch,
    pub dash_distance: f32,
    pub dash: TnuaBuiltinDash,
}

impl UiTunable for CharacterMotionConfigForPlatformerExample {
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
    }
}
