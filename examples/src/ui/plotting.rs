use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_egui::egui;
use egui_plot::{Corner, Legend, Plot};

#[derive(Component, Debug)]
pub struct PlotSource {
    input: Vec<Vec<(&'static str, f32)>>,
    fields: Vec<Vec<&'static str>>,
    rolling: VecDeque<f32>,
    last_update: f32,
    update_every: f32,
    keep: f32,
}

impl Default for PlotSource {
    fn default() -> Self {
        Self {
            input: Default::default(),
            fields: Default::default(),
            rolling: Default::default(),
            last_update: f32::NEG_INFINITY,
            update_every: 1.0 / 24.0,
            keep: 5.0,
        }
    }
}

impl PlotSource {
    pub fn set(&mut self, input: &[&[(&'static str, f32)]]) {
        if self.input.is_empty() {
            self.input = input.iter().map(|plot| plot.to_vec()).collect();
        } else {
            for (target_plot, source_plot) in self.input.iter_mut().zip(input) {
                for (target_curve, source_curve) in target_plot.iter_mut().zip(*source_plot) {
                    *target_curve = *source_curve;
                }
            }
        }
    }

    pub fn show(&self, entity: Entity, ui: &mut egui::Ui) {
        let mut plots_data = self
            .fields
            .iter()
            .map(|plot| {
                plot.iter()
                    .map(|_| Vec::<[f64; 2]>::new())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut it = self.rolling.iter();
        while let Some(timestamp) = it.next() {
            for plot_data in plots_data.iter_mut() {
                for curve in plot_data.iter_mut() {
                    curve.push([*timestamp as f64, *it.next().unwrap() as f64]);
                }
            }
        }
        for (i, (plot_fields, plot_data)) in self.fields.iter().zip(plots_data).enumerate() {
            let plot = Plot::new((entity, i))
                .legend(Legend::default().position(Corner::LeftBottom))
                .width(280.0)
                .height(180.0)
                .include_y(-20.0)
                .include_y(20.0)
                .show_axes([false, true]);
            plot.show(ui, |plot_ui| {
                for (field, curve) in plot_fields.iter().zip(plot_data) {
                    plot_ui.line(egui_plot::Line::new(curve).name(field));
                }
            });
        }
    }
}

pub fn plot_source_rolling_update(time: Res<Time>, mut query: Query<&mut PlotSource>) {
    let time = time.elapsed_seconds();
    for mut plot_source in query.iter_mut() {
        if plot_source.input.is_empty() {
            continue;
        }
        if time - plot_source.last_update < plot_source.update_every {
            continue;
        }
        let keep_from = time - plot_source.keep;
        plot_source.last_update = time;
        if plot_source.fields.is_empty() {
            plot_source.fields = plot_source
                .input
                .iter()
                .map(|plot| plot.iter().map(|(name, _)| *name).collect())
                .collect();
        }

        let record_width = 1 + plot_source
            .fields
            .iter()
            .map(|flds| flds.len())
            .sum::<usize>();
        while let Some(timestamp) = plot_source.rolling.front() {
            assert!(0 < record_width);
            if keep_from <= *timestamp {
                break;
            }
            plot_source.rolling.drain(0..record_width);
        }

        plot_source.rolling.push_back(time);
        {
            let PlotSource { input, rolling, .. } = &mut *plot_source;
            rolling.extend(
                input
                    .iter()
                    .flat_map(|plot| plot.iter().map(|(_, value)| *value)),
            );
        }
    }
}

pub fn make_update_plot_data_system<V: Component>(
    get_linvel: impl 'static + Send + Sync + Fn(&V) -> Vec3,
) -> bevy::ecs::schedule::SystemConfigs {
    (move |mut query_rapier2d: Query<(&mut PlotSource, &Transform, &V)>| {
        for (mut plot_source, transform, velocity) in query_rapier2d.iter_mut() {
            let linvel = get_linvel(velocity);
            plot_source.set(&[
                &[("Y", transform.translation.y), ("vel-Y", linvel.y)],
                &[("X", transform.translation.x), ("vel-X", linvel.x)],
            ]);
        }
    })
    .into_configs()
}
