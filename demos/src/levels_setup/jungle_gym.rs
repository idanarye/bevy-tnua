use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector3;

use crate::level_mechanics::Climbable;
use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper3d::{a_cuboid, a_cylinder, make_sensor, the_floor};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 10.0, 0.0))
        --
        @the_floor(css::WHITE)
        LevelObject
        --
        #High_wall
        LevelObject
        @a_cuboid(css::GRAY, Vector3::new(2.0, 16.0, 4.0))
        Transform::from_xyz(-3.0, 8.0, 0.0)
        --
        #Low_wall
        LevelObject
        @a_cuboid(css::GRAY, Vector3::new(4.0, 7.0, 4.0))
        Transform::from_xyz(5.0, 3.5, 0.0)
        --
        #Floating_Floor
        LevelObject
        @a_cuboid(css::GRAY, Vector3::new(4.0, 0.5, 4.0))
        Transform::from_xyz(10.0, 9.0, 0.0)
        --
        #Vine
        LevelObject
        Transform::from_xyz(5.0, 1.0, 5.0)
        @a_cylinder(css::PALE_GREEN, 0.1, 10.0)
        @make_sensor()
        Climbable
        --
        #Higher_Vine
        LevelObject
        Transform::from_xyz(-8.0, 10.0, 0.0)
        @a_cylinder(css::PALE_GREEN, 0.1, 5.0)
        @make_sensor()
        Climbable
    });
}
