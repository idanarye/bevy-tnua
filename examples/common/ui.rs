use std::marker::PhantomData;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_tnua::TnuaToggle;

use super::tuning::UiTunable;
use super::ui_plotting::PlotSource;
use super::FallingThroughControlScheme;

// TODO: Remove this once the old examples are overwritten
#[derive(Component, Default)]
pub struct DummyTunable;

impl UiTunable for DummyTunable {
    fn tune(&mut self, ui: &mut egui::Ui) {
        ui.label("DUMMY TUNABLE");
    }
}

pub struct ExampleUi<C: Component + UiTunable> {
    _phantom: PhantomData<C>,
}

impl<C: Component + UiTunable> Default for ExampleUi<C> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<C: Component + UiTunable> Plugin for ExampleUi<C> {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin);
        app.insert_resource(ExampleUiPhysicsBackendActive(true));
        app.add_systems(Update, ui_system::<C>);
        app.add_systems(Update, super::ui_plotting::plot_source_rolling_update);
        app.add_plugins(FrameTimeDiagnosticsPlugin);
    }
}

// NOTE: The examples are responsible for updating the physics backend
#[derive(Resource)]
pub struct ExampleUiPhysicsBackendActive(pub bool);

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
        set_to: Option<usize>,
    },
    Checkbox {
        checked: bool,
        caption: String,
        applier: fn(EntityCommands, bool),
        set_to: Option<bool>,
    },
}

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
                .into_iter()
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
}

fn ui_system<C: Component + UiTunable>(
    mut egui_context: EguiContexts,
    mut physics_backend_active: ResMut<ExampleUiPhysicsBackendActive>,
    mut query: Query<(
        Entity,
        &TrackedEntity,
        &PlotSource,
        &mut TnuaToggle,
        Option<&mut C>,
        &mut FallingThroughControlScheme,
        Option<&mut CommandAlteringSelectors>,
    )>,
    mut commands: Commands,
    diagnostics_store: Res<DiagnosticsStore>,
) {
    for (entity, .., command_altering_selectors) in query.iter_mut() {
        if let Some(mut command_altering_selectors) = command_altering_selectors {
            for selector in command_altering_selectors.0.iter_mut() {
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
    }
    egui::Window::new("Tnua").show(egui_context.ctx_mut(), |ui| {
        for (diagnostic_id, range) in [
            (FrameTimeDiagnosticsPlugin::FPS, 0.0..120.0),
            (FrameTimeDiagnosticsPlugin::FRAME_TIME, 0.0..50.0),
        ] {
            if let Some(diagnostic) = diagnostics_store.get(diagnostic_id) {
                if let Some(value) = diagnostic.smoothed() {
                    ui.add(
                        egui::widgets::ProgressBar::new((value as f32 - range.start) / (range.end - range.start))
                        .text(format!("{}: {:.0}", diagnostic.name, value))
                    );
                }
            }
        }
        egui::CollapsingHeader::new("Controls:")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Move with the arrow keys");
                ui.label("Jump with Spacebar (Also with the up arrow also works in 2D)");
                ui.label("Crouch or fall through pink platforms with Ctrl (Also with the down arrow key in 2D)");
                ui.label("Turn in place with Alt (only in 3D)");
                ui.label("Dash with Shift (while moving in a direction)");
            });
        ui.checkbox(&mut physics_backend_active.0, "Physics Backend Enabled");
        for (
            entity,
            TrackedEntity(name),
            plot_source,
            mut tnua_toggle,
            mut tunable,
            mut falling_through_control_scheme,
            command_altering_selectors,
        ) in query.iter_mut()
        {
            egui::CollapsingHeader::new(name)
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            egui::ComboBox::from_label("Toggle Tnua")
                                .selected_text(format!("{:?}", tnua_toggle.as_ref()))
                                .show_ui(ui, |ui| {
                                    for option in [
                                        TnuaToggle::Disabled,
                                        TnuaToggle::SenseOnly,
                                        TnuaToggle::Enabled,
                                    ] {
                                        let label = format!("{:?}", option);
                                        ui.selectable_value(tnua_toggle.as_mut(), option, label);
                                    }
                                });

                            if let Some(tunable) = tunable.as_mut() {
                                tunable.tune(ui);
                            }

                            if let Some(mut command_altering_selectors) = command_altering_selectors
                            {
                                for selector in command_altering_selectors.0.iter_mut() {
                                    match selector {
                                        CommandAlteringSelector::Combo { chosen, caption, options, set_to: _ } => {
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
                                        CommandAlteringSelector::Checkbox { checked, caption, applier, set_to: _ } => {
                                            if ui.checkbox(checked, caption.as_str()).clicked() {
                                                applier(commands.entity(entity), *checked);
                                            }
                                        }
                                    }
                                }
                            }

                            falling_through_control_scheme.edit_with_ui(ui);
                        });
                        ui.vertical(|ui| {
                            plot_source.show(entity, ui);
                        });
                    });
                });
        }
    });
}
