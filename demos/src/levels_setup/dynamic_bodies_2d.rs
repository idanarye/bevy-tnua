use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_tnua::TnuaNotPlatform;
use bevy_tnua::math::Vector2;

use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper2d::{a_rectangle, make_dynamic, the_floor};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 10.0, 0.0))
        --
        @the_floor(css::WHITE)
        LevelObject
        --
        #Nonplatform_Box
        LevelObject
        Transform::from_xyz(4.0, 4.0, 0.0)
        @a_rectangle(css::ORANGE_RED, Vector2::new(0.9, 0.9))
        @make_dynamic()
        TnuaNotPlatform
        --
        #Platform_Box
        LevelObject
        Transform::from_xyz(-4.0, 4.0, 0.0)
        @a_rectangle(css::GREEN_YELLOW, Vector2::new(0.9, 0.9))
        @make_dynamic()
    });
}
