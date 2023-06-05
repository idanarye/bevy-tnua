use bevy::prelude::*;

use crate::{TnuaGhostSensor, TnuaProximitySensor};

#[derive(Component, Default)]
pub struct TnuaSimpleFallThroughPlatformsHelper {}

impl TnuaSimpleFallThroughPlatformsHelper {
    pub fn with<'a>(
        &'a mut self,
        proximity_sensor: &'a mut TnuaProximitySensor,
        ghost_sensor: &'a TnuaGhostSensor,
        min_proximity: f32,
    ) -> TnuaSimpleFallThroughPlatformsHelperWithData<'a> {
        TnuaSimpleFallThroughPlatformsHelperWithData {
            parent: self,
            proximity_sensor,
            ghost_sensor,
            min_proximity,
        }
    }
}

#[allow(dead_code)]
pub struct TnuaSimpleFallThroughPlatformsHelperWithData<'a> {
    parent: &'a mut TnuaSimpleFallThroughPlatformsHelper,
    proximity_sensor: &'a mut TnuaProximitySensor,
    ghost_sensor: &'a TnuaGhostSensor,
    min_proximity: f32,
}

impl TnuaSimpleFallThroughPlatformsHelperWithData<'_> {
    pub fn dont_fall(&mut self) {
        if let Some(ghost_platform) = self.ghost_sensor.0.first() {
            if self.min_proximity <= ghost_platform.proximity {
                self.proximity_sensor.output = Some(ghost_platform.clone());
            }
        }
    }

    pub fn try_falling(&mut self) -> bool {
        if let Some(ghost_platform) = self.ghost_sensor.0.first() {
            if self.min_proximity <= ghost_platform.proximity {
                return true;
            }
        }
        false
    }
}
