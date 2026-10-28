use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector2;

use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper2d::{a_rectangle, some_compound_rectangles};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 2.0, 0.0))
        --
        #Area
        LevelObject
        Transform
        @some_compound_rectangles(css::BROWN, [
            (Vector2::new(0.0, 0.0), 0.0, Vector2::new(20.0, 1.0)),
            (Vector2::new(10.0, 5.0), 0.0, Vector2::new(1.0, 10.0)),
        ])
        --
        #Bug_Replicator
        LevelObject
        Transform::from_xyz(-10.0, 1.0, 0.0)
        @a_rectangle(css::LIGHT_CORAL, Vector2::new(4.0, 2.5))
    });
}
