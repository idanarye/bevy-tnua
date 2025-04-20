use std::cell::OnceCell;

use crate::math::{AdjustPrecision, AsF32, Float, Vector3};
use bevy::{math::InvalidDirectionError, prelude::*};
use bevy_tnua_physics_integration_layer::{
    obstacle_radar::TnuaObstacleRadar,
    spatial_ext::{TnuaPointProjectionResult, TnuaSpatialExt},
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
    pub collider_data: X::ColliderData<'a>,
    closest_point_cache: OnceCell<TnuaPointProjectionResult>,
    closest_point_normal_cache: OnceCell<Vector3>,
}

impl<X: TnuaSpatialExt> TnuaRadarBlipLens<'_, X> {
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

    pub fn closest_point(&self) -> TnuaPointProjectionResult {
        *self.closest_point_cache.get_or_init(|| {
            self.radar_lens.ext.project_point(
                self.radar().tracked_position(),
                false,
                &self.collider_data,
            )
        })
    }

    pub fn closest_point_from(&self, point: Vector3, solid: bool) -> TnuaPointProjectionResult {
        self.radar_lens
            .ext
            .project_point(point, solid, &self.collider_data)
    }

    pub fn closest_point_from_offset(
        &self,
        offset: Vector3,
        solid: bool,
    ) -> TnuaPointProjectionResult {
        self.closest_point_from(self.radar().tracked_position() + offset, solid)
    }

    pub fn flat_wall_score(&self, up: Dir3, offsets: &[Float]) -> Float {
        let Some(closest_point) = self.closest_point().outside() else {
            return 0.0;
        };
        1.0 - offsets
            .iter()
            .map(|offset| {
                if *offset == 0.0 {
                    return 0.0;
                }
                let offset_vec = *offset * up.adjust_precision();
                let expected = closest_point + offset_vec;
                let actual = self.closest_point_from_offset(offset_vec, false).get();
                let dist = expected.distance_squared(actual);
                dist / offset.powi(2)
            })
            .sum::<Float>()
            / offsets.len() as Float
    }

    /// Try traversing the geometry from the [`closest_point`](Self::closest_point) along
    /// `direction` until reaching `probe_at_distance`.
    ///
    /// If the geometry reaches that distance (and behind), that distance will be returned.
    ///
    /// If the geometry does not reach the desired distance, and it ends in a right angle or acute
    /// angle, the distance to that point will be returned.
    ///
    /// If the geometry does not reach the desired distance, and it "ends" in an obstute angle, the
    /// returned value will be between that point and `probe_at_distance`.
    ///
    /// This is useful to detect when the character is near the top of a wall or of a climbable
    /// object.
    pub fn probe_extent_from_closest_point(
        &self,
        direction: Dir3,
        probe_at_distance: Float,
    ) -> Float {
        let closest_point = self.closest_point().get();
        let closest_above = self
            .closest_point_from_offset(probe_at_distance * direction.adjust_precision(), false)
            .get();
        (closest_above - closest_point).dot(direction.adjust_precision())
    }

    pub fn direction_to_closest_point(&self) -> Result<Dir3, InvalidDirectionError> {
        match self.closest_point() {
            TnuaPointProjectionResult::Outside(closest_point) => {
                Dir3::new((closest_point - self.radar().tracked_position()).f32())
            }
            TnuaPointProjectionResult::Inside(closest_point) => {
                Dir3::new((self.radar().tracked_position() - closest_point).f32())
            }
        }
    }

    pub fn normal_from_closest_point(&self) -> Vector3 {
        *self.closest_point_normal_cache.get_or_init(|| {
            let origin = self.radar().tracked_position();

            let get_normal = |closest_point: Vector3| -> Vector3 {
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
            };

            match self.closest_point() {
                TnuaPointProjectionResult::Outside(closest_point) => get_normal(closest_point),
                TnuaPointProjectionResult::Inside(closest_point) => -get_normal(closest_point),
            }
        })
    }

    pub fn spatial_relation(&self, threshold: Float) -> TnuaBlipSpatialRelation {
        let Ok(direction) = self.direction_to_closest_point() else {
            return TnuaBlipSpatialRelation::Invalid;
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
    Invalid,
    Above,
    Below,
    Aeside(Dir3),
}
