use bevy::prelude::*;

use crate::math::Vector3;

pub trait TnuaSpatialExt {
    type ColliderData<'a>
    where
        Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>>;

    fn project_point(&self, point: Vector3, collider_data: &Self::ColliderData<'_>) -> Vector3;
}
