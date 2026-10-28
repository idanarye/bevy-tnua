use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector2;

use crate::level_mechanics::Climbable;
use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper2d::{a_rectangle, make_sensor, the_floor};

use super::PositionPlayer;

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 2.0, 0.0))
        --
        @the_floor(css::GRAY)
        LevelObject
        --
        #High_wall
        LevelObject
        Transform::from_xyz(-3.0, 12.0, 0.0)
        @a_rectangle(css::GRAY, Vector2::new(2.0, 16.0))
        --
        #Low_wall
        Transform::from_xyz(5.0, 3.5, 0.0)
        @a_rectangle(css::GRAY, Vector2::new(4.0, 7.0))
        --
        #Floating_Floor
        Transform::from_xyz(10.0, 9.0, 0.0)
        @a_rectangle(css::GRAY, Vector2::new(4.0, 0.5))
        --
        #Vine
        Transform::from_xyz(-8.0, 10.0, 0.0)
        @a_rectangle(css::PALE_GREEN, Vector2::new(0.1, 20.0))
        @make_sensor()
        Climbable
    });
}
