use std::cell::OnceCell;

use crate::math::{AdjustPrecision, AsF32, Float, Vector3};
use bevy::{math::InvalidDirectionError, prelude::*};
use bevy_tnua_physics_integration_layer::{
    obstacle_radar::TnuaObstacleRadar, spatial_ext::TnuaSpatialExt,
};

pub struct TnuaRadarLens<'a, X: TnuaSpatialExt> {
    radar: &'a TnuaObstacleRadar,
    ext: &'a X,
}

impl<'a, X: TnuaSpatialExt> TnuaRadarLens<'a, X> {
    pub fn new(radar: &'a TnuaObstacleRadar, ext: &'a X) -> Self {
        Self { radar, ext }
    }

    pub fn iter_blips(&self) -> impl Iterator<Item = TnuaRadarBlipLens<X>> {
        self.radar.iter_blips().filter_map(|entity| {
            Some(TnuaRadarBlipLens {
                radar_lens: self,
                entity,
                collider_data: self.ext.fetch_collider_data(entity)?,
                closest_point_cache: OnceCell::new(),
            })
        })
    }
}

pub struct TnuaRadarBlipLens<'a, X: TnuaSpatialExt> {
    radar_lens: &'a TnuaRadarLens<'a, X>,
    entity: Entity,
    collider_data: X::ColliderData<'a>,
    closest_point_cache: OnceCell<Vector3>,
}

impl<'a, X: TnuaSpatialExt> TnuaRadarBlipLens<'a, X> {
    fn radar(&self) -> &TnuaObstacleRadar {
        self.radar_lens.radar
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn closest_point(&self) -> Vector3 {
        *self.closest_point_cache.get_or_init(|| {
            self.radar_lens
                .ext
                .project_point(self.radar().tracked_position(), &self.collider_data)
        })
    }

    pub fn vector_to_closest_point(&self) -> Vector3 {
        self.closest_point() - self.radar().tracked_position()
    }

    pub fn direction_to_closest_point(&self) -> Result<Dir3, InvalidDirectionError> {
        Dir3::new(self.vector_to_closest_point().f32())
    }

    pub fn spatial_relation(&self, threshold: Float) -> TnuaBlipSpatialRelation {
        let Ok(direction) = self.direction_to_closest_point() else {
            return TnuaBlipSpatialRelation::Clipping;
        };
        let dot_up = self
            .radar()
            .up_direction()
            .dot(*direction)
            .adjust_precision();
        if threshold < dot_up {
            TnuaBlipSpatialRelation::Above
        } else if dot_up < -threshold {
            TnuaBlipSpatialRelation::Below
        } else {
            let planar_direction =
                Dir3::new(direction.reject_from_normalized(*self.radar().up_direction()))
                    .expect("since the dot-up is smaller than the threshold, the direction should not be parallel with the direction");
            TnuaBlipSpatialRelation::Aeside(planar_direction)
        }
    }
}

#[derive(Debug)]
pub enum TnuaBlipSpatialRelation {
    Clipping,
    Above,
    Below,
    Aeside(Dir3),
}
