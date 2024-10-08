use bevy::{color::palettes::css, prelude::*};

#[cfg(feature = "avian2d")]
use avian2d::{prelude as avian, prelude::*};
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
#[allow(unused_imports)]
use bevy_tnua::math::{AdjustPrecision, Vector2, Vector3};
use bevy_tnua::TnuaGhostPlatform;

use crate::level_mechanics::MovingPlatform;

use super::{
    helper::{LevelSetupHelper2d, LevelSetupHelper2dEntityCommandsExtension},
    PositionPlayer,
};

#[cfg(feature = "avian2d")]
#[derive(PhysicsLayer, Default)]
pub enum LayerNames {
    #[default]
    Default,
    Player,
    FallThrough,
    PhaseThrough,
}

pub fn setup_level(mut helper: LevelSetupHelper2d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 2.0, 0.0)));

    helper.spawn_floor(css::GRAY);

    helper.spawn_rectangle(
        "Moderate Slope",
        css::GRAY,
        Transform::from_xyz(7.0, 7.0, 0.0).with_rotation(Quat::from_rotation_z(0.6)),
        Vector2::new(10.0, 0.1),
    );
    helper.spawn_rectangle(
        "Steep Slope",
        css::GRAY,
        Transform::from_xyz(14.0, 14.0, 0.0).with_rotation(Quat::from_rotation_z(1.0)),
        Vector2::new(10.0, 0.1),
    );
    helper.spawn_rectangle(
        "Box to Step on",
        css::GRAY,
        Transform::from_xyz(-4.0, 1.0, 0.0),
        Vector2::new(4.0, 2.0),
    );
    helper.spawn_rectangle(
        "Floating Box",
        css::GRAY,
        Transform::from_xyz(-10.0, 4.0, 0.0),
        Vector2::new(6.0, 1.0),
    );
    helper.spawn_rectangle(
        "Box to Crawl Under",
        css::GRAY,
        Transform::from_xyz(-20.0, 2.6, 0.0),
        Vector2::new(6.0, 1.0),
    );

    // Fall-through platforms
    for (i, y) in [5.0, 7.5].into_iter().enumerate() {
        helper
            .spawn_rectangle(
                format!("Fall Through #{}", i + 1),
                css::PINK,
                Transform::from_xyz(-20.0, y, -1.0),
                Vector2::new(6.0, 0.5),
            )
            .insert((
                #[cfg(feature = "rapier2d")]
                SolverGroups {
                    memberships: Group::empty(),
                    filters: Group::empty(),
                },
                #[cfg(feature = "avian2d")]
                CollisionLayers::new([LayerNames::FallThrough], [LayerNames::FallThrough]),
                TnuaGhostPlatform,
            ));
    }

    helper
        .spawn_text_circle(
            "Collision Groups",
            "collision\ngroups",
            0.01,
            Transform::from_xyz(10.0, 2.0, 0.0),
            1.0,
        )
        .insert((
            #[cfg(feature = "rapier2d")]
            CollisionGroups {
                memberships: Group::GROUP_1,
                filters: Group::GROUP_1,
            },
            #[cfg(feature = "avian2d")]
            CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough]),
        ));

    #[cfg(feature = "rapier2d")]
    helper
        .spawn_text_circle(
            "Solver Groups",
            "solver\ngroups",
            0.01,
            Transform::from_xyz(15.0, 2.0, 0.0),
            1.0,
        )
        .insert(SolverGroups {
            memberships: Group::GROUP_1,
            filters: Group::GROUP_1,
        });

    helper
        .spawn_text_circle(
            "Sensor",
            "sensor",
            0.01,
            Transform::from_xyz(20.0, 2.0, 0.0),
            1.0,
        )
        .insert((
            #[cfg(feature = "rapier2d")]
            rapier::Sensor,
            #[cfg(feature = "avian2d")]
            avian::Sensor,
        ));

    // spawn moving platform
    helper
        .spawn_rectangle(
            "Moving Platform",
            css::BLUE,
            Transform::from_xyz(-4.0, 6.0, 0.0),
            Vector2::new(4.0, 1.0),
        )
        .make_kinematic()
        .insert(MovingPlatform::new(
            4.0,
            &[
                Vector3::new(-4.0, 6.0, 0.0),
                Vector3::new(-8.0, 6.0, 0.0),
                Vector3::new(-8.0, 10.0, 0.0),
                Vector3::new(-4.0, 10.0, 0.0),
            ],
        ));
}
