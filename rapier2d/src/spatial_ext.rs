use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_rapier2d::prelude::*;
use bevy_tnua_physics_integration_layer::{
    math::{Float, Vector3},
    spatial_ext::TnuaSpatialExt,
};

use crate::get_collider;

#[derive(SystemParam)]
pub struct TnuaSpatialExtRapier2d<'w, 's> {
    rapier_context: Res<'w, RapierContext>,
    colliders_query: Query<'w, 's, (&'static Collider, &'static GlobalTransform)>,
}

impl TnuaSpatialExt for TnuaSpatialExtRapier2d<'_, '_> {
    type ColliderData<'a> = (&'a Collider, Vec2, f32) where Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>> {
        let (collider, transform) = self.colliders_query.get(entity).ok()?;
        let (_scale, rotation, translation) = transform.to_scale_rotation_translation();
        Some((
            collider,
            translation.truncate(),
            rotation.to_euler(EulerRot::ZYX).0,
        ))
    }

    fn project_point<'a>(
        &'a self,
        point: Vector3,
        collider_data: &Self::ColliderData<'a>,
    ) -> Vector3 {
        let (collider, position, rotation) = collider_data;
        collider
            .project_point(*position, *rotation, point.truncate(), true)
            .point
            .extend(point.z)
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
                origin.truncate(),
                direction.truncate(),
                max_time_of_impact,
                true,
            )
            .map(|res| (res.time_of_impact, res.normal.extend(0.0)))
    }

    fn can_interact(&self, entity1: Entity, entity2: Entity) -> bool {
        let Some(collider1) = get_collider(&self.rapier_context, entity1) else {
            return true;
        };
        let Some(collider2) = get_collider(&self.rapier_context, entity2) else {
            return true;
        };
        collider1
            .collision_groups()
            .test(collider2.collision_groups())
            && collider1.solver_groups().test(collider2.solver_groups())
    }
}
