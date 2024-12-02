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
                closest_point_normal_cache: OnceCell::new(),
            })
        })
    }
}

pub struct TnuaRadarBlipLens<'a, X: TnuaSpatialExt> {
    radar_lens: &'a TnuaRadarLens<'a, X>,
    entity: Entity,
    collider_data: X::ColliderData<'a>,
    closest_point_cache: OnceCell<Vector3>,
    closest_point_normal_cache: OnceCell<Vector3>,
}

impl<'a, X: TnuaSpatialExt> TnuaRadarBlipLens<'a, X> {
    fn radar(&self) -> &TnuaObstacleRadar {
        self.radar_lens.radar
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn is_interactable(&self) -> bool {
        self.radar_lens
            .ext
            .can_interact(self.radar().tracked_entity(), self.entity)
    }

    pub fn closest_point(&self) -> Vector3 {
        *self.closest_point_cache.get_or_init(|| {
            self.radar_lens
                .ext
                .project_point(self.radar().tracked_position(), &self.collider_data)
        })
    }

    pub fn closest_point_from(&self, point: Vector3) -> Vector3 {
        self.radar_lens
            .ext
            .project_point(point, &self.collider_data)
    }

    pub fn closest_point_from_offset(&self, offset: Vector3) -> Vector3 {
        self.closest_point_from(self.radar().tracked_position() + offset)
    }

    pub fn flat_wall_score(&self, up: Dir3, offsets: &[Float]) -> Float {
        let closest_point = self.closest_point();
        1.0 - offsets
            .iter()
            .map(|offset| {
                if *offset == 0.0 {
                    return 0.0;
                }
                let offset_vec = *offset * up.adjust_precision();
                let expected = closest_point + offset_vec;
                let actual = self.closest_point_from_offset(offset_vec);
                let dist = expected.distance_squared(actual);
                dist / offset.powi(2)
            })
            .sum::<Float>()
            / offsets.len() as Float
    }

    pub fn vector_to_closest_point(&self) -> Vector3 {
        self.closest_point() - self.radar().tracked_position()
    }

    pub fn direction_to_closest_point(&self) -> Result<Dir3, InvalidDirectionError> {
        Dir3::new(self.vector_to_closest_point().f32())
    }

    pub fn normal_from_closest_point(&self) -> Vector3 {
        *self.closest_point_normal_cache.get_or_init(|| {
            let closest_point = self.closest_point();
            let origin = self.radar().tracked_position();
            let Some(direction) = (closest_point - origin).try_normalize() else {
                return Vector3::ZERO;
            };
            let Some((_, normal)) = self.radar_lens.ext.cast_ray(
                origin,
                direction,
                Float::INFINITY,
                &self.collider_data,
            ) else {
                warn!("Unable to query normal to already-found closest point");
                return Vector3::ZERO;
            };
            normal
        })
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
