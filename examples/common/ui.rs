use std::ops::RangeInclusive;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_tnua::{TnuaFreeFallBehavior, TnuaPlatformerConfig};

use super::ui_plotting::PlotSource;

pub struct ExampleUi;

impl Plugin for ExampleUi {
    fn build(&self, app: &mut App) {
        app.add_plugin(EguiPlugin);
        app.add_system(ui_system);
        app.add_system(super::ui_plotting::plot_source_rolling_update);
    }
}

#[derive(Component)]
pub struct TrackedEntity(pub String);

#[derive(Component)]
pub struct CommandAlteringSelectors(Vec<CommandAlteringSelector>);

impl Default for CommandAlteringSelectors {
    fn default() -> Self {
        Self(Default::default())
    }
}

enum CommandAlteringSelector {
    Combo {
        chosen: usize,
        caption: String,
        options: Vec<(String, fn(EntityCommands))>,
    },
    Checkbox {
        checked: bool,
        caption: String,
        applier: fn(EntityCommands, bool),
    },
}

impl CommandAlteringSelectors {
    pub fn with_combo(mut self, caption: &str, options: &[(&str, fn(EntityCommands))]) -> Self {
        self.0.push(CommandAlteringSelector::Combo {
            chosen: 0,
            caption: caption.to_owned(),
            options: options
                .into_iter()
                .map(|(name, applier)| (name.to_string(), *applier))
                .collect(),
        });
        self
    }

    pub fn with_checkbox(mut self, caption: &str, applier: fn(EntityCommands, bool)) -> Self {
        self.0.push(CommandAlteringSelector::Checkbox {
            checked: false,
            caption: caption.to_owned(),
            applier,
        });
        self
    }
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
                ui.memory().data.insert_temp(resp.id, CachedValue(*value));
                *value = f32::INFINITY
            } else {
                if let Some(CachedValue(saved_value)) = ui.memory().data.get_temp(resp.id) {
                    *value = saved_value;
                } else {
                    *value = *range.end();
                }
            }
        }
        if infinite {
            let CachedValue(mut saved_value) = ui
                .memory()
                .data
                .get_temp_mut_or(resp.id, CachedValue(*range.end()));
            ui.add_enabled(
                false,
                egui::Slider::new(&mut saved_value, range).text(caption),
            );
        } else {
            ui.add(egui::Slider::new(value, range).text(caption));
        }
    });
}

