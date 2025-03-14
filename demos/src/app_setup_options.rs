use bevy::prelude::*;
#[cfg(feature = "egui")]
use bevy_egui::egui;
use clap::{Parser, ValueEnum};

#[derive(Resource, Debug, Parser, Clone)]
pub struct AppSetupConfiguration {
    //#[arg(long = "schedule", default_value = "update")]
    #[arg(long = "schedule", default_value_t = if cfg!(feature = "avian") {
        ScheduleToUse::FixedUpdate
    } else {
        ScheduleToUse::Update
    })]
    pub schedule_to_use: ScheduleToUse,
    #[arg(long = "level")]
    pub level_to_load: Option<String>,
}

impl AppSetupConfiguration {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_environment() -> Self {
        Self::parse()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_environment() -> Self {
        let window = web_sys::window().expect("WASM must run inside window");
        let url_params =
            web_sys::UrlSearchParams::new_with_str(&window.location().search().unwrap()).unwrap();
        Self {
            schedule_to_use: if let Some(value) = url_params.get("schedule") {
                ScheduleToUse::from_str(&value, true).unwrap()
            } else if cfg!(feature = "avian") {
                ScheduleToUse::FixedUpdate
            } else {
                #[cfg(feature = "avian")]
                {
                    ScheduleToUse::FixedUpdate
                }
                #[cfg(feature = "rapier")]
                {
                    ScheduleToUse::Update
                }
                #[cfg(all(not(feature = "avian"), not(feature = "rapier")))]
                {
                    panic!("No schedule was specified, but also no physics engine is avaible. Therefore, there is no fallback.")
                }
            },
            level_to_load: url_params.get("level"),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn change_and_reload_page(&self, change_dlg: impl FnOnce(&mut Self)) {
        let mut new_cfg = self.clone();
        change_dlg(&mut new_cfg);
        let url_params = web_sys::UrlSearchParams::new().unwrap();

        if let Some(value) = new_cfg.schedule_to_use.to_possible_value() {
            url_params.append("schedule", value.get_name());
        }

        let window = web_sys::window().expect("WASM must run inside window");
        window
            .location()
            .set_search(
                &url_params
                    .to_string()
                    .as_string()
                    .expect("search params could not be converted to string"),
            )
            .unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum ScheduleToUse {
    Update,
    FixedUpdate,
}

impl std::fmt::Display for ScheduleToUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Update => "update",
            Self::FixedUpdate => "fixed-update",
        })
    }
}

impl ScheduleToUse {
    #[cfg(feature = "egui")]
    pub fn pick_different_option(&self, ui: &mut egui::Ui) -> Option<Self> {
        let response = egui::ComboBox::from_label("Schedule (changing it will restart the demo)")
            .selected_text(
                self.to_possible_value()
                    .expect("schedule with no value")
                    .get_name(),
            )
            .show_ui(ui, |ui| {
                for choice in Self::value_variants() {
                    if let Some(value) = choice.to_possible_value() {
                        if ui
                            .selectable_label(choice == self, value.get_name())
                            .clicked()
                        {
                            return Some(choice.clone());
                        }
                    }
                }
                None
            });
        response.inner.flatten()
    }
}
