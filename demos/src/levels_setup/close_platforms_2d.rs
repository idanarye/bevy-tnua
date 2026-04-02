use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::TnuaGhostPlatform;
use bevy_tnua::math::Vector2;

#[cfg(feature = "avian2d")]
use avian2d::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::prelude::*;

use super::PositionPlayer;
use super::helper::LevelSetupHelper2d;

pub fn setup_level(mut helper: LevelSetupHelper2d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 2.0, 0.0)));

    for i in 0..4 {
        helper.spawn_rectangle(
            format!("Floor {i}"),
            css::BROWN,
            Transform::from_xyz(0.0, i as f32 * 0.4, 0.0),
            Vector2::new(8.0, 0.1),
        );
    }

    for i in 0..4 {
        helper
            .spawn_rectangle(
                format!("Ghost {i}"),
                css::ORANGE,
                Transform::from_xyz(6.0, 0.2 + i as f32 * 0.4, 0.0),
                Vector2::new(8.0, 0.1),
            )
            .insert((
                #[cfg(feature = "rapier2d")]
                SolverGroups {
                    memberships: Group::empty(),
                    filters: Group::empty(),
                },
                #[cfg(feature = "avian2d")]
                CollisionLayers::new(
                    [super::for_2d_platformer::LayerNames::FallThrough],
                    [super::for_2d_platformer::LayerNames::FallThrough],
                ),
                TnuaGhostPlatform,
            ));
    }
}
