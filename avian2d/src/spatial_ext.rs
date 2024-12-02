use avian2d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_tnua_physics_integration_layer::{
    math::{Float, Vector3},
    spatial_ext::TnuaSpatialExt,
};

#[derive(SystemParam)]
pub struct TnuaSpatialExtAvian2d<'w, 's> {
    colliders_query: Query<'w, 's, (&'static Collider, &'static Position, &'static Rotation)>,
    collision_configuration_query: Query<'w, 's, (Option<&'static CollisionLayers>, Has<Sensor>)>,
}

impl TnuaSpatialExt for TnuaSpatialExtAvian2d<'_, '_> {
    type ColliderData<'a>
        = (&'a Collider, &'a Position, &'a Rotation)
    where
        Self: 'a;

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
            collider.project_point(**position, **rotation, point.truncate(), true);
        projected_point.extend(point.z)
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
            .cast_ray(
                **position,
                **rotation,
                origin.truncate(),
                direction.truncate(),
                max_time_of_impact,
                true,
            )
            .map(|(time_of_impact, normal)| (time_of_impact, normal.extend(0.0)))
    }

    fn can_interact(&self, entity1: Entity, entity2: Entity) -> bool {
        let Ok([(layers1, is_1_sensor), (layers2, is_2_sensor)]) = self
            .collision_configuration_query
            .get_many([entity1, entity2])
        else {
            return false;
        };
        if is_1_sensor || is_2_sensor {
            return false;
        }
        layers1
            .copied()
            .unwrap_or_default()
            .interacts_with(layers2.copied().unwrap_or_default())
    }
}
