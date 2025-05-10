use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    ecs::system::SystemParam,
    prelude::*,
};
use bevy_egui::egui;
#[cfg(feature = "framepace")]
use bevy_framepace::{
    debug::DiagnosticsPlugin as FramepaceDiagnosticsPlugin, FramepacePlugin, FramepaceSettings,
    Limiter,
};

pub struct DemoFrameratePlugin;

impl Plugin for DemoFrameratePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());

        #[cfg(feature = "framepace")]
        app.add_plugins((FramepacePlugin, FramepaceDiagnosticsPlugin));
    }
}

#[derive(SystemParam)]
pub struct DemoFramerateParam<'w> {
    diagnostics_store: Res<'w, DiagnosticsStore>,
    #[cfg(feature = "framepace")]
    framepace_settings: ResMut<'w, FramepaceSettings>,
}

impl DemoFramerateParam<'_> {
    pub fn show_in_ui(&mut self, ui: &mut egui::Ui) {
        for (diagnostic_path, range) in [
            (FrameTimeDiagnosticsPlugin::FPS, 0.0..120.0),
            (FrameTimeDiagnosticsPlugin::FRAME_TIME, 0.0..50.0),
            #[cfg(feature = "framepace")]
            (FramepaceDiagnosticsPlugin::FRAMEPACE_FRAMETIME, 0.0..50.0),
            #[cfg(feature = "framepace")]
            (
                FramepaceDiagnosticsPlugin::FRAMEPACE_OVERSLEEP,
                0.0..40_000.0,
            ),
        ] {
            if let Some(diagnostic) = self.diagnostics_store.get(&diagnostic_path) {
                if let Some(value) = diagnostic.smoothed() {
                    ui.add(
                        egui::widgets::ProgressBar::new(
                            (value as f32 - range.start) / (range.end - range.start),
                        )
                        .text(format!("{}: {:.0}", diagnostic_path, value)),
                    );
                }
            }
        }
        #[cfg(feature = "framepace")]
        {
            use std::time::Duration;

            let limiter = &mut self.framepace_settings.limiter;
            egui::ComboBox::from_label("Framepace Limiter")
                .selected_text(match limiter {
                    Limiter::Auto => "auto",
                    Limiter::Manual(_) => "manual",
                    Limiter::Off => "off",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(matches!(limiter, Limiter::Auto), "auto")
                        .clicked()
                    {
                        *limiter = Limiter::Auto;
                    }
                    if ui
                        .selectable_label(matches!(limiter, Limiter::Manual(_)), "manual")
                        .clicked()
                    {
                        #[allow(clippy::collapsible_if)]
                        if !matches!(limiter, Limiter::Manual(_)) {
                            *limiter = Limiter::Manual(Duration::from_secs_f32(1.0 / 60.0));
                        }
                    }
                    if ui
                        .selectable_label(matches!(limiter, Limiter::Off), "off")
                        .clicked()
                    {
                        *limiter = Limiter::Off;
                    }
                });
            if let Limiter::Manual(limit) = limiter {
                const MIN_FPS: f64 = 1.0;
                const MAX_FPS: f64 = 120.0;
                let mut limit_secs = limit.as_secs_f64();

                let mut fps_limit = 1.0 / limit_secs;
                ui.add(egui::Slider::new(&mut fps_limit, MIN_FPS..=MAX_FPS).text("FPS Limit"));
                limit_secs = 1.0 / fps_limit;

                ui.add(
                    egui::Slider::new(&mut limit_secs, (1.0 / MIN_FPS)..=(1.0 / MAX_FPS))
                        .text("Frame Time Limit"),
                );

                *limit = Duration::from_secs_f64(limit_secs);
            }
        }
    }
}
