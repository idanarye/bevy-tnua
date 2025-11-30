use std::marker::PhantomData;

use crate::math::*;
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use crate::schemes_traits::{Tnua2Basis, TnuaScheme, TnuaSchemeConfig};
use crate::{
    TnuaBasisContext, TnuaMotor, TnuaPipelineSystems, TnuaProximitySensor, TnuaRigidBodyTracker,
    TnuaSystems, TnuaToggle, TnuaUserControlsSystems,
};

pub struct Tnua2ControllerPlugin<S: TnuaScheme> {
    schedule: InternedScheduleLabel,
    _phantom: PhantomData<S>,
}

impl<S: TnuaScheme> Tnua2ControllerPlugin<S> {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
            _phantom: PhantomData,
        }
    }
}

impl<S: TnuaScheme> Plugin for Tnua2ControllerPlugin<S> {
    fn build(&self, app: &mut App) {
        app.init_asset::<S::Config>();
        app.configure_sets(
            self.schedule,
            (
                TnuaPipelineSystems::Sensors,
                TnuaPipelineSystems::SubservientSensors,
                TnuaUserControlsSystems,
                TnuaPipelineSystems::Logic,
                TnuaPipelineSystems::Motors,
            )
                .chain()
                .in_set(TnuaSystems),
        );
        app.add_systems(
            self.schedule,
            apply_controller_system::<S>.in_set(TnuaPipelineSystems::Logic),
        );
    }
}

#[derive(Component)]
#[require(TnuaMotor, TnuaRigidBodyTracker, TnuaProximitySensor)]
pub struct Tnua2Controller<S: TnuaScheme> {
    pub basis: S::Basis,
    basis_memory: <S::Basis as Tnua2Basis>::Memory,
    pub config: Handle<S::Config>,
}

impl<S: TnuaScheme> Tnua2Controller<S> {
    pub fn new(config: Handle<S::Config>) -> Self {
        Self {
            basis: Default::default(),
            basis_memory: Default::default(),
            config,
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system<S: TnuaScheme>(
    time: Res<Time>,
    mut query: Query<(
        &mut Tnua2Controller<S>,
        &TnuaRigidBodyTracker,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
        Option<&TnuaToggle>,
    )>,
    config_assets: Res<Assets<S::Config>>,
) {
    let frame_duration = time.delta().as_secs_f64() as Float;
    if frame_duration == 0.0 {
        return;
    }
    for (mut controller, tracker, mut sensor, mut motor, tnua_toggle) in query.iter_mut() {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }
        let controller = controller.as_mut();

        let Some(config) = config_assets.get(&controller.config) else {
            continue;
        };
        let basis_config = config.basis_config();

        let up_direction = Dir3::new(-tracker.gravity.f32()).ok();
        let up_direction = up_direction.unwrap_or(Dir3::Y);

        let sensor = sensor.as_mut();

        controller.basis.apply(
            basis_config,
            &mut controller.basis_memory,
            TnuaBasisContext {
                frame_duration,
                tracker,
                proximity_sensor: sensor,
                up_direction,
            },
            &mut motor,
        );
        let sensor_cast_range_for_basis = controller
            .basis
            .proximity_sensor_cast_range(basis_config, &controller.basis_memory);

        let sensor_cast_range_for_action = 0.0; // TODO - base this on the action if there is one

        sensor.cast_range = sensor_cast_range_for_basis.max(sensor_cast_range_for_action);
        sensor.cast_direction = -up_direction;
        // TODO: Maybe add the horizontal rotation as well somehow?
        sensor.cast_shape_rotation =
            Quaternion::from_rotation_arc(Vector3::Y, up_direction.adjust_precision())
    }
}
