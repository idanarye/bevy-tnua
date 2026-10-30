use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::{Quaternion, Vector3};

use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper3d::{a_cuboid, some_compound_cuboids};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 2.0, 0.0))
        --
        #Area
        LevelObject
        Transform
        @some_compound_cuboids(css::BROWN, [
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
        ])
        --
        #Bug_Replicator
        LevelObject
        Transform::from_xyz(-10.0, 1.0, 0.0)
        @a_cuboid(css::LIGHT_CORAL, Vector3::new(4.0, 2.5, 4.0))
    });
}
