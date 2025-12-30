use bevy::color::palettes::css;
use bevy::prelude::*;

use crate::level_mechanics::IsCenterOfGraity;

use super::PositionPlayer;
use super::helper::LevelSetupHelper3d;

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 25.0, 0.0)));

    helper
        .with_color(css::DARK_GRAY.with_alpha(0.5))
        .spawn_ball("Planet", Transform::from_xyz(0.0, 10.0, 0.0), 10.0)
        .insert(IsCenterOfGraity);
}
