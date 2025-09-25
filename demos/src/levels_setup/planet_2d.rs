use bevy::prelude::*;

use crate::level_mechanics::IsCenterOfGraity;

use super::helper::LevelSetupHelper2d;
use super::PositionPlayer;

pub fn setup_level(mut helper: LevelSetupHelper2d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 25.0, 0.0)));

    helper
        .spawn_circle("Planet", Transform::from_xyz(0.0, 5.0, 0.0), 15.0)
        .insert(IsCenterOfGraity);
}
