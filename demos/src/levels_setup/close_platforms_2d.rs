use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::TnuaGhostPlatform;
use bevy_tnua::math::Vector2;

use super::helper::helper2d::{a_rectangle, make_intangible};
use super::{LevelObject, PositionPlayer};

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        PositionPlayer(Vec3::new(0.0, 2.0, 0.0))
    });

    for i in 0..4 {
        commands.spawn_scene(bsn! {
            Name(format!("Floor_{i}"))
            LevelObject
            @a_rectangle(css::BROWN, Vector2::new(8.0, 0.1))
            Transform::from_xyz(0.0, i as f32 * 0.4, 0.0)
        });
    }

    for i in 0..4 {
        commands.spawn_scene(bsn! {
            Name(format!("Ghost_{i}"))
            LevelObject
            @a_rectangle(css::ORANGE, Vector2::new(8.0, 0.1))
            Transform::from_xyz(6.0, 0.2 + i as f32 * 0.4, 0.0)
            @make_intangible()
            TnuaGhostPlatform
        });
    }
}
