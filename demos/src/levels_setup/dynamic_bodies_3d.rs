use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_tnua::TnuaNotPlatform;

use super::helper::LevelSetupHelper3d;
use super::PositionPlayer;

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 10.0, 0.0)));

    helper.spawn_floor(css::WHITE);

    helper
        .with_color(css::ORANGE_RED)
        .spawn_dynamic_ball("Nonplatform Ball", Transform::from_xyz(4.0, 4.0, 0.0), 0.45)
        .insert(TnuaNotPlatform);

    helper.with_color(css::GREEN_YELLOW).spawn_dynamic_ball(
        "Platform Ball",
        Transform::from_xyz(-4.0, 4.0, 0.0),
        0.45,
    );
}
