use crate::sensor_sets::TnuaSensors;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaProximitySensor;

#[derive(Copy, Clone)]
pub struct TnuaBuiltinWalkSensors<'a> {
    pub ground: &'a TnuaProximitySensor,
    pub headroom: Option<&'a TnuaProximitySensor>,
}

impl<'a> TnuaSensors<'a> for TnuaBuiltinWalkSensors<'a> {
    type Entities = TnuaBuiltinWalkSensorsEntities;
}

#[derive(Default)]
pub struct TnuaBuiltinWalkSensorsEntities {
    pub ground: Option<Entity>,
    pub headroom: Option<Entity>,
}
