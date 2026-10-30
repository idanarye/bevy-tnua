use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::TnuaGhostPlatform;
use bevy_tnua::math::Vector3;

use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper3d::{a_cuboid, make_intangible};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 2.0, 0.0))
        --
        {
            (0..4).into_iter().map(|i| bsn! {
                Name(format!("Floor_{i}"))
                LevelObject
                @a_cuboid(css::BROWN, Vector3::new(8.0, 0.1, 2.0))
                Transform::from_xyz(0.0, i as f32 * 0.4, 0.0)
            }).collect::<Vec<_>>()
        }
        --
        {
            (0..4).into_iter().map(|i| bsn! {
                Name(format!("Ghost_{i}"))
                LevelObject
                @a_cuboid(css::ORANGE, Vector3::new(8.0, 0.1, 2.0))
                Transform::from_xyz(6.0, 0.2 + i as f32 * 0.4, 0.0)
                @make_intangible()
                TnuaGhostPlatform
            }).collect::<Vec<_>>()
        }
    });
}
