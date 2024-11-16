use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_tnua_physics_integration_layer::{
    math::{Float, Vector3},
    spatial_ext::TnuaSpatialExt,
};

#[derive(SystemParam)]
pub struct TnuaSpatialExtAvian3d<'w, 's> {
    colliders_query: Query<'w, 's, (&'static Collider, &'static Position, &'static Rotation)>,
    collision_layers_query: Query<'w, 's, &'static CollisionLayers>,
}

impl TnuaSpatialExt for TnuaSpatialExtAvian3d<'_, '_> {
    type ColliderData<'a> = (&'a Collider, &'a Position, &'a Rotation) where Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>> {
        self.colliders_query.get(entity).ok()
    }

    fn project_point<'a>(
        &'a self,
        point: Vector3,
        collider_data: &Self::ColliderData<'a>,
    ) -> Vector3 {
        let (collider, position, rotation) = collider_data;
        let (projected_point, _is_inside) =
            collider.project_point(**position, **rotation, point, true);
        projected_point
    }

    fn cast_ray<'a>(
        &'a self,
        origin: Vector3,
        direction: Vector3,
        max_time_of_impact: Float,
        collider_data: &Self::ColliderData<'a>,
    ) -> Option<(Float, Vector3)> {
        let (collider, position, rotation) = collider_data;
        collider.cast_ray(
            **position,
            **rotation,
            origin,
            direction,
            max_time_of_impact,
            true,
        )
    }

    fn can_interact(&self, entity1: Entity, entity2: Entity) -> bool {
        let Ok([layers1, layers2]) = self.collision_layers_query.get_many([entity1, entity2])
        else {
            return true;
        };
        layers1.interacts_with(*layers2)
    }
}
