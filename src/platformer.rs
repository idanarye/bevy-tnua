use bevy::prelude::*;

use crate::{tnua_system_set_for_computing_logic, TnuaMotor, TnuaProximitySensor};

pub struct TnuaPlatformerPlugin;

impl Plugin for TnuaPlatformerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            tnua_system_set_for_computing_logic().with_system(platformer_control_system),
        );
    }
}

#[derive(Component)]
pub struct TnuaPlatformerConfig {
    pub ride_height: f32,
    pub spring_strengh: f32,
    pub spring_dampening: f32,
}

#[derive(Component)]
pub struct TnuaPlatformerControls {
    pub up: Vec3,
    pub float_at: f32,
    pub move_direction: Vec3,
}

impl Default for TnuaPlatformerControls {
    fn default() -> Self {
        Self {
            up: Vec3::Y,
            float_at: 1.5, // TODO: This should not have a default
            move_direction: Vec3::ZERO,
        }
    }
}

fn platformer_control_system(
    mut query: Query<(
        &TnuaPlatformerControls,
        &TnuaPlatformerConfig,
        &TnuaProximitySensor,
        &mut TnuaMotor,
    )>,
) {
    for (controls, config, sensor, mut motor) in query.iter_mut() {
        if let Some(sensor_output) = &sensor.output {
            let spring_offset = controls.float_at - sensor_output.proximity;
            let spring_force = spring_offset * config.spring_strengh /* subtract dumpning */;

            let relative_velocity =
                sensor_output.relative_velocity.dot(sensor.cast_direction) * sensor.cast_direction;

            let dampening_force = relative_velocity * config.spring_dampening;
            let spring_force = spring_force - dampening_force;
            motor.desired_acceleration = controls.up * spring_force;
        } else {
            motor.desired_acceleration = Vec3::ZERO;
        }
    }
}
