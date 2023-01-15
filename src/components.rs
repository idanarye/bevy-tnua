use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct TnuaProximitySensor {
    pub cast_origin: Vec3,
    pub cast_direction: Vec3,
    pub cast_range: f32,
    pub output: Option<TnuaProximitySensorOutput>,
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
}
