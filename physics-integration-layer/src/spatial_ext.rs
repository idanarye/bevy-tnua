use bevy::prelude::*;

use crate::math::{Float, Vector3};

pub trait TnuaSpatialExt {
    type ColliderData<'a>
    where
        Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>>;

    fn project_point(&self, point: Vector3, collider_data: &Self::ColliderData<'_>) -> Vector3;
    fn cast_ray(
        &self,
        origin: Vector3,
        direction: Vector3,
        max_time_of_impact: Float,
        collider_data: &Self::ColliderData<'_>,
    ) -> Option<(Float, Vector3)>;
}
