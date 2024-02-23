use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
#[cfg(feature = "egui")]
use bevy_egui::egui;

#[derive(Component, Default)]
pub struct CommandAlteringSelectors(Vec<CommandAlteringSelector>);

enum CommandAlteringSelector {
    Combo {
        chosen: usize,
        #[cfg_attr(not(feature = "egui"), allow(unused))]
        caption: String,
        options: Vec<(String, fn(EntityCommands))>,
        set_to: Option<usize>,
    },
    Checkbox {
        checked: bool,
        #[cfg_attr(not(feature = "egui"), allow(unused))]
        caption: String,
        applier: fn(EntityCommands, bool),
        set_to: Option<bool>,
    },
}

#[allow(clippy::type_complexity)]
impl CommandAlteringSelectors {
    pub fn with_combo(
        mut self,
        caption: &str,
        initial: usize,
        options: &[(&str, fn(EntityCommands))],
    ) -> Self {
        self.0.push(CommandAlteringSelector::Combo {
            chosen: 0,
            caption: caption.to_owned(),
            options: options
                .iter()
                .map(|(name, applier)| (name.to_string(), *applier))
                .collect(),
            set_to: Some(initial),
        });
        self
    }

    pub fn with_checkbox(
        mut self,
        caption: &str,
        initial: bool,
        applier: fn(EntityCommands, bool),
    ) -> Self {
        self.0.push(CommandAlteringSelector::Checkbox {
            checked: false,
            caption: caption.to_owned(),
            applier,
            set_to: Some(initial),
        });
        self
    }

    pub fn apply_set_to(&mut self, commands: &mut Commands, entity: Entity) {
        for selector in self.0.iter_mut() {
            match selector {
                CommandAlteringSelector::Combo {
                    chosen,
                    caption: _,
                    options,
                    set_to,
                } => {
                    if let Some(set_to) = set_to.take() {
                        *chosen = set_to;
                        options[set_to].1(commands.entity(entity));
                    }
                }
                CommandAlteringSelector::Checkbox {
                    checked,
                    caption: _,
                    applier,
                    set_to,
                } => {
                    if let Some(set_to) = set_to.take() {
                        *checked = set_to;
                        applier(commands.entity(entity), set_to);
                    }
                }
            }
        }
    }

    #[cfg(feature = "egui")]
    pub fn show_ui(&mut self, ui: &mut egui::Ui, commands: &mut Commands, entity: Entity) {
        for selector in self.0.iter_mut() {
            match selector {
                CommandAlteringSelector::Combo {
                    chosen,
                    caption,
                    options,
                    set_to: _,
                } => {
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
                CommandAlteringSelector::Checkbox {
                    checked,
                    caption,
                    applier,
                    set_to: _,
                } => {
                    if ui.checkbox(checked, caption.as_str()).clicked() {
                        applier(commands.entity(entity), *checked);
                    }
                }
            }
        }
    }
}
