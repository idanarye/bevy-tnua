#![allow(unexpected_cfgs)]

use bevy::{color::palettes::css, prelude::*};

#[cfg(feature = "avian3d")]
use avian3d::{prelude as avian, prelude::*};
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
use bevy_tnua::TnuaGhostPlatform;
#[allow(unused_imports)]
use bevy_tnua::math::{AdjustPrecision, Vector3};

use crate::{
    level_mechanics::MovingPlatform,
    levels_setup::{
        LevelObject,
        helper::helper3d::{
            a_cuboid, a_cylinder, an_invisible_cuboid, make_kinematic,
            make_kinematic_with_angular_velocity, make_sensor, the_floor,
        },
    },
};

use super::PositionPlayer;

#[cfg(feature = "avian3d")]
#[derive(PhysicsLayer, Default)]
pub enum LayerNames {
    #[default]
    Default,
    Player,
    FallThrough,
    PhaseThrough,
}

pub fn setup_level(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 10.0, 0.0))
        --
        @the_floor(css::WHITE)
        LevelObject
        --
        #Moderate_Slope
        LevelObject
        Transform::from_xyz(7.0, 7.0, 0.0).with_rotation(Quat::from_rotation_z(0.6))
        @a_cuboid(css::GRAY, Vector3::new(10.0, 0.1, 2.0))
        --
        #Steep_Slope
        LevelObject
        Transform::from_xyz(14.0, 14.0, 0.0).with_rotation(Quat::from_rotation_z(1.0))
        @a_cuboid(css::GRAY, Vector3::new(10.0, 0.1, 2.0))
        --
        #Box_to_Step_on
        LevelObject
        Transform::from_xyz(-4.0, 1.0, 0.0)
        @a_cuboid(css::GRAY, Vector3::new(4.0, 2.0, 2.0))
        --
        #Floating_Box
        LevelObject
        Transform::from_xyz(-10.0, 4.0, 0.0)
        @a_cuboid(css::GRAY, Vector3::new(6.0, 1.0, 2.0))
        --
        #Box_to_Crawl_Under
        LevelObject
        Transform::from_xyz(0.0, 2.6, -5.0)
        @a_cuboid(css::GRAY, Vector3::new(6.0, 1.0, 2.0))
        --
        // Fall-through platforms
        {
            [2.0, 4.5].into_iter().enumerate().map(|(i, y)| {
                bsn! {
                    Name(format!("Fall_Through_#{}", i + 1))
                        LevelObject
                        @a_cuboid(css::PINK.with_alpha(0.8), Vector3::new(6.0, 0.5, 2.0))
                        Transform::from_xyz(6.0, y, 10.0)
                        @{
                            #[cfg(feature = "rapier3d")]
                            bsn! {
                                SolverGroups {
                                    memberships: Group::empty(),
                                    filters: Group::empty(),
                                }
                            }
                        }
                    @{
                        #[cfg(feature = "avian3d")]
                        bsn! {
                            CollisionLayers::new(
                                [LayerNames::FallThrough],
                                [LayerNames::FallThrough],
                            )
                        }
                    }
                    TnuaGhostPlatform
                }
            }).collect::<Vec<_>>()
        }
        --
        #Collision_Groups
        LevelObject
        WorldAssetRoot("collision-groups-text.glb#Scene0")
        @an_invisible_cuboid(Vector3::new(4.0, 2.0, 4.0))
        Transform::from_xyz(10.0, 2.0, 1.0)
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::cuboid(2.0, 1.0, 2.0)
                CollisionGroups {
                    memberships: Group::GROUP_1,
                    filters: Group::GROUP_1,
                }
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::cuboid(4.0, 2.0, 4.0)
                CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough])
            },
        }
        --
        {
            #[cfg(feature = "rapier3d")]
            bsn_list! {
                #Solver_Groups
                LevelObject
                WorldAssetRoot("solver-groups-text.glb#Scene0")
                @an_invisible_cuboid(Vector3::new(4.0, 2.0, 4.0))
                Transform::from_xyz(15.0, 2.0, 1.0) // .with_scale(0.01 * Vec3::ONE),
                SolverGroups {
                    memberships: Group::GROUP_1,
                    filters: Group::GROUP_1,
                }
            }
        }
        --
        #Sensor
        LevelObject
        WorldAssetRoot("sensor-text.glb#Scene0")
        @an_invisible_cuboid(Vector3::new(4.0, 2.0, 4.0))
        Transform::from_xyz(20.0, 2.0, 1.0)
        @make_sensor()
        --
        // spawn moving and spinning platforms
        #Moving_Platform
        @a_cuboid(css::BLUE, Vector3::new(4.0, 1.0, 4.0))
        Transform::from_xyz(-4.0, 6.0, 0.0)
        @make_kinematic()
        MovingPlatform {
            speed: 4.0,
            locations: vec![
                Vector3::new(-4.0, 6.0, 0.0),
                Vector3::new(-8.0, 6.0, 0.0),
                Vector3::new(-8.0, 10.0, 0.0),
                Vector3::new(-8.0, 10.0, -4.0),
                Vector3::new(-4.0, 10.0, -4.0),
                Vector3::new(-4.0, 10.0, 0.0),
            ],
        }
        --
        #Spinning_Platform
        @a_cylinder(css::BLUE, 3.0, 0.5)
        Transform::from_xyz(-2.0, 2.0, 10.0)
        @make_kinematic_with_angular_velocity(Vector3::Y)
    });
}
