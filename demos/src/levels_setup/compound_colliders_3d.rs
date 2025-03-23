use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::{Quaternion, Vector3};

use super::helper::LevelSetupHelper3d;
use super::PositionPlayer;

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 2.0, 0.0)));

    helper.with_color(css::BROWN).spawn_compound_cuboids(
        "Area",
        Transform::default(),
        &[
            (
                Vector3::new(0.0, 0.0, 0.0),
                Quaternion::IDENTITY,
                Vector3::new(20.0, 1.0, 4.0),
            ),
            (
                Vector3::new(10.0, 5.0, 0.0),
                Quaternion::IDENTITY,
                Vector3::new(1.0, 10.0, 4.0),
            ),
        ],
    );

    helper.with_color(css::LIGHT_CORAL).spawn_cuboid(
        "Bug Replicator",
        Transform::from_xyz(-10.0, 1.0, 0.0),
        Vector3::new(4.0, 2.5, 4.0),
    );
}
