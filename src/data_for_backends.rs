use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct TnuaRigidBodyTracker {
    pub velocity: Vec3,
    pub angvel: Vec3,
    pub gravity: Vec3,
}

impl Default for TnuaRigidBodyTracker {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angvel: Vec3::ZERO,
            gravity: Vec3::ZERO,
        }
    }
}

#[derive(Component, Debug)]
pub struct TnuaProximitySensor {
    pub cast_origin: Vec3,
    pub cast_direction: Vec3,
    pub cast_range: f32,
    pub output: Option<TnuaProximitySensorOutput>,
}

impl Default for TnuaProximitySensor {
    fn default() -> Self {
        Self {
            cast_origin: Vec3::ZERO,
            cast_direction: -Vec3::Y,
            cast_range: 0.0,
            output: None,
        }
    }
}

#[derive(Debug)]
pub struct TnuaProximitySensorOutput {
    pub entity: Entity,
    pub proximity: f32,
    pub normal: Vec3,
    pub entity_linvel: Vec3,
    pub entity_angvel: Vec3,
}

#[derive(Component, Default)]
pub struct TnuaMotor {
    pub desired_acceleration: Vec3,
    pub desired_angacl: Vec3,
}
