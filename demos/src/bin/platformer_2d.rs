use bevy::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
use bevy_tnua::builtins::TnuaBuiltinCrouch;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::math::{AsF32, Vector3};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaGhostSensor, TnuaToggle};
#[cfg(feature = "rapier2d")]
use bevy_tnua_rapier2d::*;
#[cfg(feature = "xpbd2d")]
use bevy_tnua_xpbd2d::*;
#[cfg(feature = "xpbd2d")]
use bevy_xpbd_2d::{prelude as xpbd, prelude::*};

use tnua_demos_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerDemo, FallingThroughControlScheme,
};
use tnua_demos_crate::character_control_systems::Dimensionality;
#[cfg(feature = "xpbd2d")]
use tnua_demos_crate::levels_setup::for_2d_platformer::LayerNames;
use tnua_demos_crate::ui::component_alterbation::CommandAlteringSelectors;
#[cfg(feature = "egui")]
use tnua_demos_crate::ui::plotting::PlotSource;
use tnua_demos_crate::MovingPlatformPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "rapier2d")]
    {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(RapierDebugRenderPlugin::default());
        // To use Tnua with bevy_rapier2d, you need the `TnuaRapier2dPlugin` plugin from
        // bevy-tnua-rapier2d.
        app.add_plugins(TnuaRapier2dPlugin);
    }
    #[cfg(feature = "xpbd2d")]
    {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(PhysicsDebugPlugin::default());
        // To use Tnua with bevy_xpbd_2d, you need the `TnuaXpbd2dPlugin` plugin from
        // bevy-tnua-xpbd2d.
        app.add_plugins(TnuaXpbd2dPlugin);
    }

    // This is Tnua's main plugin.
    app.add_plugins(TnuaControllerPlugin);

    // This plugin supports `TnuaCrouchEnforcer`, which prevents the character from standing up
    // while obstructed by an obstacle.
    app.add_plugins(TnuaCrouchEnforcerPlugin);

    app.add_plugins(tnua_demos_crate::ui::DemoUi::<
        CharacterMotionConfigForPlatformerDemo,
    >::default());
    app.add_systems(Startup, setup_camera_and_lights);
    app.add_systems(
        Startup,
        tnua_demos_crate::levels_setup::for_2d_platformer::setup_level,
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

fn setup_camera_and_lights(mut commands: Commands) {
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

    // The character entity must be configured as a dynamic rigid body of the physics backend.
    #[cfg(feature = "rapier2d")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
        // For Rapier, an "IO" bundle needs to be added so that Tnua will have all the components
        // it needs to interact with Rapier.
        cmd.insert(TnuaRapier2dIOBundle::default());
    }
    #[cfg(feature = "xpbd2d")]
    {
        cmd.insert(xpbd::RigidBody::Dynamic);
        cmd.insert(xpbd::Collider::capsule(1.0, 0.5));
        // XPBD does not need an "IO" bundle.
    }

    // This bundle container `TnuaController` - the main interface of Tnua with the user code - as
    // well as the main components used as API between the main plugin and the physics backend
    // integration. These components (and the IO bundle, in case of backends that need one like
    // Rapier) are the only mandatory Tnua components - but this example will also add some
    // components used for more advanced features.
    //
    // Read examples/src/character_control_systems/platformer_control_systems.rs to see how
    // `TnuaController` is used in this example.
    cmd.insert(TnuaControllerBundle::default());

    cmd.insert(CharacterMotionConfigForPlatformerDemo {
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
        one_way_platforms_min_proximity: 1.0,
        falling_through: FallingThroughControlScheme::SingleFall,
    });

    // An entity's Tnua behavior can be toggled individually with this component, if inserted.
    cmd.insert(TnuaToggle::default());
    cmd.insert({
        let command_altering_selectors = CommandAlteringSelectors::default()
            // By default Tnua uses a raycast, but this could be a problem if the character stands
            // just past the edge while part of its body is above the platform. To solve this, we
            // need to cast a shape - which is physics-engine specific. We set the shape using a
            // component.
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
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::rectangle(0.99, 0.0)));
                    }),
                    ("Flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::rectangle(1.0, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.51, 0.0)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::rectangle(1.01, 0.0)));
                    }),
                    ("Ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::circle(0.49)));
                    }),
                    ("Ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.5)));
                        #[cfg(feature = "xpbd2d")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::circle(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", false, |mut cmd, lock_tilt| {
                // Tnua will automatically apply angular impulses/forces to fix the tilt and make
                // the character stand upward, but it is also possible to just let the physics
                // engine prevent rotation (other than around the Y axis, for turning)
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
                        let player_layers: LayerMask = if use_collision_groups {
                            [LayerNames::Player].into()
                        } else {
                            [LayerNames::Player, LayerNames::PhaseThrough].into()
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

    // `TnuaCrouchEnforcer` can be used to prevent the character from standing up when obstructed.
    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vector3::Y, |cmd| {
        // It needs a sensor shape because it needs to do a shapecast upwards. Without a sensor shape
        // it'd do a raycast.
        #[cfg(feature = "rapier2d")]
        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
        #[cfg(feature = "xpbd2d")]
        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::rectangle(1.0, 0.0)));
    }));

    // The ghost sensor is used for detecting ghost platforms - platforms configured in the physics
    // backend to not contact with the character (or detect the contact but not apply physical
    // forces based on it) and marked with the `TnuaGhostPlatform` component. These can then be
    // used as one-way platforms.
    cmd.insert(TnuaGhostSensor::default());

    // This helper is used to operate the ghost sensor and ghost platforms and implement
    // fall-through behavior where the player can intentionally fall through a one-way platform.
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());

    // This helper keeps track of air actions like jumps or air dashes.
    cmd.insert(TnuaSimpleAirActionsCounter::default());

    cmd.insert(tnua_demos_crate::ui::TrackedEntity("Player".to_owned()));
    #[cfg(feature = "egui")]
    cmd.insert(PlotSource::default());
}
