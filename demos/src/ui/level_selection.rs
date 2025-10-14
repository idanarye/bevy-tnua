#[allow(unused_imports)]
use bevy::{ecs::system::SystemParam, prelude::*};
#[cfg(feature = "egui")]
use bevy_egui::egui;

use crate::levels_setup::level_switching::{SwitchToLevel, SwitchableLevels};

#[derive(SystemParam)]
#[allow(unused)]
pub struct LevelSelectionParam<'w> {
    switchable_levels: Option<Res<'w, SwitchableLevels>>,
    writer: Option<ResMut<'w, Messages<SwitchToLevel>>>,
}

impl LevelSelectionParam<'_> {
    #[cfg(feature = "egui")]
    pub fn show_in_ui(&mut self, ui: &mut egui::Ui) {
        let (Some(switchable_levels), Some(writer)) =
            (self.switchable_levels.as_ref(), self.writer.as_mut())
        else {
            return;
        };
        let response = egui::ComboBox::from_label("Select Level")
            .selected_text(switchable_levels.current().name())
            .show_ui(ui, |ui| {
                for (idx, level) in switchable_levels.iter().enumerate() {
                    if ui
                        .selectable_label(idx == switchable_levels.current, level.name())
                        .clicked()
                    {
                        return Some(idx);
                    }
                }
                None
            });
        if let Some(new_idx) = response.inner.flatten() {
            writer.write(SwitchToLevel(new_idx));
        }
    }
}
