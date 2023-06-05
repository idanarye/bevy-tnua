use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::{TnuaGhostSensor, TnuaProximitySensor};

#[derive(Component, Default)]
pub struct TnuaSimpleFallThroughPlatformsHelper {
    currently_falling_through: HashSet<Entity>,
}

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
        let mut already_falling_through_not_yet_seen =
            self.parent.currently_falling_through.clone();
        for ghost_platform in self.ghost_sensor.0.iter() {
            if self.min_proximity <= ghost_platform.proximity
                && !already_falling_through_not_yet_seen.remove(&ghost_platform.entity)
            {
                self.proximity_sensor.output = Some(ghost_platform.clone());
                break;
            }
        }
        self.parent
            .currently_falling_through
            .retain(|entity| !already_falling_through_not_yet_seen.contains(entity));
    }

    pub fn try_falling(&mut self) -> bool {
        self.parent.currently_falling_through.clear();
        for ghost_platform in self.ghost_sensor.0.iter() {
            if self.min_proximity <= ghost_platform.proximity {
                self.parent
                    .currently_falling_through
                    .insert(ghost_platform.entity);
            }
        }
        !self.parent.currently_falling_through.is_empty()
    }

    pub fn try_falling_one_step_at_a_time(&mut self, just_pressed: bool) -> bool {
        if !just_pressed && !self.parent.currently_falling_through.is_empty() {
            for ghost_platform in self.ghost_sensor.0.iter() {
                if self.min_proximity <= ghost_platform.proximity
                    && !self
                        .parent
                        .currently_falling_through
                        .contains(&ghost_platform.entity)
                {
                    self.proximity_sensor.output = Some(ghost_platform.clone());
                    return true;
                }
            }
            return true;
        }
        self.parent.currently_falling_through.clear();
        for ghost_platform in self.ghost_sensor.0.iter() {
            if self.min_proximity <= ghost_platform.proximity {
                self.parent
                    .currently_falling_through
                    .insert(ghost_platform.entity);
            }
        }
        !self.parent.currently_falling_through.is_empty()
    }
}