fn ui_system(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<(
        Entity,
        &TrackedEntity,
        &PlotSource,
        &mut TnuaPlatformerConfig,
        Option<&mut CommandAlteringSelectors>,
    )>,
    mut commands: Commands,
) {
    egui::Window::new("Tnua").show(egui_context.ctx_mut(), |ui| {
        ui.label("Controls: Move with the arrow keys. Jump with Spacebar. Turn in place with Alt");
        for (
            entity,
            TrackedEntity(name),
            plot_source,
            mut platformer_config,
            command_altering_selectors,
        ) in query.iter_mut()
        {
            egui::CollapsingHeader::new(name)
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Slider::new(&mut platformer_config.full_speed, 0.0..=60.0)
                                    .text("Speed"),
                            );
                            ui.add(
                                egui::Slider::new(&mut platformer_config.full_jump_height, 0.0..=10.0)
                                    .text("Jump Height"),
                            );
                            platformer_config.full_jump_height = platformer_config.full_jump_height.max(0.1);

                            if let Some(mut command_altering_selectors) = command_altering_selectors
                            {
                                for selector in command_altering_selectors.0.iter_mut() {
                                    match selector {
                                        CommandAlteringSelector::Combo { chosen, caption, options } => {
                                            let mut selected_idx: usize = *chosen;
                                            egui::ComboBox::from_label(caption.as_str())
                                                .selected_text(&options[*chosen].0)
                                                .show_ui(ui, |ui| {
                                                    for (idx, (name, _)) in options.iter().enumerate() {
                                                        ui.selectable_value(&mut selected_idx, idx, name);
                                                    }
                                                });
                                            if selected_idx != *chosen {
                                                options[selected_idx].1(commands.entity(entity));
                                                *chosen = selected_idx;
                                            }
                                        }
                                        CommandAlteringSelector::Checkbox { checked, caption, applier } => {
                                            if ui.checkbox(checked, caption.as_str()).clicked() {
                                                applier(commands.entity(entity), *checked);
                                            }
                                        }
                                    }
                                }
                            }

                            ui.add(
                                egui::Slider::new(&mut platformer_config.float_height, 0.0..=10.0)
                                    .text("Float At"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.cling_distance,
                                    0.0..=10.0,
                                )
                                .text("Cling Distance"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.spring_strengh,
                                    0.0..=4000.0,
                                )
                                .text("Spring Strengh"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.spring_dampening,
                                    0.0..=100.0,
                                )
                                .text("Spring Dampening"),
                            );
                            slider_or_infinity(ui, "Acceleration", &mut platformer_config.acceleration, 0.0..=200.0);
                            slider_or_infinity(ui, "Air Acceleration", &mut platformer_config.air_acceleration, 0.0..=200.0);
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.coyote_time,
                                    0.0..=1.0,
                                )
                                .text("Coyote Time"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_start_extra_gravity,
                                    0.0..=100.0,
                                )
                                .text("Jump Start Extra Gravity"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_fall_extra_gravity,
                                    0.0..=50.0,
                                )
                                .text("Jump Fall Extra Gravity"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_shorten_extra_gravity,
                                    0.0..=100.0,
                                )
                                .text("Jump Shorten Extra Gravity"),
                            );

                            let free_fall_options: [(bool, &str, fn() -> TnuaFreeFallBehavior); 3] = [
                                (
                                    matches!(platformer_config.free_fall_behavior, TnuaFreeFallBehavior::ExtraGravity(_)),
                                    "Extra Gravity",
                                    || TnuaFreeFallBehavior::ExtraGravity(0.0),
                                ),
                                (
                                    matches!(platformer_config.free_fall_behavior, TnuaFreeFallBehavior::LikeJumpShorten),
                                    "Like Jump Shorten",
                                    || TnuaFreeFallBehavior::LikeJumpShorten,
                                ),
                                (
                                    matches!(platformer_config.free_fall_behavior, TnuaFreeFallBehavior::LikeJumpFall),
                                    "Like Jump Fall",
                                    || TnuaFreeFallBehavior::LikeJumpFall,
                                ),
                            ];
                            egui::ComboBox::from_label("Free Fall Behavior")
                                .selected_text(free_fall_options.iter().find_map(|(chosen, name, _)| chosen.then_some(*name)).unwrap_or("???"))
                                .show_ui(ui, |ui| {
                                    for (chosen, name, make_variant) in free_fall_options {
                                        if ui.selectable_label(chosen, name).clicked() {
                                             platformer_config.free_fall_behavior = make_variant();
                                        }
                                    }
                                });
                            if let TnuaFreeFallBehavior::ExtraGravity(extra_gravity) = &mut platformer_config.free_fall_behavior {
                                ui.add(
                                    egui::Slider::new(extra_gravity, 0.0..=100.0).text("Extra Gravity"),
                                );
                            }

                            slider_or_infinity(ui, "Staying Upward Max Angular Velocity", &mut platformer_config.tilt_offset_angvel, 0.0..=20.0);
                            slider_or_infinity(ui, "Staying Upward Max Angular Acceleration", &mut platformer_config.tilt_offset_angacl, 0.0..=2000.0);

                            slider_or_infinity(ui, "Turning Angular Velocity", &mut platformer_config.turning_angvel, 0.0..=70.0);
                        });
                        ui.vertical(|ui| {
                            plot_source.show(entity, ui);
                        });
                    });
                });
        }
    });
}
