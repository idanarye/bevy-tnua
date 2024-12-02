use bevy::prelude::*;

use crate::math::{Float, Vector3};

/// Structured spatial queries.
///
/// Physics integration crates should define a [`SystemParam`](bevy::ecs::system::SystemParam) type
/// and implement this trait on it. The main Tnua crate (or third party crates) can define wrappers
/// (like `TnuaRadarLens`) that can utilize these queries to present helper methods for user code.
pub trait TnuaSpatialExt {
    /// The data required to answer queries on a collider.
    type ColliderData<'a>
    where
        Self: 'a;

    /// Get the [`ColliderData`](TnuaSpatialExt::ColliderData) from the sytem.
    ///
    /// Since the struct implementing `TnuaSpatialExt` is typically a `SystemParam`, it can use its
    /// fields to get that data from the ECS.
    fn fetch_collider_data(&self, entity: Entity) -> Option<Self::ColliderData<'_>>;

    /// Return the point on the collider that's closest to some external point.
    fn project_point(&self, point: Vector3, collider_data: &Self::ColliderData<'_>) -> Vector3;

    /// Cast a ray on the collider, returning the time-of-impact and the normal.
    fn cast_ray(
        &self,
        origin: Vector3,
        direction: Vector3,
        max_time_of_impact: Float,
        collider_data: &Self::ColliderData<'_>,
    ) -> Option<(Float, Vector3)>;

    /// Check if the physics engine is solving interaction between the two entities.
    ///
    /// If the physics engine is detecting the collision but does not apply forces according to it,
    /// this method should return `false`.
    fn can_interact(&self, entity1: Entity, entity2: Entity) -> bool;
}
