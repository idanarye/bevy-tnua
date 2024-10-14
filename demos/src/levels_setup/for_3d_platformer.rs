use bevy::{color::palettes::css, prelude::*};

#[cfg(feature = "avian3d")]
use avian3d::{prelude as avian, prelude::*};
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
#[allow(unused_imports)]
use bevy_tnua::math::{AdjustPrecision, Vector3};
use bevy_tnua::TnuaGhostPlatform;

use crate::level_mechanics::MovingPlatform;

use super::{
    helper::{LevelSetupHelper3d, LevelSetupHelper3dEntityCommandsExtension},
    PositionPlayer,
};

#[cfg(feature = "avian3d")]
#[derive(PhysicsLayer, Default)]
pub enum LayerNames {
    #[default]
    Default,
    Player,
    FallThrough,
    PhaseThrough,
}

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 10.0, 0.0)));

    helper.spawn_floor(css::WHITE);

    let mut obstacles_helper = helper.with_color(css::GRAY);
    obstacles_helper.spawn_cuboid(
        "Moderate Slope",
        Transform::from_xyz(7.0, 7.0, 0.0).with_rotation(Quat::from_rotation_z(0.6)),
        Vector3::new(10.0, 0.1, 2.0),
    );
    obstacles_helper.spawn_cuboid(
        "Steep Slope",
        Transform::from_xyz(14.0, 14.0, 0.0).with_rotation(Quat::from_rotation_z(1.0)),
        Vector3::new(10.0, 0.1, 2.0),
    );
    obstacles_helper.spawn_cuboid(
        "Box to Step on",
        Transform::from_xyz(-4.0, 1.0, 0.0),
        Vector3::new(4.0, 2.0, 2.0),
    );
    obstacles_helper.spawn_cuboid(
        "Floating Box",
        Transform::from_xyz(-10.0, 4.0, 0.0),
        Vector3::new(6.0, 1.0, 2.0),
    );
    obstacles_helper.spawn_cuboid(
        "Box to Crawl Under",
        Transform::from_xyz(0.0, 2.6, -5.0),
        Vector3::new(6.0, 1.0, 2.0),
    );

    // Fall-through platforms
    //let fall_through_obstacles_material = materials.add(css::PINK.with_alpha(0.8));
    let mut fall_through_obstacles_helper = helper.with_color(css::PINK.with_alpha(0.8));
    for (i, y) in [2.0, 4.5].into_iter().enumerate() {
        let mut cmd = fall_through_obstacles_helper.spawn_cuboid(
            format!("Fall Through #{}", i + 1),
            Transform::from_xyz(6.0, y, 10.0),
            Vector3::new(6.0, 0.5, 2.0),
        );
        #[cfg(feature = "rapier3d")]
        {
            cmd.insert(SolverGroups {
                memberships: Group::empty(),
                filters: Group::empty(),
            });
        }
        #[cfg(feature = "avian3d")]
        {
            cmd.insert(CollisionLayers::new(
                [LayerNames::FallThrough],
                [LayerNames::FallThrough],
            ));
        }
        cmd.insert(TnuaGhostPlatform);
    }

    helper
        .spawn_scene_cuboid(
            "Collision Groups",
            "collision-groups-text.glb#Scene0",
            Transform::from_xyz(10.0, 2.0, 1.0),
            Vector3::new(4.0, 2.0, 4.0),
        )
        .insert((
            #[cfg(feature = "rapier3d")]
            (
                rapier::Collider::cuboid(2.0, 1.0, 2.0),
                CollisionGroups {
                    memberships: Group::GROUP_1,
                    filters: Group::GROUP_1,
                },
            ),
            #[cfg(feature = "avian3d")]
            (
                avian::RigidBody::Static,
                avian::Collider::cuboid(4.0, 2.0, 4.0),
                CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough]),
            ),
        ));

    #[cfg(feature = "rapier3d")]
    helper
        .spawn_scene_cuboid(
            "Solver Groups",
            "solver-groups-text.glb#Scene0",
            Transform::from_xyz(15.0, 2.0, 1.0), // .with_scale(0.01 * Vec3::ONE),
            Vector3::new(4.0, 2.0, 4.0),
        )
        .insert(SolverGroups {
            memberships: Group::GROUP_1,
            filters: Group::GROUP_1,
        });

    helper
        .spawn_scene_cuboid(
            "Sensor",
            "sensor-text.glb#Scene0",
            Transform::from_xyz(20.0, 2.0, 1.0),
            Vector3::new(4.0, 2.0, 4.0),
        )
        .insert((
            #[cfg(feature = "rapier3d")]
            rapier::Sensor,
            #[cfg(feature = "avian3d")]
            avian::Sensor,
        ));

    // spawn moving and spinning platforms
    let mut moving_platform_helper = helper.with_color(css::BLUE);
    moving_platform_helper
        .spawn_cuboid(
            "Moving Platform",
            Transform::from_xyz(-4.0, 6.0, 0.0),
            Vector3::new(4.0, 1.0, 4.0),
        )
        .make_kinematic()
        .insert(MovingPlatform::new(
            4.0,
            &[
                Vector3::new(-4.0, 6.0, 0.0),
                Vector3::new(-8.0, 6.0, 0.0),
                Vector3::new(-8.0, 10.0, 0.0),
                Vector3::new(-8.0, 10.0, -4.0),
                Vector3::new(-4.0, 10.0, -4.0),
                Vector3::new(-4.0, 10.0, 0.0),
            ],
        ));

    moving_platform_helper
        .spawn_cylinder(
            "Spinning Platform",
            Transform::from_xyz(-2.0, 2.0, 10.0),
            3.0,
            0.5,
        )
        .make_kinematic_with_angular_velocity(Vector3::Y);
}
