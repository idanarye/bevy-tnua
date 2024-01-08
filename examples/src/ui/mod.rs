pub mod component_alterbation;
pub mod plotting;
pub mod tuning;

use std::marker::PhantomData;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_tnua::TnuaToggle;

use self::component_alterbation::CommandAlteringSelectors;
use self::plotting::{plot_source_rolling_update, update_plot_data};

use super::FallingThroughControlScheme;
use plotting::PlotSource;
use tuning::UiTunable;

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
        app.add_systems(Update, plot_source_rolling_update);
        app.add_plugins(FrameTimeDiagnosticsPlugin);
        app.add_systems(Update, update_plot_data);
        app.add_systems(Update, update_physics_active_from_ui);
    }
}

// NOTE: The examples are responsible for updating the physics backend
#[derive(Resource)]
pub struct ExampleUiPhysicsBackendActive(pub bool);

#[derive(Component)]
pub struct TrackedEntity(pub String);

#[allow(clippy::type_complexity)]
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
            command_altering_selectors.apply_set_to(&mut commands, entity);
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
                                command_altering_selectors.show_ui(ui, &mut commands, entity);
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

fn update_physics_active_from_ui(
    setting_from_ui: Res<ExampleUiPhysicsBackendActive>,
    #[cfg(feature = "rapier2d")] mut config_rapier2d: ResMut<
        bevy_rapier2d::plugin::RapierConfiguration,
    >,
    #[cfg(feature = "rapier3d")] mut config_rapier3d: ResMut<
        bevy_rapier3d::plugin::RapierConfiguration,
    >,
    #[cfg(feature = "xpbd2d")] mut physics_time_xpbd2d: ResMut<
        Time<bevy_xpbd_2d::plugins::setup::Physics>,
    >,
    #[cfg(feature = "xpbd3d")] mut physics_time_xpbd3d: ResMut<
        Time<bevy_xpbd_3d::plugins::setup::Physics>,
    >,
) {
    #[cfg(feature = "rapier2d")]
    {
        config_rapier2d.physics_pipeline_active = setting_from_ui.0;
    }
    #[cfg(feature = "rapier3d")]
    {
        config_rapier3d.physics_pipeline_active = setting_from_ui.0;
    }
    #[cfg(feature = "xpbd2d")]
    {
        use bevy_xpbd_2d::plugins::setup::PhysicsTime;
        if setting_from_ui.0 {
            physics_time_xpbd2d.unpause();
        } else {
            physics_time_xpbd2d.pause();
        }
    }
    #[cfg(feature = "xpbd3d")]
    {
        use bevy_xpbd_3d::plugins::setup::PhysicsTime;
        if setting_from_ui.0 {
            physics_time_xpbd3d.unpause();
        } else {
            physics_time_xpbd3d.pause();
        }
    }
}
