use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::TnuaProximitySensorOutput;

use crate::sensor_sets::TnuaSensors;
use crate::{TnuaBasis, TnuaScheme};

pub trait TnuaGhostOverwritesForBasis: 'static + Send + Sync + Default {
    type Entities: 'static + Send + Sync + Default;
}

#[derive(Component, Deref, DerefMut)]
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

#[derive(Default)]
pub struct TnuaGhostOverwrite(Option<Entity>);

impl TnuaGhostOverwrite {
    pub fn set(&mut self, sensor_output: &TnuaProximitySensorOutput) {
        self.0 = Some(sensor_output.entity);
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }

    pub fn find_in<'a>(
        &self,
        ghost_outputs: &'a [TnuaProximitySensorOutput],
    ) -> Option<&'a TnuaProximitySensorOutput> {
        let entity = self.0?;
        ghost_outputs.iter().find(|output| output.entity == entity)
    }
}
