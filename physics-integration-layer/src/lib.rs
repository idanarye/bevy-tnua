use bevy::prelude::*;

pub mod data_for_backends;
pub mod subservient_sensors;

/// Umbrella system set for [`TnuaPipelineStages`].
///
/// The physics backends' plugins are responsible for preventing this entire system set from
/// running when the physics backend itself is paused.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct TnuaSystemSet;

/// The various stages of the Tnua pipeline.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum TnuaPipelineStages {
    /// Data is read from the physics backend.
    Sensors,
    /// Data is propagated through the subservient sensors.
    SubservientSensors,
    /// Tnua decieds how the entity should be manipulated.
    Logic,
    /// Forces are applied in the physiscs backend.
    Motors,
}
