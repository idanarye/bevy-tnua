use bevy::prelude::*;

use crate::basis_trait::{BoxableBasis, DynamicBasis, TnuaBasisContext};
use crate::{
    TnuaBasis, TnuaMotor, TnuaPipelineStages, TnuaProximitySensor, TnuaRigidBodyTracker,
    TnuaSystemSet, TnuaUserControlsSystemSet,
};

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
    current_basis: Option<(&'static str, Box<dyn DynamicBasis>)>,
}

impl TnuaController {
    pub fn basis<B: TnuaBasis>(&mut self, name: &'static str, basis: B) -> &mut Self {
        if let Some((existing_name, existing_basis)) =
            self.current_basis.as_mut().and_then(|(n, b)| {
                let b = b.as_mut_any().downcast_mut::<BoxableBasis<B>>()?;
                Some((n, b))
            })
        {
            *existing_name = name;
            existing_basis.input = basis;
        } else {
            self.current_basis = Some((name, Box::new(BoxableBasis::new(basis))));
        }
        self
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system(
    time: Res<Time>,
    mut query: Query<(
        &mut TnuaController,
        &TnuaRigidBodyTracker,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
    )>,
) {
    let frame_duration = time.delta().as_secs_f32();
    if frame_duration == 0.0 {
        return;
    }
    for (mut controller, tracker, mut sensor, mut motor) in query.iter_mut() {
        if let Some((_, basis)) = controller.current_basis.as_mut() {
            basis.apply(
                TnuaBasisContext {
                    frame_duration: time.delta(),
                    tracker,
                    proximity_sensor: sensor.as_ref(),
                },
                motor.as_mut(),
            );
            let sensor_cast_range = basis.proximity_sensor_cast_range();
            sensor.cast_range = sensor_cast_range;
        }
    }
}
