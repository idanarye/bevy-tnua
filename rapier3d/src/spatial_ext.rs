use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_tnua_physics_integration_layer::{
    math::{Float, Vector3},
    spatial_ext::{TnuaPointProjectionResult, TnuaSpatialExt},
};

use crate::get_collider;

#[derive(SystemParam)]
pub struct TnuaSpatialExtRapier3d<'w, 's> {
    rapier_context: Res<'w, RapierContext>,
    colliders_query: Query<'w, 's, (&'static Collider, &'static GlobalTransform)>,
}

impl TnuaSpatialExt for TnuaSpatialExtRapier3d<'_, '_> {
    type ColliderData<'a>
        = (&'a Collider, Vec3, Quat)
    where
        Self: 'a;

    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>> {
        let (collider, transform) = self.colliders_query.get(entity).ok()?;
        let (_scale, rotation, translation) = transform.to_scale_rotation_translation();
        Some((collider, translation, rotation))
    }

    fn project_point<'a>(
        &'a self,
        point: Vector3,
        solid: bool,
        collider_data: &Self::ColliderData<'a>,
    ) -> TnuaPointProjectionResult {
        let (collider, position, rotation) = collider_data;
        let result = collider.project_point(*position, *rotation, point, solid);
        if result.is_inside {
            TnuaPointProjectionResult::Inside(result.point)
        } else {
            TnuaPointProjectionResult::Outside(result.point)
        }
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

    fn can_interact(&self, entity1: Entity, entity2: Entity) -> bool {
        let Some(collider1) = get_collider(&self.rapier_context, entity1) else {
            return true;
        };
        if collider1.is_sensor() {
            return false;
        }
        let Some(collider2) = get_collider(&self.rapier_context, entity2) else {
            return true;
        };
        if collider2.is_sensor() {
            return false;
        }
        collider1
            .collision_groups()
            .test(collider2.collision_groups())
            && collider1.solver_groups().test(collider2.solver_groups())
    }
}
