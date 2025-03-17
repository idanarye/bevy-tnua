#[cfg(feature = "avian2d")]
use avian2d::{prelude as avian, prelude::*};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
use bevy_tnua::control_helpers::TnuaCrouchEnforcer;
use bevy_tnua::control_helpers::TnuaSimpleAirActionsCounter;
use bevy_tnua::control_helpers::TnuaSimpleFallThroughPlatformsHelper;
use bevy_tnua::math::Vector2;
#[allow(unused_imports)]
use bevy_tnua::math::{float_consts, AsF32, Vector3};
use bevy_tnua::prelude::*;
use bevy_tnua::TnuaGhostSensor;
use bevy_tnua::TnuaToggle;
#[cfg(feature = "avian2d")]
use bevy_tnua_avian2d::*;
#[cfg(feature = "rapier2d")]
use bevy_tnua_rapier2d::*;

use tnua_demos_crate::app_setup_options::{AppSetupConfiguration, ScheduleToUse};
use tnua_demos_crate::character_control_systems::info_dumpeing_systems::character_control_info_dumping_system;
use tnua_demos_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerDemo, FallingThroughControlScheme,
    JustPressedCachePlugin,
};
use tnua_demos_crate::character_control_systems::Dimensionality;
use tnua_demos_crate::level_mechanics::LevelMechanicsPlugin;
#[cfg(feature = "avian2d")]
use tnua_demos_crate::levels_setup::for_2d_platformer::LayerNames;
use tnua_demos_crate::levels_setup::level_switching::LevelSwitchingPlugin;
use tnua_demos_crate::levels_setup::IsPlayer;
use tnua_demos_crate::ui::component_alterbation::CommandAlteringSelectors;
use tnua_demos_crate::ui::info::InfoSource;
#[cfg(feature = "egui")]
use tnua_demos_crate::ui::plotting::PlotSource;
use tnua_demos_crate::ui::DemoInfoUpdateSystemSet;
use tnua_demos_crate::ui::DemoUiPhysicsBackendSettings;

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
                app.add_plugins(TnuaRapier2dPlugin::default());
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
        app.add_plugins(PhysicsDebugPlugin::default());
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
            app.add_plugins(TnuaControllerPlugin::default());
        }
        ScheduleToUse::FixedUpdate => {
            app.add_plugins(TnuaControllerPlugin::new(FixedUpdate));
        }
    }

    #[cfg(feature = "egui")]
    app.add_systems(
        Update,
        character_control_info_dumping_system.in_set(DemoInfoUpdateSystemSet),
    );
    app.add_plugins(tnua_demos_crate::ui::DemoUi::<
        CharacterMotionConfigForPlatformerDemo,
    >::default());
    app.add_systems(Startup, setup_camera_and_lights);
    app.add_plugins({
        LevelSwitchingPlugin::new(app_setup_configuration.level_to_load.as_ref()).with(
            "Default",
            tnua_demos_crate::levels_setup::for_2d_platformer::setup_level,
        )
    });
    app.add_systems(Startup, setup_player);
    app.add_systems(
        match app_setup_configuration.schedule_to_use {
            ScheduleToUse::Update => Update.intern(),
            ScheduleToUse::FixedUpdate => FixedUpdate.intern(),
        },
        apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
    );
    app.add_plugins((LevelMechanicsPlugin, JustPressedCachePlugin));
    app.add_systems(
        Startup,
        (
            |mut settings: ResMut<DemoUiPhysicsBackendSettings>| {
                settings.gravity = Vector3::ZERO;
            },
            #[cfg(feature = "rapier2d")]
            |mut cfg: Single<&mut RapierConfiguration>| {
                cfg.gravity = Vec2::ZERO;
            },
            #[cfg(feature = "avian2d")]
            |mut gravity: ResMut<Gravity>| {
                gravity.0 = Vector2::ZERO;
            },
        ),
    );
    app.run();
}

fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 14.0, 30.0)
            .with_scale((0.05 * Vec2::ONE).extend(1.0))
            .looking_at(Vec3::new(0.0, 14.0, 0.0), Vec3::Y),
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn(IsPlayer);
    cmd.insert(Transform::default());
    cmd.insert(Visibility::default());

    // The character entity must be configured as a dynamic rigid body of the physics backend.
    #[cfg(feature = "rapier2d")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
    }
    #[cfg(feature = "avian2d")]
    {
        cmd.insert(avian::RigidBody::Dynamic);
        cmd.insert(avian::Collider::capsule(0.5, 1.0));
    }

    // `TnuaController` is Tnua's main interface with the user code. Read
    // examples/src/character_control_systems/platformer_control_systems.rs to see how
    // `TnuaController` is used in this example.
    cmd.insert(TnuaController::default());

    cmd.insert(CharacterMotionConfigForPlatformerDemo {
        dimensionality: Dimensionality::TopDown,
        speed: 40.0,
        walk: TnuaBuiltinWalk {
            float_height: 2.0,
            max_slope: float_consts::FRAC_PI_4,
            ..Default::default()
        },
        actions_in_air: Default::default(),
        jump: Default::default(),
        crouch: Default::default(),
        dash_distance: 10.0,
        dash: Default::default(),
        one_way_platforms_min_proximity: Default::default(),
        falling_through: FallingThroughControlScheme::SingleFall,
        knockback: Default::default(),
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
                        #[cfg(feature = "avian2d")]
                        cmd.remove::<TnuaAvian2dSensorShape>();
                    }),
                    ("Flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.49, 0.0)));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::rectangle(
                            0.99, 0.0,
                        )));
                    }),
                    ("Flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::rectangle(1.0, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.51, 0.0)));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::rectangle(
                            1.01, 0.0,
                        )));
                    }),
                    ("Ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "avian2d")]
                        cmd.insert(TnuaAvian2dSensorShape(avian::Collider::circle(0.49)));
                    }),
                    ("Ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier2d")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.5)));
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
                        });
                    } else {
                        cmd.insert(CollisionGroups {
                            memberships: Group::ALL,
                            filters: Group::ALL,
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

    // These are not used in top-down view, but the apply_platformer_controls needs them to exist.
    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vector3::Y, |_| {}));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(TnuaSimpleAirActionsCounter::default());

    #[cfg(feature = "egui")]
    cmd.insert((
        tnua_demos_crate::ui::TrackedEntity("Player".to_owned()),
        PlotSource::default(),
        InfoSource::default(),
    ));
}
