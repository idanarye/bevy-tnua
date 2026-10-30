use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy_tnua::TnuaNotPlatform;

use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper3d::{a_ball, make_dynamic, the_floor};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 10.0, 0.0))
        --
        @the_floor(css::WHITE)
        LevelObject
        --
        #Nonplatform_Ball
        LevelObject
        Transform::from_xyz(4.0, 4.0, 0.0)
        @a_ball(css::ORANGE_RED, 0.45)
        @make_dynamic()
        TnuaNotPlatform
        --
        #Platform_Ball
        LevelObject
        Transform::from_xyz(-4.0, 4.0, 0.0)
        @a_ball(css::GREEN_YELLOW, 0.45)
        @make_dynamic()
    });
}
