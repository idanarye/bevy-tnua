#![allow(unexpected_cfgs)]

use bevy::{color::palettes::css, prelude::*};

#[cfg(feature = "avian2d")]
use avian2d::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::prelude::*;
use bevy_tnua::TnuaGhostPlatform;
#[allow(unused_imports)]
use bevy_tnua::math::{AdjustPrecision, Vector2, Vector3};

use crate::{
    level_mechanics::MovingPlatform,
    levels_setup::helper::helper2d::{a_rectangle, a_text_circle, make_kinematic, make_sensor},
};

use super::{
    PositionPlayer,
    helper::{LevelSetupHelper2d, helper2d::the_floor},
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
    helper.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 2.0, 0.0))
        --
        @the_floor(css::GRAY)
        --
        #Moderate_Slope
        @a_rectangle(css::GRAY, Vector2::new(10.0, 0.1))
        Transform::from_xyz(7.0, 7.0, 0.0).with_rotation(Quat::from_rotation_z(0.6))
        --
        #Steep_Slope
        @a_rectangle(css::GRAY, Vector2::new(10.0, 0.1))
        Transform::from_xyz(14.0, 14.0, 0.0).with_rotation(Quat::from_rotation_z(1.0))
        --
        #Box_to_Step_on
        @a_rectangle(css::GRAY, Vector2::new(4.0, 2.0))
        Transform::from_xyz(-4.0, 1.0, 0.0)
        --
        #Floating_Box
        @a_rectangle(css::GRAY, Vector2::new(6.0, 1.0))
        Transform::from_xyz(-10.0, 4.0, 0.0)
        --
        #Box_to_Crawl_Under
        @a_rectangle(css::GRAY, Vector2::new(6.0, 1.0))
        Transform::from_xyz(-20.0, 2.6, 0.0)
    });

    // Fall-through platforms
    for (i, y) in [5.0, 7.5].into_iter().enumerate() {
        helper.spawn_scene(bsn! {
            Name(format!("Fall_Through_#{}", i + 1))
            @a_rectangle(css::PINK, Vector2::new(6.0, 0.5))
            Transform::from_xyz(-20.0, y, -1.0)
            @{
                #[cfg(feature = "rapier2d")]
                bsn! {
                    SolverGroups {
                        memberships: Group::empty(),
                        filters: Group::empty(),
                    }
                }
            }
            @{
                #[cfg(feature = "avian2d")]
                bsn! {
                    CollisionLayers::new([LayerNames::FallThrough], [LayerNames::FallThrough])
                }
            }
            TnuaGhostPlatform
        });
    }

    helper.spawn_scene(bsn! {
        #Collision_Groups
        @a_text_circle("collision\ngroups", 0.01, 1.0)
        Transform::from_xyz(10.0, 2.0, 0.0)
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                CollisionGroups {
                    memberships: Group::GROUP_1,
                    filters: Group::GROUP_1,
                }
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough])
            }
        }
    });

    #[cfg(feature = "rapier2d")]
    helper.spawn_scene(bsn! {
        #Solver_Groups
        @a_text_circle("solver\ngroups", 0.01, 1.0)
        Transform::from_xyz(15.0, 2.0, 0.0)
        SolverGroups {
            memberships: Group::GROUP_1,
            filters: Group::GROUP_1,
        }
    });

    helper.spawn_scene_list(bsn_list! {
        #Sensor
        @a_text_circle("sensor", 0.01, 1.0)
        Transform::from_xyz(20.0, 2.0, 0.0)
        @make_sensor()
        --
        // spawn moving platform
        #Moving_Platform
        @a_rectangle(css::BLUE, Vector2::new(4.0, 1.0))
        Transform::from_xyz(-4.0, 6.0, 0.0)
        @make_kinematic()
        MovingPlatform {
            speed: 4.0,
            locations: vec![
                Vector3::new(-4.0, 6.0, 0.0),
                Vector3::new(-8.0, 6.0, 0.0),
                Vector3::new(-8.0, 10.0, 0.0),
                Vector3::new(-4.0, 10.0, 0.0),
            ],
        }
    });
}
