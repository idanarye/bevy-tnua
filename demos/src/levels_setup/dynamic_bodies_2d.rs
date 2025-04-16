use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_tnua::math::Vector2;
use bevy_tnua::TnuaNotPlatform;

use super::helper::LevelSetupHelper2d;
use super::PositionPlayer;

pub fn setup_level(mut helper: LevelSetupHelper2d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 10.0, 0.0)));

    helper.spawn_floor(css::WHITE);

    helper
        .spawn_dynamic_rectangle(
            "Nonplatform Box",
            css::ORANGE_RED,
            Transform::from_xyz(4.0, 4.0, 0.0),
            Vector2::new(0.9, 0.9),
        )
        .insert(TnuaNotPlatform);

    helper.spawn_dynamic_rectangle(
        "Platform Box",
        css::GREEN_YELLOW,
        Transform::from_xyz(-4.0, 4.0, 0.0),
        Vector2::new(0.9, 0.9),
    );
}
