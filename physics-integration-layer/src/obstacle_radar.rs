use bevy::{platform::collections::HashMap, prelude::*};

use crate::math::{Float, Vector3};

/// Add this to a character entity to detect obstacles around it.
///
/// Obstacles can be used for environment movement actions like climbing and wall-jumping.
///
/// This component stores rather limited data - only the detected entities and nothing about their
/// form or position. See `TnuaRadarLens` in the main Tnua crate, which wraps this with a
/// [`TnuaSpatialExt`](crate::spatial_ext::TnuaSpatialExt) to provide many helper methods for
/// running more queries on the detected obstacles.
#[derive(Component)]
pub struct TnuaObstacleRadar {
    /// The radius of the radar's cylinder.
    pub radius: Float,
    /// The height of the radar's cylinder.
    pub height: Float,
    tracked_entity: Entity,
    tracked_position: Vector3,
    up_direction: Dir3,
    blips: HashMap<Entity, BlipStatus>,
}

impl TnuaObstacleRadar {
    pub fn new(radius: Float, height: Float) -> Self {
        Self {
            radius,
            height,
            tracked_entity: Entity::PLACEHOLDER,
            tracked_position: Vector3::NAN,
            up_direction: Dir3::Y,
            blips: Default::default(),
        }
    }

    /// Physics integration crates must call this each frame before they start calling
    /// [`mark_seen`](Self::mark_seen), and feed it some general information about the character
    /// entity.
    pub fn pre_marking_update(
        &mut self,
        tracked_entity: Entity,
        tracked_position: Vector3,
        up_direction: Dir3,
    ) {
        self.tracked_entity = tracked_entity;
        self.tracked_position = tracked_position;
        self.up_direction = up_direction;
        self.blips.retain(|_, blip_status| match blip_status {
            BlipStatus::Unseen => false,
            BlipStatus::Seen => {
                *blip_status = BlipStatus::Unseen;
                true
            }
        });
    }

    /// Physics integration crates should call this for each detected entity during each frame,
    /// after invoking [`pre_marking_update`](Self::pre_marking_update).
    pub fn mark_seen(&mut self, entity: Entity) {
        self.blips.insert(entity, BlipStatus::Seen);
    }

    /// Get the character entity who owns the radar (not the detected obstacle entities!)
    pub fn tracked_entity(&self) -> Entity {
        self.tracked_entity
    }

    /// Get the position of the character who owns the radar (not the detected obstacle's
    /// positions!)
    pub fn tracked_position(&self) -> Vector3 {
        self.tracked_position
    }

    /// Get the direction considered up.
    pub fn up_direction(&self) -> Dir3 {
        self.up_direction
    }

    /// Iterate over all the blip entities.
    pub fn iter_blips(&self) -> impl '_ + Iterator<Item = Entity> {
        self.blips.keys().copied()
    }

    /// Check if a particular entity has been detected this frame.
    pub fn has_blip(&self, entity: Entity) -> bool {
        self.blips.contains_key(&entity)
    }
}

pub enum BlipStatus {
    Unseen,
    Seen,
}
