use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector2;

use super::helper::LevelSetupHelper2d;
use super::PositionPlayer;

pub fn setup_level(mut helper: LevelSetupHelper2d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 2.0, 0.0)));

    helper.spawn_compound_rectangles(
        "Area",
        css::BROWN,
        Transform::default(),
        &[
            (Vector2::new(0.0, 0.0), 0.0, Vector2::new(20.0, 1.0)),
            (Vector2::new(10.0, 5.0), 0.0, Vector2::new(1.0, 10.0)),
        ],
    );

    helper.spawn_rectangle(
        "Bug Replicator",
        css::LIGHT_CORAL,
        Transform::from_xyz(-10.0, 1.0, 0.0),
        Vector2::new(4.0, 2.5),
    );
}
