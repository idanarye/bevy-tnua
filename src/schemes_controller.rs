use std::marker::PhantomData;

use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use crate::schemes_traits::TnuaScheme;
use crate::{
    TnuaMotor, TnuaPipelineSystems, TnuaProximitySensor, TnuaRigidBodyTracker, TnuaSystems,
    TnuaToggle, TnuaUserControlsSystems,
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
    pub config: Handle<S::Config>,
}

impl<S: TnuaScheme> Tnua2Controller<S> {
    pub fn new(config: Handle<S::Config>) -> Self {
        Self {
            basis: Default::default(),
            config,
        }
    }
}

#[allow(clippy::type_complexity)]
fn apply_controller_system<S: TnuaScheme>(
    mut query: Query<(
        &mut Tnua2Controller<S>,
        &TnuaRigidBodyTracker,
        &mut TnuaProximitySensor,
        &mut TnuaMotor,
        Option<&TnuaToggle>,
    )>,
) {
    for (mut controller, _tracker, mut _sensor, mut _motor, tnua_toggle) in query.iter_mut() {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }

        let _controller = controller.as_mut();
    }
}
