use bevy::{prelude::*, utils::HashMap};

use crate::math::{Float, Vector3};

#[derive(Component)]
pub struct TnuaObstacleRadar {
    pub radius: Float,
    pub height: Float,
    tracked_position: Vector3,
    blips: HashMap<Entity, BlipStatus>,
}

impl TnuaObstacleRadar {
    pub fn new(radius: Float, height: Float) -> Self {
        Self {
            radius,
            height,
            tracked_position: Vector3::NAN,
            blips: Default::default(),
        }
    }

    pub fn pre_marking_update(&mut self, tracked_position: Vector3) {
        self.tracked_position = tracked_position;
        self.blips.retain(|_, blip_status| match blip_status {
            BlipStatus::Unseen => false,
            BlipStatus::Seen => {
                *blip_status = BlipStatus::Unseen;
                true
            }
        });
    }

    pub fn mark_seen(&mut self, entity: Entity) {
        self.blips.insert(entity, BlipStatus::Seen);
    }

    pub fn tracked_position(&self) -> Vector3 {
        self.tracked_position
    }

    pub fn up_direction(&self) -> Dir3 {
        Dir3::Y
    }

    pub fn iter_blips(&self) -> impl '_ + Iterator<Item = Entity> {
        self.blips.keys().copied()
    }

    pub fn has_blip(&self, entity: Entity) -> bool {
        self.blips.contains_key(&entity)
    }
}

pub enum BlipStatus {
    Unseen,
    Seen,
}
