use crate::ghost_overrides::{TnuaGhostOverwrite, TnuaGhostOverwritesForBasis};
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
    type GhostOverwrites = TnuaBuiltinWalkSensorsGhostOverwrites;
}

#[derive(Default)]
pub struct TnuaBuiltinWalkSensorsEntities {
    pub ground: Option<Entity>,
    pub headroom: Option<Entity>,
}

#[derive(Component, Default)]
pub struct TnuaBuiltinWalkSensorsGhostOverwrites {
    pub ground: TnuaGhostOverwrite,
}

impl TnuaGhostOverwritesForBasis for TnuaBuiltinWalkSensorsGhostOverwrites {
    type Entities = TnuaBuiltinWalkSensorsEntities;
}
