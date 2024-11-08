use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector3;

use super::{helper::LevelSetupHelper3d, PositionPlayer};

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 10.0, 0.0)));

    helper.spawn_floor(css::WHITE);

    let mut obstacles_helper = helper.with_color(css::GRAY);
    obstacles_helper.spawn_cuboid(
        "High wall",
        Transform::from_xyz(-3.0, 8.0, 0.0),
        Vector3::new(2.0, 16.0, 4.0),
    );

    obstacles_helper.spawn_cuboid(
        "Low wall",
        Transform::from_xyz(5.0, 3.5, 0.0),
        Vector3::new(4.0, 7.0, 4.0),
    );

    obstacles_helper.spawn_cuboid(
        "Floating Floor",
        Transform::from_xyz(10.0, 9.0, 0.0),
        Vector3::new(4.0, 0.5, 4.0),
    );
}