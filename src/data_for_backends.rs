use bevy::prelude::*;

#[derive(Resource)]
pub struct TnuaDataSynchronizedFromBackend {
    pub gravity: Vec3,
}

impl Default for TnuaDataSynchronizedFromBackend {
    fn default() -> Self {
        Self {
            gravity: -9.8 * Vec3::Y,
        }
    }
}

#[derive(Component, Debug)]
pub struct TnuaProximitySensor {
    pub cast_origin: Vec3,
    pub cast_direction: Vec3,
    pub cast_range: f32,
    pub velocity: Vec3,
    pub angvel: Vec3,
    pub output: Option<TnuaProximitySensorOutput>,
}

impl Default for TnuaProximitySensor {
    fn default() -> Self {
        Self {
            cast_origin: Vec3::ZERO,
            cast_direction: -Vec3::Y,
            cast_range: 0.0,
            velocity: Vec3::ZERO,
            angvel: Vec3::ZERO,
            output: None,
        }
    }
}

#[derive(Debug)]
pub struct TnuaProximitySensorOutput {
    pub entity: Entity,
    pub proximity: f32,
    pub normal: Vec3,
    pub relative_velocity: Vec3,
}

#[derive(Component, Default)]
pub struct TnuaMotor {
    pub desired_acceleration: Vec3,
    pub desired_angacl: Vec3,
}
