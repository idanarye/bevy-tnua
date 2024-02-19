use bevy::prelude::*;

#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
use bevy_tnua::TnuaGhostPlatform;
#[cfg(feature = "xpbd3d")]
use bevy_xpbd_3d::{prelude as xpbd, prelude::*};

use crate::MovingPlatform;

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
    let mut cmd = commands.spawn_empty();
    cmd.insert(PbrBundle {
        mesh: meshes.add(Mesh::from(Plane3d::new(Vec3::Y).mesh().size(128.0, 128.0))),
        material: materials.add(Color::WHITE),
        ..Default::default()
    });
    #[cfg(feature = "rapier3d")]
    cmd.insert(rapier::Collider::halfspace(Vec3::Y).unwrap());
    #[cfg(feature = "xpbd3d")]
    {
        cmd.insert(xpbd::RigidBody::Static);
        cmd.insert(xpbd::Collider::halfspace(Vec3::Y));
    }

    let obstacles_material = materials.add(Color::GRAY);
    for ([width, height, depth], transform) in [
        (
            [20.0, 0.1, 2.0],
            Transform::from_xyz(10.0, 10.0, 0.0).with_rotation(Quat::from_rotation_z(0.6)),
        ),
        ([4.0, 2.0, 2.0], Transform::from_xyz(-4.0, 1.0, 0.0)),
        ([6.0, 1.0, 2.0], Transform::from_xyz(-10.0, 4.0, 0.0)),
        ([6.0, 1.0, 2.0], Transform::from_xyz(0.0, 2.6, -5.0)),
    ] {
        let mut cmd = commands.spawn_empty();
        cmd.insert(PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::new(width, height, depth))),
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
            cmd.insert(xpbd::Collider::cuboid(width, height, depth));
        }
    }

    // Fall-through platforms
    let fall_through_obstacles_material = materials.add(Color::PINK.with_a(0.8));
    for y in [2.0, 4.5] {
        let mut cmd = commands.spawn_empty();
        cmd.insert(PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::new(6.0, 0.5, 2.0))),
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
        let mut cmd = commands.spawn_empty();

        cmd.insert(PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid::new(4.0, 1.0, 4.0))),
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
                Vec3::new(-4.0, 6.0, 0.0),
                Vec3::new(-8.0, 6.0, 0.0),
                Vec3::new(-8.0, 10.0, 0.0),
                Vec3::new(-8.0, 10.0, -4.0),
                Vec3::new(-4.0, 10.0, -4.0),
                Vec3::new(-4.0, 10.0, 0.0),
            ],
        ));
    }

    // spawn spinning platform
    {
        let mut cmd = commands.spawn_empty();

        cmd.insert(PbrBundle {
            mesh: meshes.add(Mesh::from(Cylinder {
                radius: 3.0,
                half_height: 0.5,
            })),
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
            cmd.insert(AngularVelocity(Vec3::Y));
            cmd.insert(xpbd::RigidBody::Kinematic);
        }
    }
}
