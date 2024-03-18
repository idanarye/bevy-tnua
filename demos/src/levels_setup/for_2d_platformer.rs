use bevy::prelude::*;

#[cfg(feature = "rapier2d")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
#[allow(unused_imports)]
use bevy_tnua::math::{AdjustPrecision, Vector2, Vector3};
use bevy_tnua::TnuaGhostPlatform;
#[cfg(feature = "xpbd2d")]
use bevy_xpbd_2d::{prelude as xpbd, prelude::*};

use crate::MovingPlatform;

#[cfg(feature = "xpbd2d")]
#[derive(PhysicsLayer)]
pub enum LayerNames {
    Player,
    FallThrough,
    PhaseThrough,
}

pub fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(128.0, 0.5)),
            color: Color::GRAY,
            ..Default::default()
        },
        ..Default::default()
    });
    #[cfg(feature = "rapier2d")]
    cmd.insert(rapier::Collider::halfspace(Vec2::Y).unwrap());
    #[cfg(feature = "xpbd2d")]
    {
        cmd.insert(xpbd::RigidBody::Static);
        cmd.insert(xpbd::Collider::halfspace(Vector2::Y));
    }

    for ([width, height], transform) in [
        (
            [20.0, 0.1],
            Transform::from_xyz(10.0, 10.0, 0.0).with_rotation(Quat::from_rotation_z(0.6)),
        ),
        ([4.0, 2.0], Transform::from_xyz(-4.0, 1.0, 0.0)),
        ([6.0, 1.0], Transform::from_xyz(-10.0, 4.0, 0.0)),
        ([6.0, 1.0], Transform::from_xyz(-20.0, 2.6, 0.0)),
    ] {
        let mut cmd = commands.spawn_empty();
        cmd.insert(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(width, height)),
                color: Color::GRAY,
                ..Default::default()
            },
            transform,
            ..Default::default()
        });
        #[cfg(feature = "rapier2d")]
        cmd.insert(rapier::Collider::cuboid(0.5 * width, 0.5 * height));
        #[cfg(feature = "xpbd2d")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::rectangle(
                width.adjust_precision(),
                height.adjust_precision(),
            ));
        }
    }

    // Fall-through platforms
    for y in [5.0, 7.5] {
        let mut cmd = commands.spawn_empty();
        cmd.insert(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(6.0, 0.5)),
                color: Color::PINK,
                ..Default::default()
            },
            transform: Transform::from_xyz(-20.0, y, -1.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier2d")]
        {
            cmd.insert(rapier::Collider::cuboid(3.0, 0.25));
            cmd.insert(SolverGroups {
                memberships: Group::empty(),
                filters: Group::empty(),
            });
        }
        #[cfg(feature = "xpbd2d")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::rectangle(6.0, 0.5));
            cmd.insert(CollisionLayers::new(
                [LayerNames::FallThrough],
                [LayerNames::FallThrough],
            ));
        }
        cmd.insert(TnuaGhostPlatform);
    }

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(10.0, 2.0, 0.0)),
        #[cfg(feature = "rapier2d")]
        (
            rapier::Collider::ball(1.0),
            CollisionGroups {
                memberships: Group::GROUP_1,
                filters: Group::GROUP_1,
            },
        ),
        #[cfg(feature = "xpbd2d")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::circle(1.0),
            CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough]),
        ),
    ));
    commands.spawn(Text2dBundle {
        text: Text::from_section(
            "collision\ngroups",
            TextStyle {
                font: asset_server.load("FiraSans-Bold.ttf"),
                font_size: 72.0,
                color: Color::WHITE,
            },
        )
        .with_justify(JustifyText::Center),
        transform: Transform::from_xyz(10.0, 2.0, 1.0).with_scale(0.01 * Vec3::ONE),
        ..Default::default()
    });

    #[cfg(feature = "rapier2d")]
    {
        commands.spawn((
            TransformBundle::from_transform(Transform::from_xyz(15.0, 2.0, 0.0)),
            rapier::Collider::ball(1.0),
            SolverGroups {
                memberships: Group::GROUP_1,
                filters: Group::GROUP_1,
            },
        ));
        commands.spawn(Text2dBundle {
            text: Text::from_section(
                "solver\ngroups",
                TextStyle {
                    font: asset_server.load("FiraSans-Bold.ttf"),
                    font_size: 72.0,
                    color: Color::WHITE,
                },
            )
            .with_justify(JustifyText::Center),
            transform: Transform::from_xyz(15.0, 2.0, 1.0).with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        });
    }

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(20.0, 2.0, 0.0)),
        #[cfg(feature = "rapier2d")]
        (rapier::Collider::ball(1.0), rapier::Sensor),
        #[cfg(feature = "xpbd2d")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::circle(1.0),
            xpbd::Sensor,
        ),
    ));
    commands.spawn(Text2dBundle {
        text: Text::from_section(
            "sensor",
            TextStyle {
                font: asset_server.load("FiraSans-Bold.ttf"),
                font_size: 72.0,
                color: Color::WHITE,
            },
        )
        .with_justify(JustifyText::Center),
        transform: Transform::from_xyz(20.0, 2.0, 1.0).with_scale(0.01 * Vec3::ONE),
        ..Default::default()
    });

    // spawn moving platform
    {
        let mut cmd = commands.spawn_empty();
        cmd.insert(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(4.0, 1.0)),
                color: Color::BLUE,
                ..Default::default()
            },
            transform: Transform::from_xyz(-4.0, 6.0, 0.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier2d")]
        {
            cmd.insert(rapier::Collider::cuboid(2.0, 0.5));
            cmd.insert(Velocity::default());
            cmd.insert(rapier::RigidBody::KinematicVelocityBased);
        }
        #[cfg(feature = "xpbd2d")]
        {
            cmd.insert(xpbd::Collider::rectangle(4.0, 1.0));
            cmd.insert(xpbd::RigidBody::Kinematic);
        }
        cmd.insert(MovingPlatform::new(
            4.0,
            &[
                Vector3::new(-4.0, 6.0, 0.0),
                Vector3::new(-8.0, 6.0, 0.0),
                Vector3::new(-8.0, 10.0, 0.0),
                Vector3::new(-4.0, 10.0, 0.0),
            ],
        ));
    }
}
