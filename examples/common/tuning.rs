use std::ops::RangeInclusive;

use bevy::prelude::*;

use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinDash};
use bevy_tnua::prelude::*;

use bevy_egui::egui;

pub trait UiTunable {
    fn tune(&mut self, ui: &mut egui::Ui);
}

fn slider_or_infinity(
    ui: &mut egui::Ui,
    caption: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
) {
    #[derive(Clone)]
    struct CachedValue(f32);

    ui.horizontal(|ui| {
        let mut infinite = !value.is_finite();
        let resp = ui.toggle_value(&mut infinite, "\u{221e}");
        if resp.clicked() {
            if infinite {
                ui.memory_mut(|memory| memory.data.insert_temp(resp.id, CachedValue(*value)));
                *value = f32::INFINITY
            } else {
                if let Some(CachedValue(saved_value)) =
                    ui.memory_mut(|memory| memory.data.get_temp(resp.id))
                {
                    *value = saved_value;
                } else {
                    *value = *range.end();
                }
            }
        }
        if infinite {
            let mut copied_saved_value = ui.memory_mut(|memory| {
                let CachedValue(saved_value) = memory
                    .data
                    .get_temp_mut_or(resp.id, CachedValue(*range.end()));
                *saved_value
            });
            ui.add_enabled(
                false,
                egui::Slider::new(&mut copied_saved_value, range).text(caption),
            );
        } else {
            ui.add(egui::Slider::new(value, range).text(caption));
        }
    });
}

fn slider_or_none(
    ui: &mut egui::Ui,
    caption: &str,
    value: &mut Option<f32>,
    range: RangeInclusive<f32>,
) {
    #[derive(Clone)]
    struct CachedValue(f32);

    ui.horizontal(|ui| {
        let mut is_none = value.is_none();
        let resp = ui.toggle_value(&mut is_none, "\u{d8}");
        if resp.clicked() {
            if is_none {
                ui.memory_mut(|memory| memory.data.insert_temp(resp.id, CachedValue(value.expect("checkbox was clicked, and is_none is now true, so previously it was false, which means value should not be None"))));
                *value = None;
            } else {
                if let Some(CachedValue(saved_value)) =
                    ui.memory_mut(|memory| memory.data.get_temp(resp.id))
                {
                    *value = Some(saved_value);
                } else {
                    *value = Some(*range.start());
                }
            }
        }
        if let Some(value) = value.as_mut() {
            ui.add(egui::Slider::new(value, range).text(caption));
        } else {
            let mut copied_saved_value = ui.memory_mut(|memory| {
                let CachedValue(saved_value) = memory
                    .data
                    .get_temp_mut_or(resp.id, CachedValue(*range.start()));
                *saved_value
            });
            ui.add_enabled(
                false,
                egui::Slider::new(&mut copied_saved_value, range).text(caption),
            );
        }
    });
}

#[derive(Component)]
pub struct CharacterMotionConfigForPlatformerExample {
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

impl UiTunable for TnuaBuiltinWalk {
    fn tune(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.float_height, 0.0..=10.0).text("Float At"));
        ui.add(egui::Slider::new(&mut self.cling_distance, 0.0..=10.0).text("Cling Distance"));
        ui.add(egui::Slider::new(&mut self.spring_strengh, 0.0..=4000.0).text("Spring Strengh"));
        ui.add(egui::Slider::new(&mut self.spring_dampening, 0.0..=1.9).text("Spring Dampening"));
        slider_or_infinity(ui, "Acceleration", &mut self.acceleration, 0.0..=200.0);
        slider_or_infinity(
            ui,
            "Air Acceleration",
            &mut self.air_acceleration,
            0.0..=200.0,
        );
        ui.add(egui::Slider::new(&mut self.coyote_time, 0.0..=1.0).text("Coyote Time"));

        ui.add(
            egui::Slider::new(&mut self.free_fall_extra_gravity, 0.0..=100.0)
                .text("Free Fall Extra Gravity"),
        );

        slider_or_infinity(
            ui,
            "Staying Upward Max Angular Velocity",
            &mut self.tilt_offset_angvel,
            0.0..=20.0,
        );
        slider_or_infinity(
            ui,
            "Staying Upward Max Angular Acceleration",
            &mut self.tilt_offset_angacl,
            0.0..=2000.0,
        );

        slider_or_infinity(
            ui,
            "Turning Angular Velocity",
            &mut self.turning_angvel,
            0.0..=70.0,
        );
    }
}

impl UiTunable for TnuaBuiltinJump {
    fn tune(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.height, 0.0..=10.0).text("Jump Height"));
        ui.add(
            egui::Slider::new(&mut self.input_buffer_time, 0.0..=1.0)
                .text("Jump Input Buffer Time"),
        );
        slider_or_none(
            ui,
            "Held Jump Cooldown",
            &mut self.reschedule_cooldown,
            0.0..=2.0,
        );
        ui.add(
            egui::Slider::new(&mut self.upslope_extra_gravity, 0.0..=100.0)
                .text("Upslope Jump Extra Gravity"),
        );
        ui.add(
            egui::Slider::new(&mut self.takeoff_extra_gravity, 0.0..=100.0)
                .text("Jump Takeoff Extra Gravity"),
        );
        slider_or_infinity(
            ui,
            "Jump Takeoff Above Velocity",
            &mut self.takeoff_above_velocity,
            0.0..=20.0,
        );
        ui.add(
            egui::Slider::new(&mut self.fall_extra_gravity, 0.0..=50.0)
                .text("Jump Fall Extra Gravity"),
        );
        ui.add(
            egui::Slider::new(&mut self.shorten_extra_gravity, 0.0..=100.0)
                .text("Jump Shorten Extra Gravity"),
        );

        ui.add(
            egui::Slider::new(&mut self.peak_prevention_at_upward_velocity, 0.0..=20.0)
                .text("Jump Peak Prevention At Upward Velocity"),
        );

        ui.add(
            egui::Slider::new(&mut self.peak_prevention_extra_gravity, 0.0..=100.0)
                .text("Jump Peak Prevention Extra Gravity"),
        );
    }
}
impl UiTunable for TnuaBuiltinCrouch {
    fn tune(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::Slider::new(&mut self.height_change_impulse_for_duration, 0.001..=0.2)
                .text("Height Change Impulse for Duration"),
        );

        slider_or_infinity(
            ui,
            "Height Change Impulse",
            &mut self.height_change_impulse_limit,
            0.0..=40.0,
        );
    }
}

impl UiTunable for TnuaBuiltinDash {
    fn tune(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.speed, 0.0..=200.0).text("Dash Speed"));
        slider_or_infinity(
            ui,
            "Brake to Speed After Dash",
            &mut self.brake_to_speed,
            0.0..=80.0,
        );
        slider_or_infinity(ui, "Dash Acceleration", &mut self.acceleration, 0.0..=800.0);
        slider_or_infinity(
            ui,
            "Dash Brake Acceleration",
            &mut self.brake_acceleration,
            0.0..=800.0,
        );
        ui.add(
            egui::Slider::new(&mut self.input_buffer_time, 0.0..=1.0)
                .text("Dash Input Buffer Time"),
        );
    }
}
