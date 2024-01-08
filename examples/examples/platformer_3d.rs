use bevy::prelude::*;
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
use bevy_tnua::builtins::TnuaBuiltinCrouch;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaAnimatingState, TnuaGhostSensor, TnuaToggle};
#[cfg(feature = "rapier3d")]
use bevy_tnua_rapier3d::*;
#[cfg(feature = "xpbd3d")]
use bevy_tnua_xpbd3d::*;
#[cfg(feature = "xpbd3d")]
use bevy_xpbd_3d::{prelude as xpbd, prelude::*};

use tnua_examples_crate::character_animating_systems::platformer_animating_systems::{
    animate_platformer_character, AnimationState,
};
use tnua_examples_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerExample,
};
use tnua_examples_crate::character_control_systems::Dimensionality;
#[cfg(feature = "xpbd3d")]
use tnua_examples_crate::levels_setup::for_3d_platformer::LayerNames;
use tnua_examples_crate::ui::component_alterbation::CommandAlteringSelectors;
use tnua_examples_crate::ui::plotting::PlotSource;
use tnua_examples_crate::util::animating::{animation_patcher_system, GltfSceneHandler};
use tnua_examples_crate::{FallingThroughControlScheme, MovingPlatformPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    #[cfg(feature = "rapier3d")]
    {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(TnuaRapier3dPlugin);
    }
    #[cfg(feature = "xpbd3d")]
    {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(TnuaXpbd3dPlugin);
    }
    app.add_plugins(TnuaControllerPlugin);
    app.add_plugins(TnuaCrouchEnforcerPlugin);
    app.add_plugins(tnua_examples_crate::ui::ExampleUi::<
        CharacterMotionConfigForPlatformerExample,
    >::default());
    app.add_systems(Startup, setup_camera);
    app.add_systems(
        Startup,
        tnua_examples_crate::levels_setup::for_3d_platformer::setup_level,
    );
    app.add_systems(Startup, setup_player);
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
    );
    app.add_systems(Update, animation_patcher_system);
    app.add_systems(Update, animate_platformer_character);
    app.add_plugins(MovingPlatformPlugin);
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 16.0, 40.0)
            .looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::default().looking_at(-Vec3::Y, Vec3::Z),
        ..Default::default()
    });
}

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(SceneBundle {
        scene: asset_server.load("player.glb#Scene0"),
        transform: Transform::from_xyz(0.0, 10.0, 0.0),
        ..Default::default()
    });
    cmd.insert(GltfSceneHandler {
        names_from: asset_server.load("player.glb"),
    });
    #[cfg(feature = "rapier3d")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
        cmd.insert(TnuaRapier3dIOBundle::default());
    }
    #[cfg(feature = "xpbd3d")]
    {
        cmd.insert(xpbd::RigidBody::Dynamic);
        cmd.insert(xpbd::Collider::capsule(1.0, 0.5));
    }
    cmd.insert(TnuaControllerBundle::default());

    cmd.insert(CharacterMotionConfigForPlatformerExample {
        dimensionality: Dimensionality::Dim3,
        speed: 20.0,
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
        #[cfg(feature = "rapier3d")]
        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
            0.0, 0.5,
        )));
        #[cfg(feature = "xpbd3d")]
        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.5)));
    }));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(TnuaSimpleAirActionsCounter::default());
    cmd.insert(FallingThroughControlScheme::default());
    cmd.insert(TnuaAnimatingState::<AnimationState>::default());
    cmd.insert({
        let command_altering_selectors = CommandAlteringSelectors::default()
            .with_combo(
                "Sensor Shape",
                1,
                &[
                    ("no", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.remove::<TnuaRapier3dSensorShape>();
                        #[cfg(feature = "xpbd3d")]
                        cmd.remove::<TnuaXpbd3dSensorShape>();
                    }),
                    ("flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.49,
                        )));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.49)));
                    }),
                    ("flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.5,
                        )));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.5)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.51,
                        )));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.51)));
                    }),
                    ("ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::ball(0.49)));
                    }),
                    ("ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::ball(0.5)));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::ball(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", true, |mut cmd, lock_tilt| {
                if lock_tilt {
                    #[cfg(feature = "rapier3d")]
                    cmd.insert(
                        rapier::LockedAxes::ROTATION_LOCKED_X
                            | rapier::LockedAxes::ROTATION_LOCKED_Z,
                    );
                    #[cfg(feature = "xpbd3d")]
                    cmd.insert(xpbd::LockedAxes::new().lock_rotation_x().lock_rotation_z());
                } else {
                    #[cfg(feature = "rapier3d")]
                    cmd.insert(rapier::LockedAxes::empty());
                    #[cfg(feature = "xpbd3d")]
                    cmd.insert(xpbd::LockedAxes::new());
                }
            })
            .with_checkbox(
                "Phase Through Collision Groups",
                true,
                |mut cmd, use_collision_groups| {
                    #[cfg(feature = "rapier3d")]
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
                    #[cfg(feature = "xpbd3d")]
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
        #[cfg(feature = "rapier3d")]
        let command_altering_selectors = command_altering_selectors.with_checkbox(
            "Phase Through Solver Groups",
            true,
            |mut cmd, use_solver_groups| {
                if use_solver_groups {
                    cmd.insert(SolverGroups {
                        memberships: Group::GROUP_2,
                        filters: Group::GROUP_2,
                    });
                } else {
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
