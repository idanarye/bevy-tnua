use bevy::prelude::*;

use crate::basis_trait::{BoxableBasis, DynamicBasis};
use crate::{TnuaBasis, TnuaPipelineStages, TnuaSystemSet, TnuaUserControlsSystemSet};

pub struct TnuaPlatformerPlugin2;

impl Plugin for TnuaPlatformerPlugin2 {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                TnuaPipelineStages::Sensors,
                TnuaPipelineStages::SubservientSensors,
                TnuaUserControlsSystemSet,
                TnuaPipelineStages::Logic,
                TnuaPipelineStages::Motors,
            )
                .chain()
                .in_set(TnuaSystemSet),
        );
        app.add_systems(
            Update,
            apply_controller_system.in_set(TnuaPipelineStages::Logic),
        );
        //app.add_systems(
        //Update,
        //handle_keep_crouching_below_obstacles.in_set(TnuaPipelineStages::SubservientSensors),
        //);
    }
}

#[derive(Component, Default)]
pub struct TnuaController {
    current_basis: Option<Box<dyn DynamicBasis>>,
}

impl TnuaController {
    pub fn basis<B: TnuaBasis>(&mut self, basis: B) -> &mut Self {
        if let Some(existing_basis) = self
            .current_basis
            .as_mut()
            .and_then(|b| b.as_mut_any().downcast_mut::<BoxableBasis<B>>())
        {
            info!("replacing");
            existing_basis.input = basis;
        } else {
            info!("setting");
            self.current_basis = Some(Box::new(BoxableBasis::new(basis)));
        }
        self
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system(time: Res<Time>, mut query: Query<(&mut TnuaController,)>) {
    let frame_duration = time.delta().as_secs_f32();
    if frame_duration == 0.0 {
        return;
    }
    for (mut controller,) in query.iter_mut() {
        if let Some(basis) = controller.current_basis.as_mut() {
            basis.apply();
        }
    }
}
