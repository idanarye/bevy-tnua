use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector2;

use crate::level_mechanics::Climbable;

use super::helper::LevelSetupHelper2dEntityCommandsExtension;
use super::{helper::LevelSetupHelper2d, PositionPlayer};

pub fn setup_level(mut helper: LevelSetupHelper2d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 2.0, 0.0)));

    helper.spawn_floor(css::GRAY);

    helper.spawn_rectangle(
        "High wall",
        css::GRAY,
        Transform::from_xyz(-3.0, 12.0, 0.0),
        Vector2::new(2.0, 16.0),
    );

    helper.spawn_rectangle(
        "Low wall",
        css::GRAY,
        Transform::from_xyz(5.0, 3.5, 0.0),
        Vector2::new(4.0, 7.0),
    );

    helper.spawn_rectangle(
        "Floating Floor",
        css::GRAY,
        Transform::from_xyz(10.0, 9.0, 0.0),
        Vector2::new(4.0, 0.5),
    );

    helper
        .spawn_rectangle(
            "Vine",
            css::PALE_GREEN,
            Transform::from_xyz(-8.0, 10.0, 0.0),
            Vector2::new(0.1, 20.0),
        )
        .make_sensor()
        .insert(Climbable);
}
