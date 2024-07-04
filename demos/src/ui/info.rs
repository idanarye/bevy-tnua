use std::collections::BTreeMap;

use bevy::prelude::*;
#[cfg(feature = "egui")]
use bevy_egui::egui;

#[derive(Component, Default)]
pub struct InfoSource {
    is_active: bool,
    data: BTreeMap<String, Option<InfoBit>>,
}

pub enum InfoBit {
    Label(#[cfg(feature = "egui")] egui::WidgetText),
}

impl InfoSource {
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        if !active {
            self.data.clear();
        }
    }

    #[cfg(feature = "egui")]
    pub(crate) fn show(&mut self, _entity: Entity, ui: &mut egui::Ui) {
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .column(egui_extras::Column::auto().at_least(80.0).resizable(true))
            .column(egui_extras::Column::remainder())
            .body(|mut body| {
                self.data.retain(|key, info_bit| {
                    let Some(info_bit) = info_bit.take() else {
                        return false;
                    };

                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            ui.strong(key);
                        });
                        row.col(|ui| match info_bit {
                            InfoBit::Label(label) => {
                                ui.label(label);
                            }
                        });
                    });

                    true
                });
            });
    }

    fn set_info_bit(&mut self, key: &str, info_bit: InfoBit) {
        if let Some(value) = self.data.get_mut(key) {
            *value = Some(info_bit);
        } else {
            self.data.insert(key.to_owned(), Some(info_bit));
        }
    }

    #[cfg(feature = "egui")]
    pub fn label(&mut self, key: &str, label: impl Into<egui::WidgetText>) {
        self.set_info_bit(key, InfoBit::Label(label.into()));
    }
    #[cfg(not(feature = "egui"))]
    pub fn label<T>(&mut self, key: &str, _: T) {
        self.set_info_bit(key, InfoBit::Label());
    }
}
