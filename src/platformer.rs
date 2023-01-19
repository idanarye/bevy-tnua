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
    pub spring_strengh: f32,
    pub spring_dampening: f32,
    pub acceleration: f32,
}

#[derive(Component)]
pub struct TnuaPlatformerControls {
    pub up: Vec3,
    pub float_at: f32,
    pub move_direction: Vec3,
}

impl TnuaPlatformerControls {
    pub fn new_floating_at(float_at: f32) -> Self {
        Self {
            up: Vec3::Y,
            float_at,
            move_direction: Vec3::ZERO,
        }
    }
}

fn platformer_control_system(
    time: Res<Time>,
    mut query: Query<(
        &TnuaPlatformerControls,
        &TnuaPlatformerConfig,
        &TnuaProximitySensor,
        &mut TnuaMotor,
    )>,
) {
    for (controls, config, sensor, mut motor) in query.iter_mut() {
        let effective_velocity;
        if let Some(sensor_output) = &sensor.output {
            let spring_offset = controls.float_at - sensor_output.proximity;
            let spring_force = spring_offset * config.spring_strengh /* subtract dumpning */;

            let relative_velocity =
                sensor_output.relative_velocity.dot(sensor.cast_direction) * sensor.cast_direction;

            let dampening_force = relative_velocity * config.spring_dampening;
            let spring_force = spring_force - dampening_force;
            motor.desired_acceleration = time.delta().as_secs_f32() * controls.up * spring_force;
            effective_velocity = sensor_output.relative_velocity;
        } else {
            motor.desired_acceleration = Vec3::ZERO;
            effective_velocity = sensor.velocity;
        }

        let velocity_on_plane =
            effective_velocity - controls.up * controls.up.dot(effective_velocity);

        let desired_velocity = controls.move_direction;
        let exact_acceleration = desired_velocity - velocity_on_plane;

        let safe_direction_coefficient = desired_velocity
            .normalize_or_zero()
            .dot(velocity_on_plane.normalize_or_zero());
        let direction_change_factor = 1.5 - 0.5 * safe_direction_coefficient;

        let acceleration = direction_change_factor * config.acceleration;

        let capped_acceperation =
            exact_acceleration.clamp_length_max(time.delta().as_secs_f32() * acceleration);

        // TODO: Do I need maximum force capping?

        motor.desired_acceleration += capped_acceperation;
    }
}