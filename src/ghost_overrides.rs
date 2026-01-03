use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaProximitySensorOutput;

use crate::sensor_sets::TnuaSensors;
use crate::{TnuaBasis, TnuaScheme};

/// A struct with fields of [`TnuaGhostOverwrite`] for each sensor that can have a ghost sensor.
pub trait TnuaGhostOverwritesForBasis: 'static + Send + Sync + Default {
    /// A struct that points to the sensor entities. Must match the entities of [the basis'
    /// sensors](TnuaBasis::Sensors).
    type Entities: 'static + Send + Sync + Default;
}

/// Add this component to an entity with a [`TnuaController`](crate::TnuaController) (that has the
/// same control scheme) to generate ghost sensors and to control them.
///
/// This component holds and refers to a struct defined by the
/// [`GhostOverwrites`](TnuaSensors::GhostOverwrites) of the [`Sensors`](TnuaBasis::Sensors) of the
/// control scheme's basis. The fields of that struct should be of type [`TnuaGhostOverwrite`], and
/// should have matching fields in the sensors' [`Entities`](TnuaSensors::Entities) (accessible via
/// the controller's [`sensors_entities`](crate::TnuaController::sensors_entities)) that point to
/// entities with a [`TnuaGhostSensor`](crate::TnuaGhostSensor) component on them that holds the
/// ghost hits that can be set in the [`TnuaGhostOverwrite`] using its
/// [`set`](TnuaGhostOverwrite::set) method.
#[derive(Component, Deref, DerefMut)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TnuaGhostOverwrites<S: TnuaScheme>(pub <<<S as TnuaScheme>::Basis as TnuaBasis>::Sensors<'static> as TnuaSensors<'static>>::GhostOverwrites);

impl<S: TnuaScheme> AsMut<
<<<S as TnuaScheme>::Basis as TnuaBasis>::Sensors<'static> as TnuaSensors<'static>>::GhostOverwrites
> for TnuaGhostOverwrites<S> {
    fn as_mut(&mut self) -> &mut <<<S as TnuaScheme>::Basis as TnuaBasis>::Sensors<'static> as TnuaSensors<'static>>::GhostOverwrites {
        &mut self.0
    }
}

impl<S: TnuaScheme> Default for TnuaGhostOverwrites<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Controls how Tnua uses the ghost sensor of a single
/// [`TnuaProximitySensor`](crate::TnuaProximitySensor).
///
/// Note that this is not a component because it is not stored on the sensor entity - instead it is
/// stored with the entity that has the [`TnuaController`](crate::TnuaController) component, inside
/// a [`TnuaGhostOverwrites`] component. To [`TnuaGhostSensor`](crate::TnuaGhostSensor) itself is
/// stored on the sensor entity - to access it use the
/// [`sensors_entities`](crate::TnuaController::sensors_entities) field of the controller.
#[derive(Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TnuaGhostOverwrite(Option<Entity>);

impl TnuaGhostOverwrite {
    /// Set an output of the ghost sensor so that the controller will use it instead of the
    /// (non-ghost) output of the proximity sensor.
    ///
    /// Note that the controller does not use that output has is - it picks an output from the
    /// ghost sensor that matches the output provided here. This is important because the ghost
    /// sensor will have its list of outputs refrehsed before this happens.
    ///
    /// The ghost overwrite will remain in place until one of the following happens:
    /// 1. This method is invoked again.
    /// 2. The [`clear`](Self::clear) method is invoked.
    /// 3. The ghost sensor no longer has a matching hit in its outputs list.
    pub fn set(&mut self, sensor_output: &TnuaProximitySensorOutput) {
        self.0 = Some(sensor_output.entity);
    }

    /// Clear the ghost overwrite, so that the controller will use the outpuit from the proximity
    /// sensor instead of one from the ghost sensor.
    ///
    /// It's preferable to call this before iterating over the ghost outputs in the user control
    /// system, so that if no output is chosen one from the previous frame will not linger.
    pub fn clear(&mut self) {
        self.0 = None;
    }

    pub(crate) fn find_in<'a>(
        &self,
        ghost_outputs: &'a [TnuaProximitySensorOutput],
    ) -> Option<&'a TnuaProximitySensorOutput> {
        let entity = self.0?;
        ghost_outputs.iter().find(|output| output.entity == entity)
    }
}
