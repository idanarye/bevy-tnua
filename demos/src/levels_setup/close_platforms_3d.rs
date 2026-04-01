use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::TnuaGhostPlatform;
use bevy_tnua::math::{Float, Vector3};

#[cfg(feature = "avian3d")]
use avian3d::prelude::*;
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::prelude::*;

use super::PositionPlayer;
use super::helper::LevelSetupHelper3d;

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 2.0, 0.0)));

    let mut platforms_helper = helper.with_color(css::BROWN);
    for i in 0..4 {
        platforms_helper.spawn_cuboid(
            format!("Floor {i}"),
            Transform::from_xyz(0.0, i as Float * 0.4, 0.0),
            Vector3::new(8.0, 0.1, 2.0)
        );
    }

    let mut ghost_platforms_helper = helper.with_color(css::ORANGE);
    for i in 0..4 {
        ghost_platforms_helper.spawn_cuboid(
            format!("Ghost {i}"),
            Transform::from_xyz(6.0, 0.2 + i as Float * 0.4, 0.0),
            Vector3::new(8.0, 0.1, 2.0)
        ).insert((
            #[cfg(feature = "rapier3d")]
            SolverGroups {
                memberships: Group::empty(),
                filters: Group::empty(),
            },
            #[cfg(feature = "avian3d")]
            CollisionLayers::new([super::for_3d_platformer::LayerNames::FallThrough], [super::for_3d_platformer::LayerNames::FallThrough]),
            TnuaGhostPlatform,
        ));
    }
}
