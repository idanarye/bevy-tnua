use crate::math::Vector3;
use bevy::prelude::*;
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
            })
        })
    }
}

pub struct TnuaRadarBlipLens<'a, X: TnuaSpatialExt> {
    radar_lens: &'a TnuaRadarLens<'a, X>,
    entity: Entity,
    collider_data: X::ColliderData<'a>,
}

impl<'a, X: TnuaSpatialExt> TnuaRadarBlipLens<'a, X> {
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn closest_point(&self) -> Vector3 {
        self.radar_lens.ext.project_point(
            self.radar_lens.radar.tracked_position(),
            &self.collider_data,
        )
    }
}
