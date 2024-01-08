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
        &mut TnuaController,
        &mut TnuaCrouchEnforcer,
        &mut TnuaProximitySensor,
        &TnuaGhostSensor,
        &mut TnuaSimpleFallThroughPlatformsHelper,
        &FallingThroughControlScheme,
        &mut TnuaSimpleAirActionsCounter,
    )>,
) {
    if egui_context.ctx_mut().wants_keyboard_input() {
        for (_, mut controller, ..) in query.iter_mut() {
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
        air_actions_counter.update(controller.as_mut());

        let crouch = falling_through_control_scheme.perform_and_check_if_still_crouching(
            crouch,
            crouch_just_pressed,
            fall_through_helper.as_mut(),
            sensor.as_mut(),
            ghost_sensor,
            1.0,
        );

        let speed_factor =
            if let Some((_, state)) = controller.concrete_action::<TnuaBuiltinCrouch>() {
                if matches!(state, TnuaBuiltinCrouchState::Rising) {
                    1.0
                } else {
                    0.2
                }
            } else {
                1.0
            };

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
            controller.action(crouch_enforcer.enforcing(config.crouch.clone()));
        }

        if jump {
            controller.action(TnuaBuiltinJump {
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinJump::NAME)
                    <= config.actions_in_air,
                ..config.jump.clone()
            });
        }

        if dash {
            controller.action(TnuaBuiltinDash {
                displacement: direction.normalize() * config.dash_distance,
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
