#[cfg(feature = "avian2d")]
use avian2d::{prelude as avian, prelude::*};
use bevy::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
use bevy_tnua::control_helpers::{
    TnuaAirActionsPlugin, TnuaBlipReuseAvoidance, TnuaSimpleFallThroughPlatformsHelper,
};
#[allow(unused_imports)]
use bevy_tnua::math::{AsF32, Vector3, float_consts};
use bevy_tnua::{TnuaGhostOverwrites, TnuaToggle};
use bevy_tnua::{TnuaObstacleRadar, prelude::*};
#[cfg(feature = "avian2d")]
use bevy_tnua_avian2d::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_tnua_rapier2d::prelude::*;

use tnua_demos_crate::app_setup_options::{AppSetupConfiguration, ScheduleToUse};
use tnua_demos_crate::character_control_systems::Dimensionality;
use tnua_demos_crate::character_control_systems::info_dumpeing_systems::{
    character_control_info_dumping_system, character_control_radar_visualization_system,
};
use tnua_demos_crate::character_control_systems::platformer_control_scheme::{
    DemoControlScheme, DemoControlSchemeAirActions, DemoControlSchemeConfig,
};
use tnua_demos_crate::character_control_systems::platformer_control_systems::{
    CharacterMotionConfigForPlatformerDemo, JustPressedCachePlugin, apply_platformer_controls,
};
use tnua_demos_crate::level_mechanics::LevelMechanicsPlugin;
#[cfg(feature = "avian2d")]
use tnua_demos_crate::levels_setup::for_2d_platformer::LayerNames;
use tnua_demos_crate::levels_setup::level_switching::LevelSwitchingPlugin;
use tnua_demos_crate::levels_setup::{IsPlayer, levels_for_2d};
use tnua_demos_crate::ui::DemoInfoUpdateSystems;
use tnua_demos_crate::ui::component_alterbation::CommandAlteringSelectors;
use tnua_demos_crate::ui::info::InfoSource;
#[cfg(feature = "egui")]
use tnua_demos_crate::ui::plotting::PlotSource;

fn main() {
    tnua_demos_crate::verify_physics_backends_features!("rapier2d", "avian2d");

    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let app_setup_configuration = AppSetupConfiguration::from_environment();
    app.insert_resource(app_setup_configuration.clone());

    #[cfg(feature = "rapier2d")]
    {
        app.add_plugins(RapierDebugRenderPlugin::default());
        match app_setup_configuration.schedule_to_use {
            ScheduleToUse::Update => {
                app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
                // To use Tnua with bevy_rapier2d, you need the `TnuaRapier2dPlugin` plugin from
                // bevy-tnua-rapier2d.
                app.add_plugins(TnuaRapier2dPlugin::new(Update));
            }
            ScheduleToUse::FixedUpdate => {
                app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule());
                // To use Tnua with bevy_rapier2d, you need the `TnuaRapier2dPlugin` plugin from
                // bevy-tnua-rapier2d.
                app.add_plugins(TnuaRapier2dPlugin::new(FixedUpdate));
            }
        }
    }
    #[cfg(feature = "avian2d")]
    {
        app.add_plugins(PhysicsDebugPlugin);
        match app_setup_configuration.schedule_to_use {
            ScheduleToUse::Update => {
                app.add_plugins(PhysicsPlugins::new(PostUpdate));
                // To use Tnua with avian2d, you need the `TnuaAvian2dPlugin` plugin from
                // bevy-tnua-avian2d.
                app.add_plugins(TnuaAvian2dPlugin::new(Update));
            }
            ScheduleToUse::FixedUpdate => {
                app.add_plugins(PhysicsPlugins::new(FixedPostUpdate));
                app.add_plugins(TnuaAvian2dPlugin::new(FixedUpdate));
            }
        }
    }

    match app_setup_configuration.schedule_to_use {
        ScheduleToUse::Update => {
            // This is Tnua's main plugin.
            app.add_plugins(TnuaControllerPlugin::<DemoControlScheme>::new(Update));
            // This plugin updates the TnuaActionsCounter.
            app.add_plugins(TnuaAirActionsPlugin::<DemoControlSchemeAirActions>::new(
                Update,
            ));
        }
        ScheduleToUse::FixedUpdate => {
            app.add_plugins(TnuaControllerPlugin::<DemoControlScheme>::new(FixedUpdate));
            app.add_plugins(TnuaAirActionsPlugin::<DemoControlSchemeAirActions>::new(
                FixedUpdate,
            ));
        }
    }

    #[cfg(feature = "egui")]
    app.add_systems(
        Update,
        character_control_info_dumping_system.in_set(DemoInfoUpdateSystems),
    );
    app.add_systems(Update, character_control_radar_visualization_system);
    app.add_plugins(tnua_demos_crate::ui::DemoUi::<DemoControlScheme>::default());
    app.add_systems(Startup, setup_camera_and_lights);
    app.add_plugins(
        LevelSwitchingPlugin::new(app_setup_configuration.level_to_load.as_ref())
            .with_levels(levels_for_2d),
    );
    app.add_systems(Startup, setup_player);
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystems),
    );
    app.add_plugins((LevelMechanicsPlugin, JustPressedCachePlugin));
    #[cfg(feature = "rapier2d")]
    {
        app.add_systems(Startup, |mut cfg: Single<&mut RapierConfiguration>| {
            // For some odd reason, Rapier 2D defaults to a gravity of 98.1
            cfg.gravity = Vec2::Y * -9.81;
        });
    }

    app.add_plugins(tnua_demos_crate::hacks::Register3dResourcesInThe2dDemos);

    app.run();
}

fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn_scene_list(bsn_list! {
        Camera2d
        Transform::from_xyz(0.0, 14.0, 30.0)
            .with_scale((0.05 * Vec2::ONE).extend(1.0))
            .looking_at(Vec3::new(0.0, 14.0, 0.0), Vec3::Y)
        --
        PointLight
        Transform::from_xyz(5.0, 5.0, 5.0)
    });
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn_scene(bsn! {
        IsPlayer
        Transform
        Visibility

        // The character entity must be configured as a dynamic rigid body of the physics backend.
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::RigidBody::Dynamic
                rapier::Collider::capsule_y(0.5, 0.5)
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn!{
                avian::RigidBody::Dynamic
                avian::Collider::capsule(0.5, 1.0)
            }
        }

        // `TnuaController` is Tnua's main interface with the user code. Read
        // examples/src/character_control_systems/platformer_control_systems.rs to see how
        // `TnuaController` is used in this example.
        TnuaController<DemoControlScheme>
        // `TnuaConfig` holds the configuration for the Tnua controller. It can be loaded from a
        // file as an asset, but in this case we are creating it by code and injecting it to the
        // assets resource.
        TnuaConfig<DemoControlScheme>(asset_value(
            DemoControlSchemeConfig {
                ext: CharacterMotionConfigForPlatformerDemo {
                    dimensionality: Dimensionality::Dim2,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))

        // The obstacle radar is used to detect obstacles around the player that the player can use
        // for environment actions (e.g. climbing). The physics backend integration plugin is
        // responsible for generating the collider in a child object. The collider is a cylinder
        // around the player character (it needs to be a little bigger than the character's
        // collider), configured so that it'll generate collision data without generating forces
        // for the actual physics simulation.
        TnuaObstacleRadar{ radius: 1.0, height: 3.0 }

        // We use the blip reuse avoidance helper to avoid initiating actions on obstacles we've
        // just finished an action with.
        TnuaBlipReuseAvoidance<DemoControlScheme>

        // An entity's Tnua behavior can be toggled individually with this component, if inserted.
        TnuaToggle

        // The ghost sensor is used for detecting ghost platforms - platforms configured in the
        // physics backend to not contact with the character (or detect the contact but not apply
        // physical forces based on it) and marked with the `TnuaGhostPlatform` component. These
        // can then be used as one-way platforms.
        TnuaGhostOverwrites<DemoControlScheme>

        // This helper is used to operate the ghost sensor and ghost platforms and implement
        // fall-through behavior where the player can intentionally fall through a one-way
        // platform.
        TnuaSimpleFallThroughPlatformsHelper

        @{
            #[cfg(feature = "egui")]
            bsn! {
                tnua_demos_crate::ui::TrackedEntity("Player")
                PlotSource
                InfoSource
            }
        }
    });
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
                        #[cfg(feature = "avian2d")]
                        cmd.remove::<TnuaAvian2dSensorShape>();
                    }),
                    ("Flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(
                            bevy_rapier2d::parry::shape::SharedShape::cuboid(0.49, 0.0),
                        ));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::rectangle(
                            0.99, 0.0,
                        )));
                    }),
                    ("Flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(
                            bevy_rapier2d::parry::shape::SharedShape::cuboid(0.5, 0.0),
                        ));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::rectangle(1.0, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(
                            bevy_rapier2d::parry::shape::SharedShape::cuboid(0.51, 0.0),
                        ));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::rectangle(
                            1.01, 0.0,
                        )));
                    }),
                    ("Ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(
                            bevy_rapier2d::parry::shape::SharedShape::ball(0.49),
                        ));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::circle(0.49)));
                    }),
                    ("Ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(
                            bevy_rapier2d::parry::shape::SharedShape::ball(0.5),
                        ));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::circle(0.5)));
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
                    #[cfg(feature = "avian2d")]
                    cmd.insert(avian::LockedAxes::new().lock_rotation());
                } else {
                    #[cfg(feature = "rapier2d")]
                    cmd.insert(rapier::LockedAxes::empty());
                    #[cfg(feature = "avian2d")]
                    cmd.insert(avian::LockedAxes::new());
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
                            test_mode: Default::default(),
                        });
                    } else {
                        cmd.insert(CollisionGroups {
                            memberships: Group::ALL,
                            filters: Group::ALL,
                            test_mode: Default::default(),
                        });
                    }
                    #[cfg(feature = "avian2d")]
                    {
                        let player_layers: LayerMask = if use_collision_groups {
                            [LayerNames::Default, LayerNames::Player].into()
                        } else {
                            [
                                LayerNames::Default,
                                LayerNames::Player,
                                LayerNames::PhaseThrough,
                            ]
                            .into()
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
                    cmd.insert(SolverGroups {
                        memberships: Group::GROUP_2,
                        filters: Group::GROUP_2,
                        test_mode: Default::default(),
                    });
                } else {
                    cmd.insert(SolverGroups {
                        memberships: Group::ALL,
                        filters: Group::ALL,
                        test_mode: Default::default(),
                    });
                }
            },
        );
        command_altering_selectors
    });
}
