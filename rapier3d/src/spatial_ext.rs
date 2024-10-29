use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_tnua_physics_integration_layer::{
    math::{Float, Vector3},
    spatial_ext::TnuaSpatialExt,
};

#[derive(SystemParam)]
pub struct TnuaSpatialExtRapier3d<'w, 's> {
    colliders_query: Query<'w, 's, (&'static Collider, &'static GlobalTransform)>,
}

impl TnuaSpatialExt for TnuaSpatialExtRapier3d<'_, '_> {
    type ColliderData<'a> = (&'a Collider, Vec3, Quat) where Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>> {
        let (collider, transform) = self.colliders_query.get(entity).ok()?;
        let (_scale, rotation, translation) = transform.to_scale_rotation_translation();
        Some((collider, translation, rotation))
    }

    fn project_point<'a>(
        &'a self,
        point: Vector3,
        collider_data: &Self::ColliderData<'a>,
    ) -> Vector3 {
        let (collider, position, rotation) = collider_data;
        collider
            .project_point(*position, *rotation, point, true)
            .point
    }

    fn cast_ray<'a>(
        &'a self,
        origin: Vector3,
        direction: Vector3,
        max_time_of_impact: Float,
        collider_data: &Self::ColliderData<'a>,
    ) -> Option<(Float, Vector3)> {
        let (collider, position, rotation) = collider_data;
        collider
            .cast_ray_and_get_normal(
                *position,
                *rotation,
                origin,
                direction,
                max_time_of_impact,
                true,
            )
            .map(|res| (res.time_of_impact, res.normal))
    }
}
