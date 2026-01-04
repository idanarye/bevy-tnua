use crate::ghost_overrides::TnuaGhostOverwritesForBasis;
use crate::{TnuaBasis, TnuaScheme, math::*};
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::data_for_backends::{
    TnuaGhostSensor, TnuaProximitySensor, TnuaSensorOf,
};

/// The entities that hold the [proximity sensors](TnuaProximitySensor) of the basis.
///
/// Automatically added for [`TnuaController`](crate::TnuaController).
#[derive(Component, Deref)]
pub struct TnuaSensorsEntities<S: TnuaScheme> {
    pub(crate) sensors_entities:
        <<S::Basis as TnuaBasis>::Sensors<'static> as TnuaSensors<'static>>::Entities,
}

pub trait TnuaSensors<'a>: 'a + Copy + Clone {
    type Entities: 'static + Send + Sync + Default;
    #[cfg(feature = "serialize")]
    type GhostOverwrites: TnuaGhostOverwritesForBasis<Entities = Self::Entities>
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>;
    #[cfg(not(feature = "serialize"))]
    type GhostOverwrites: TnuaGhostOverwritesForBasis<Entities = Self::Entities>;
}

pub struct ProximitySensorPreparationHelper {
    pub cast_origin: Vector3,
    pub cast_direction: Dir3,
    pub cast_shape_rotation: Quaternion,
    pub cast_range: Float,
    pub ghost_sensor: bool,
}

impl Default for ProximitySensorPreparationHelper {
    fn default() -> Self {
        Self {
            cast_origin: Vector3::ZERO,
            cast_direction: Dir3::NEG_Y,
            cast_shape_rotation: Quaternion::IDENTITY,
            cast_range: 0.0,
            ghost_sensor: false,
        }
    }
}

impl ProximitySensorPreparationHelper {
    fn already_set_in_sensor(&self, sensor: &TnuaProximitySensor) -> bool {
        let Self {
            cast_origin,
            cast_direction,
            cast_shape_rotation,
            cast_range,
            ghost_sensor: _,
        } = self;
        *cast_origin == sensor.cast_origin
            && *cast_direction == sensor.cast_direction
            && *cast_shape_rotation == sensor.cast_shape_rotation
            && *cast_range == sensor.cast_range
    }

    fn to_sensor(&self) -> TnuaProximitySensor {
        TnuaProximitySensor {
            cast_origin: self.cast_origin,
            cast_direction: self.cast_direction,
            cast_shape_rotation: self.cast_shape_rotation,
            cast_range: self.cast_range,
            output: None,
        }
    }

    pub fn prepare_for<'a>(
        &self,
        put_in_entity: &mut Option<Entity>,
        proximity_sensors_query: &'a Query<(&TnuaProximitySensor, Has<TnuaGhostSensor>)>,
        controller_entity: Entity,
        commands: &mut Commands,
    ) -> Option<&'a TnuaProximitySensor> {
        if let Some(sensor_entity) = put_in_entity
            && let Ok((existing_sensor, has_ghost_sensor)) =
                proximity_sensors_query.get(*sensor_entity)
        {
            if !self.already_set_in_sensor(existing_sensor) {
                // TODO: send a command that only alters the sensor properties?
                commands.entity(*sensor_entity).insert(self.to_sensor());
            }
            if self.ghost_sensor && !has_ghost_sensor {
                commands
                    .entity(*sensor_entity)
                    .insert(TnuaGhostSensor::default());
            } else if !self.ghost_sensor && has_ghost_sensor {
                commands
                    .entity(*sensor_entity)
                    .try_remove::<TnuaGhostSensor>();
            }
            Some(existing_sensor)
        } else {
            commands
                .entity(controller_entity)
                .with_related_entities::<TnuaSensorOf>(|commands| {
                    let mut cmd = commands.spawn(self.to_sensor());
                    if self.ghost_sensor {
                        cmd.insert(TnuaGhostSensor::default());
                    }
                    *put_in_entity = Some(cmd.id());
                });
            None
        }
    }

    pub fn ensure_not_existing<'a>(
        put_in_entity: &mut Option<Entity>,
        proximity_sensors_query: &'a Query<(&TnuaProximitySensor, Has<TnuaGhostSensor>)>,
        commands: &mut Commands,
    ) -> Option<&'a TnuaProximitySensor> {
        if let Some(sensor_entity) = put_in_entity {
            if proximity_sensors_query.contains(*sensor_entity) {
                commands.entity(*sensor_entity).despawn();
            } else {
                *put_in_entity = None;
            }
        }
        None
    }
}
