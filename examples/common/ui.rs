use bevy::prelude::*;
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_tnua::TnuaPlatformerConfig;

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

fn ui_system(
    mut egui_context: ResMut<EguiContext>,
    mut query: Query<(
        Entity,
        &TrackedEntity,
        &PlotSource,
        &mut TnuaPlatformerConfig,
        &mut ControlFactors,
    )>,
) {
    egui::Window::new("Tnua").show(egui_context.ctx_mut(), |ui| {
        for (
            entity,
            TrackedEntity(name),
            plot_source,
            mut platformer_config,
            mut control_factors,
        ) in query.iter_mut()
        {
            egui::CollapsingHeader::new(name)
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
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
                                egui::Slider::new(&mut platformer_config.jump_impulse, 0.0..=40.0)
                                    .text("Jump Impulse"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_height_reached_fall_speed,
                                    -10.0..=20.0,
                                )
                                .text("Jump Height Reached Fall Speed"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_height_reached_acceleration,
                                    0.0..=400.0,
                                )
                                .text("Jump Height Reached Acceleration"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_shorted_fall_speed,
                                    -10.0..=20.0,
                                )
                                .text("Jump Shorted Fall Speed"),
                            );
                            ui.add(
                                egui::Slider::new(
                                    &mut platformer_config.jump_shorted_acceleration,
                                    0.0..=400.0,
                                )
                                .text("Jump Shorted Acceleration"),
                            );
                            ui.add(
                                egui::Slider::new(&mut control_factors.speed, 0.0..=60.0)
                                    .text("Speed"),
                            );
                            ui.add(
                                egui::Slider::new(&mut control_factors.jump_height, 0.0..=10.0)
                                    .text("Jump Height"),
                            );
                        });
                        plot_source.show(entity, ui);
                    });
                });
        }
    });
}
