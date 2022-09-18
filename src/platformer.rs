use bevy::prelude::*;

use crate::{tnua_system_set_for_computing_logic, TnuaProximitySensor};

pub struct TnuaPlatformerPlugin;

impl Plugin for TnuaPlatformerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            tnua_system_set_for_computing_logic().with_system(platformer_control_system),
        );
    }
}

#[derive(Component)]
pub struct TnuaPlatformControls {
    pub up: Vec3,
    pub float_at: f32,
    pub move_direction: Vec3,
}

impl Default for TnuaPlatformControls {
    fn default() -> Self {
        Self {
            up: Vec3::Y,
            float_at: 0.5, // TODO: This should not have a default
            move_direction: Vec3::ZERO,
        }
    }
}

fn platformer_control_system(query: Query<(&TnuaPlatformControls, &TnuaProximitySensor)>) {
    for (controls, sensor) in query.iter() {
        info!(
            "{} above ground, going {}",
            sensor.proximity, controls.move_direction
        );
    }
}
