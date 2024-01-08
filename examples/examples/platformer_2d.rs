use bevy::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
use bevy_tnua::builtins::TnuaBuiltinCrouch;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaGhostSensor, TnuaToggle};
#[cfg(feature = "rapier2d")]
use bevy_tnua_rapier2d::*;
#[cfg(feature = "xpbd2d")]
use bevy_tnua_xpbd2d::*;
#[cfg(feature = "xpbd2d")]
use bevy_xpbd_2d::{prelude as xpbd, prelude::*};

use tnua_examples_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerExample,
};
use tnua_examples_crate::character_control_systems::Dimensionality;
#[cfg(feature = "xpbd2d")]
use tnua_examples_crate::levels_setup::for_2d_platformer::LayerNames;
use tnua_examples_crate::ui::component_alterbation::CommandAlteringSelectors;
use tnua_examples_crate::ui::plotting::PlotSource;
use tnua_examples_crate::{FallingThroughControlScheme, MovingPlatformPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    #[cfg(feature = "rapier2d")]
    {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(RapierDebugRenderPlugin::default());
        app.add_plugins(TnuaRapier2dPlugin);
    }
    #[cfg(feature = "xpbd2d")]
    {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(PhysicsDebugPlugin::default());
        app.add_plugins(TnuaXpbd2dPlugin);
    }
    app.add_plugins(TnuaControllerPlugin);
    app.add_plugins(TnuaCrouchEnforcerPlugin);
    app.add_plugins(tnua_examples_crate::ui::ExampleUi::<
        CharacterMotionConfigForPlatformerExample,
    >::default());
    app.add_systems(Startup, setup_camera);
    app.add_systems(
        Startup,
        tnua_examples_crate::levels_setup::for_2d_platformer::setup_level,
    );
    app.add_systems(Startup, setup_player);
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
    );
    app.add_plugins(MovingPlatformPlugin);
    #[cfg(feature = "rapier2d")]
    {
        app.add_systems(Startup, |mut cfg: ResMut<RapierConfiguration>| {
            // For some odd reason, Rapier 2D defaults to a gravity of 98.1
            cfg.gravity = Vec2::Y * -9.81;
        });
    }
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 14.0, 30.0)
            .with_scale((0.05 * Vec2::ONE).extend(1.0))
            .looking_at(Vec3::new(0.0, 14.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(TransformBundle::from_transform(Transform::from_xyz(
        0.0, 2.0, 0.0,
    )));
    cmd.insert(VisibilityBundle::default());
    #[cfg(feature = "rapier2d")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
        cmd.insert(TnuaRapier2dIOBundle::default());
    }
    #[cfg(feature = "xpbd2d")]
    {
        cmd.insert(xpbd::RigidBody::Dynamic);
        cmd.insert(xpbd::Collider::capsule(1.0, 0.5));
    }
    cmd.insert(TnuaControllerBundle::default());
    cmd.insert(CharacterMotionConfigForPlatformerExample {
        dimensionality: Dimensionality::Dim2,
        speed: 40.0,
        walk: TnuaBuiltinWalk {
            float_height: 2.0,
            ..Default::default()
        },
        actions_in_air: 1,
        jump: TnuaBuiltinJump {
            height: 4.0,
            ..Default::default()
        },
        crouch: TnuaBuiltinCrouch {
            float_offset: -0.9,
            ..Default::default()
        },
        dash_distance: 10.0,
        dash: Default::default(),
    });
    cmd.insert(TnuaToggle::default());
    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vec3::Y, |cmd| {
        #[cfg(feature = "rapier2d")]
        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
        #[cfg(feature = "xpbd2d")]
        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(1.0, 0.0)));
    }));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(TnuaSimpleAirActionsCounter::default());
    cmd.insert(FallingThroughControlScheme::default());
    cmd.insert({
        let command_altering_selectors = CommandAlteringSelectors::default()
            .with_combo(
                "Sensor Shape",
                1,
                &[
                    ("Point", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.remove::<TnuaRapier2dSensorShape>();
                        #[cfg(feature = "xpbd2d")]
                        cmd.remove::<TnuaXpbd2dSensorShape>();
                    }),
                    ("Flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.49, 0.0)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(0.99, 0.0)));
                    }),
                    ("Flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(1.0, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.51, 0.0)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(1.01, 0.0)));
                    }),
                    ("Ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::ball(0.49)));
                    }),
                    ("Ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.5)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::ball(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", false, |mut cmd, lock_tilt| {
                if lock_tilt {
                    #[cfg(feature = "rapier2d")]
                    cmd.insert(rapier::LockedAxes::ROTATION_LOCKED);
                    #[cfg(feature = "xpbd2d")]
                    cmd.insert(xpbd::LockedAxes::new().lock_rotation());
                } else {
                    #[cfg(feature = "rapier2d")]
                    cmd.insert(rapier::LockedAxes::empty());
                    #[cfg(feature = "xpbd2d")]
                    cmd.insert(xpbd::LockedAxes::new());
                }
            })
            .with_checkbox(
                "Phase Through Collision Groups",
                true,
                |mut cmd, use_collision_groups| {
                    #[cfg(feature = "rapier2d")]
                    if use_collision_groups {
                        cmd.insert(CollisionGroups {
                            memberships: Group::GROUP_2,
                            filters: Group::GROUP_2,
                        });
                    } else {
                        cmd.insert(CollisionGroups {
                            memberships: Group::ALL,
                            filters: Group::ALL,
                        });
                    }
                    #[cfg(feature = "xpbd2d")]
                    {
                        let player_layers: &[LayerNames] = if use_collision_groups {
                            &[LayerNames::Player]
                        } else {
                            &[LayerNames::Player, LayerNames::PhaseThrough]
                        };
                        cmd.insert(CollisionLayers::new(player_layers, player_layers));
                    }
                },
            );
        #[cfg(feature = "rapier2d")]
        let command_altering_selectors = command_altering_selectors.with_checkbox(
            "Phase Through Solver Groups",
            true,
            |mut cmd, use_solver_groups| {
                if use_solver_groups {
                    #[cfg(feature = "rapier2d")]
                    cmd.insert(SolverGroups {
                        memberships: Group::GROUP_2,
                        filters: Group::GROUP_2,
                    });
                } else {
                    #[cfg(feature = "rapier2d")]
                    cmd.insert(SolverGroups {
                        memberships: Group::ALL,
                        filters: Group::ALL,
                    });
                }
            },
        );
        command_altering_selectors
    });
    cmd.insert(tnua_examples_crate::ui::TrackedEntity("Player".to_owned()));
    cmd.insert(PlotSource::default());
}
