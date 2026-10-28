use bevy::prelude::*;

use crate::level_mechanics::IsCenterOfGraity;
use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper2d::a_circle;

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 25.0, 0.0))
        --
        #Planet
        LevelObject
        Transform::from_xyz(0.0, 5.0, 0.0)
        @a_circle(15.0)
        IsCenterOfGraity
    });
}
