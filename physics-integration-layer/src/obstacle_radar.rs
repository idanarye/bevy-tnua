use bevy::{prelude::*, utils::HashMap};

use crate::math::{Float, Vector3};

#[derive(Component)]
pub struct TnuaObstacleRadar {
    pub radius: Float,
    pub height: Float,
    pub blips: HashMap<Entity, TnuaObstacleRadarBlip>,
}

impl TnuaObstacleRadar {
    pub fn new(radius: Float, height: Float) -> Self {
        Self {
            radius,
            height,
            blips: Default::default(),
        }
    }

    pub fn mark_unseen(&mut self) {
        for blip in self.blips.values_mut() {
            blip.seen = false;
        }
    }

    pub fn delete_unseen(&mut self) {
        self.blips.retain(|_, blip| blip.seen);
    }
}

pub struct TnuaObstacleRadarBlip {
    pub radar_position: Vector3,
    pub position: Vector3,
    pub to_top: Float,
    pub to_bottom: Float,
    pub seen: bool,
}
