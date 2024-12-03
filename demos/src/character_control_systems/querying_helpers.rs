use bevy::ecs::query::QueryData;
use bevy::prelude::*;

use crate::level_mechanics::Climbable;

#[derive(QueryData)]
pub struct ObstacleQueryHelper {
    pub climbable: Has<Climbable>,
}
