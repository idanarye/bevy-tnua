use bevy::prelude::*;

#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
#[allow(unused_imports)]
use bevy_tnua::math::{AdjustPrecision, Vector3};
use bevy_tnua::TnuaGhostPlatform;
#[cfg(feature = "xpbd3d")]
use bevy_xpbd_3d::{prelude as xpbd, prelude::*};

use crate::MovingPlatform;

use super::{LevelObject, PositionPlayer};

#[cfg(feature = "xpbd3d")]
#[derive(PhysicsLayer)]
pub enum LayerNames {
    Player,
    FallThrough,
    PhaseThrough,
}

pub fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(PositionPlayer::from(Vec3::new(0.0, 10.0, 0.0)));

    let mut cmd = commands.spawn((LevelObject, Name::new("Floor")));
    cmd.insert(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(128.0, 128.0)),
        material: materials.add(Color::WHITE),
        ..Default::default()
    });
    #[cfg(feature = "rapier3d")]
    cmd.insert(rapier::Collider::halfspace(Vec3::Y).unwrap());
    #[cfg(feature = "xpbd3d")]
    {
        cmd.insert(xpbd::RigidBody::Static);
        cmd.insert(xpbd::Collider::halfspace(Vector3::Y));
    }

    let obstacles_material = materials.add(Color::GRAY);
    for (name, [width, height, depth], transform) in [
        (
            "Moderate Slope",
            [10.0, 0.1, 2.0],
            Transform::from_xyz(7.0, 7.0, 0.0).with_rotation(Quat::from_rotation_z(0.6)),
        ),
        (
            "Steep Slope",
            [10.0, 0.1, 2.0],
            Transform::from_xyz(14.0, 14.0, 0.0).with_rotation(Quat::from_rotation_z(1.0)),
        ),
        (
            "Box to Step on",
            [4.0, 2.0, 2.0],
            Transform::from_xyz(-4.0, 1.0, 0.0),
        ),
        (
            "Floating Box",
            [6.0, 1.0, 2.0],
            Transform::from_xyz(-10.0, 4.0, 0.0),
        ),
        (
            "Box to Crawl Under",
            [6.0, 1.0, 2.0],
            Transform::from_xyz(0.0, 2.6, -5.0),
        ),
    ] {
        let mut cmd = commands.spawn((LevelObject, Name::new(name)));
        cmd.insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(width, height, depth)),
            material: obstacles_material.clone(),
            transform,
            ..Default::default()
        });
        #[cfg(feature = "rapier3d")]
        cmd.insert(rapier::Collider::cuboid(
            0.5 * width,
            0.5 * height,
            0.5 * depth,
        ));
        #[cfg(feature = "xpbd3d")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::cuboid(
                width.adjust_precision(),
                height.adjust_precision(),
                depth.adjust_precision(),
            ));
        }
    }

    // Fall-through platforms
    let fall_through_obstacles_material = materials.add(Color::PINK.with_a(0.8));
    for (i, y) in [2.0, 4.5].into_iter().enumerate() {
        let mut cmd = commands.spawn((LevelObject, Name::new(format!("Fall Through #{}", i + 1))));
        cmd.insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(6.0, 0.5, 2.0)),
            material: fall_through_obstacles_material.clone(),
            transform: Transform::from_xyz(6.0, y, 10.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier3d")]
        {
            cmd.insert(rapier::Collider::cuboid(3.0, 0.25, 1.0));
            cmd.insert(SolverGroups {
                memberships: Group::empty(),
                filters: Group::empty(),
            });
        }
        #[cfg(feature = "xpbd3d")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::cuboid(6.0, 0.5, 2.0));
            cmd.insert(CollisionLayers::new(
                [LayerNames::FallThrough],
                [LayerNames::FallThrough],
            ));
        }
        cmd.insert(TnuaGhostPlatform);
    }

    commands.spawn((
        LevelObject,
        Name::new("Collision Groups"),
        SceneBundle {
            scene: asset_server.load("collision-groups-text.glb#Scene0"),
            transform: Transform::from_xyz(10.0, 2.0, 1.0), // .with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        },
        #[cfg(feature = "rapier3d")]
        (
            rapier::Collider::cuboid(2.0, 1.0, 2.0),
            CollisionGroups {
                memberships: Group::GROUP_1,
                filters: Group::GROUP_1,
            },
        ),
        #[cfg(feature = "xpbd3d")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::cuboid(4.0, 2.0, 4.0),
            CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough]),
        ),
    ));

    #[cfg(feature = "rapier3d")]
    commands.spawn((
        LevelObject,
        Name::new("Solver Groups"),
        SceneBundle {
            scene: asset_server.load("solver-groups-text.glb#Scene0"),
            transform: Transform::from_xyz(15.0, 2.0, 1.0), // .with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        },
        rapier::Collider::cuboid(2.0, 1.0, 2.0),
        SolverGroups {
            memberships: Group::GROUP_1,
            filters: Group::GROUP_1,
        },
    ));

    commands.spawn((
        LevelObject,
        Name::new("Sensor"),
        SceneBundle {
            scene: asset_server.load("sensor-text.glb#Scene0"),
            transform: Transform::from_xyz(20.0, 2.0, 1.0), // .with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        },
        #[cfg(feature = "rapier3d")]
        (rapier::Collider::cuboid(2.0, 1.0, 2.0), rapier::Sensor),
        #[cfg(feature = "xpbd3d")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::cuboid(4.0, 2.0, 4.0),
            xpbd::Sensor,
        ),
    ));

    // spawn moving platform
    {
        let mut cmd = commands.spawn((LevelObject, Name::new("Moving Platform")));
        cmd.insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(4.0, 1.0, 4.0)),
            material: materials.add(Color::BLUE),
            transform: Transform::from_xyz(-4.0, 6.0, 0.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier3d")]
        {
            cmd.insert(rapier::Collider::cuboid(2.0, 0.5, 2.0));
            cmd.insert(Velocity::default());
            cmd.insert(rapier::RigidBody::KinematicVelocityBased);
        }
        #[cfg(feature = "xpbd3d")]
        {
            cmd.insert(xpbd::Collider::cuboid(4.0, 1.0, 4.0));
            cmd.insert(xpbd::RigidBody::Kinematic);
        }
        cmd.insert(MovingPlatform::new(
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
    }

    // spawn spinning platform
    {
        let mut cmd = commands.spawn((LevelObject, Name::new("Spinning Platform")));

        cmd.insert(PbrBundle {
            mesh: meshes.add(Cylinder {
                radius: 3.0,
                half_height: 0.5,
            }),
            material: materials.add(Color::BLUE),
            transform: Transform::from_xyz(-2.0, 2.0, 10.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier3d")]
        {
            cmd.insert(rapier::Collider::cylinder(0.5, 3.0));
            cmd.insert(Velocity::angular(Vec3::Y));
            cmd.insert(rapier::RigidBody::KinematicVelocityBased);
        }
        #[cfg(feature = "xpbd3d")]
        {
            cmd.insert(xpbd::Collider::cylinder(1.0, 3.0));
            cmd.insert(AngularVelocity(Vector3::Y));
            cmd.insert(xpbd::RigidBody::Kinematic);
        }
    }
}
