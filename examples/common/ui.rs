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
pub struct ControlFactors {
    pub speed: f32,
    pub jump_height: f32,
}

#[derive(Component)]
pub struct CommandAlteringSelectors(Vec<(usize, String, Vec<(String, fn(EntityCommands))>)>);

impl Default for CommandAlteringSelectors {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl CommandAlteringSelectors {
    pub fn with(mut self, caption: &str, options: &[(&str, fn(EntityCommands))]) -> Self {
        self.0.push((
            0,
            caption.to_owned(),
            options
                .into_iter()
                .map(|(name, applier)| (name.to_string(), *applier))
                .collect(),
        ));
        self
    }
}

fn ui_system(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<(
        Entity,
        &TrackedEntity,
        &PlotSource,
        &mut TnuaPlatformerConfig,
        &mut ControlFactors,
        Option<&mut CommandAlteringSelectors>,
    )>,
    mut commands: Commands,
) {
    egui::Window::new("Tnua").show(egui_context.ctx_mut(), |ui| {
        ui.label("Controls: Move with the arrow keys. Jump with Spacebar.");
        for (
            entity,
            TrackedEntity(name),
            plot_source,
            mut platformer_config,
            mut control_factors,
            command_altering_selectors,
        ) in query.iter_mut()
        {
            egui::CollapsingHeader::new(name)
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Slider::new(&mut control_factors.speed, 0.0..=60.0)
                                    .text("Speed"),
                            );
                            ui.add(
                                egui::Slider::new(&mut control_factors.jump_height, 0.0..=10.0)
                                    .text("Jump Height"),
                            );
                            control_factors.jump_height = control_factors.jump_height.max(0.1);

                            if let Some(mut command_altering_selectors) = command_altering_selectors
                            {
                                for (chosen, caption, options) in
                                    command_altering_selectors.0.iter_mut()
                                {
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
                                    0.0..=400.0,
                                )
                                .text("Spring Strengh"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.spring_dampening,
                                    0.0..=40.0,
                                )
                                .text("Spring Dampening"),
                            );
                            ui.add(
                                egui::Slider::new(&mut platformer_config.acceleration, 0.0..=200.0)
                                    .text("Acceleration"),
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
                        });
                        plot_source.show(entity, ui);
                    });
                });
        }
    });
}
