use crate::ghost_overrides::{TnuaGhostOverwrite, TnuaGhostOverwritesForBasis};
use crate::sensor_sets::TnuaSensors;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaProximitySensor;

#[derive(Copy, Clone)]
pub struct TnuaBuiltinWalkSensors<'a> {
    /// The main sensor of the floating character model.
    pub ground: &'a TnuaProximitySensor,

    /// An upward-facing sensor that checks for obstacles above the character.
    pub headroom: Option<&'a TnuaProximitySensor>,
}

impl<'a> TnuaSensors<'a> for TnuaBuiltinWalkSensors<'a> {
    type Entities = TnuaBuiltinWalkSensorsEntities;
    type GhostOverwrites = TnuaBuiltinWalkSensorsGhostOverwrites;
}

#[derive(Default)]
pub struct TnuaBuiltinWalkSensorsEntities {
    /// The main sensor of the floating character model.
    pub ground: Option<Entity>,

    /// An upward-facing sensor that checks for
    pub headroom: Option<Entity>,
}

#[derive(Component, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TnuaBuiltinWalkSensorsGhostOverwrites {
    /// The main sensor of the floating character model.
    pub ground: TnuaGhostOverwrite,
}

impl TnuaGhostOverwritesForBasis for TnuaBuiltinWalkSensorsGhostOverwrites {
    type Entities = TnuaBuiltinWalkSensorsEntities;
}
