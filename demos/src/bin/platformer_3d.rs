#[cfg(feature = "avian3d")]
use avian3d::{prelude as avian, prelude::*};
use bevy::prelude::*;
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
use bevy_tnua::control_helpers::{
    TnuaBlipReuseAvoidance, TnuaSimpleAirActionsCounter, TnuaSimpleFallThroughPlatformsHelper,
};
#[allow(unused_imports)]
use bevy_tnua::math::{AsF32, Vector3, float_consts};
use bevy_tnua::{TnuaAnimatingState, TnuaGhostOverwrites, TnuaToggle};
use bevy_tnua::{TnuaObstacleRadar, prelude::*};
#[cfg(feature = "avian3d")]
use bevy_tnua_avian3d::prelude::*;
#[cfg(feature = "rapier3d")]
use bevy_tnua_rapier3d::prelude::*;

use tnua_demos_crate::app_setup_options::{AppSetupConfiguration, ScheduleToUse};
use tnua_demos_crate::character_control_systems::Dimensionality;
#[cfg(feature = "egui")]
use tnua_demos_crate::character_control_systems::info_dumpeing_systems::character_control_info_dumping_system;
use tnua_demos_crate::character_control_systems::info_dumpeing_systems::character_control_radar_visualization_system;
use tnua_demos_crate::character_control_systems::platformer_control_scheme::{
    DemoControlScheme, DemoControlSchemeConfig,
};
use tnua_demos_crate::character_control_systems::platformer_control_systems::{
    CameraControllerFloating, CharacterMotionConfigForPlatformerDemo, FallingThroughControlScheme,
    apply_platformer_controls,
};
use tnua_demos_crate::level_mechanics::LevelMechanicsPlugin;
#[cfg(feature = "avian3d")]
use tnua_demos_crate::levels_setup::for_3d_platformer::LayerNames;
use tnua_demos_crate::levels_setup::level_switching::LevelSwitchingPlugin;
use tnua_demos_crate::levels_setup::{IsPlayer, levels_for_3d};
#[cfg(feature = "egui")]
use tnua_demos_crate::ui::DemoInfoUpdateSystems;
use tnua_demos_crate::ui::component_alterbation::CommandAlteringSelectors;
#[cfg(feature = "egui")]
use tnua_demos_crate::ui::info::InfoSource;
#[cfg(feature = "egui")]
use tnua_demos_crate::ui::plotting::PlotSource;
use tnua_demos_crate::util::animating::{GltfSceneHandler, animation_patcher_system};
use tnua_demos_crate::{
    character_animating_systems::platformer_animating_systems::{
        AnimationState, animate_platformer_character,
    },
    character_control_systems::platformer_control_systems::JustPressedCachePlugin,
};

fn main() {
    tnua_demos_crate::verify_physics_backends_features!("rapier3d", "avian3d");

    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    let app_setup_configuration = AppSetupConfiguration::from_environment();
    app.insert_resource(app_setup_configuration.clone());

    #[cfg(feature = "rapier3d")]
    {
        match app_setup_configuration.schedule_to_use {
            ScheduleToUse::Update => {
                app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
                // To use Tnua with bevy_rapier3d, you need the `TnuaRapier3dPlugin` plugin from
                // bevy-tnua-rapier3d.
                app.add_plugins(TnuaRapier3dPlugin::new(Update));
            }
            ScheduleToUse::FixedUpdate => {
                app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default().in_fixed_schedule());
                app.add_plugins(TnuaRapier3dPlugin::new(FixedUpdate));
            }
        }
    }
    #[cfg(feature = "avian3d")]
    {
        match app_setup_configuration.schedule_to_use {
            ScheduleToUse::Update => {
                app.add_plugins(PhysicsPlugins::new(PostUpdate));
                // To use Tnua with avian3d, you need the `TnuaAvian3dPlugin` plugin from
                // bevy-tnua-avian3d.
                app.add_plugins(TnuaAvian3dPlugin::new(Update));
            }
            ScheduleToUse::FixedUpdate => {
                app.add_plugins(PhysicsPlugins::new(FixedPostUpdate));
                app.add_plugins(TnuaAvian3dPlugin::new(FixedUpdate));
            }
        }
    }

    match app_setup_configuration.schedule_to_use {
        ScheduleToUse::Update => {
            // This is Tnua's main plugin.
            app.add_plugins(TnuaControllerPlugin::<DemoControlScheme>::new(Update));
        }
        ScheduleToUse::FixedUpdate => {
            app.add_plugins(TnuaControllerPlugin::<DemoControlScheme>::new(FixedUpdate));
        }
    }

    #[cfg(feature = "egui")]
    app.add_systems(
        Update,
        character_control_info_dumping_system.in_set(DemoInfoUpdateSystems),
    );
    app.add_systems(Update, character_control_radar_visualization_system);
    app.add_plugins(tnua_demos_crate::ui::DemoUi::<
        DemoControlScheme,
        CharacterMotionConfigForPlatformerDemo,
    >::default());
    app.add_systems(Startup, setup_camera_and_lights);
    app.add_plugins(
        LevelSwitchingPlugin::new(app_setup_configuration.level_to_load.as_ref())
            .with_levels(levels_for_3d),
    );
    app.add_systems(Startup, setup_player);
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystems),
    );
    app.add_systems(Update, animation_patcher_system);
    app.add_systems(Update, animate_platformer_character);
    app.add_systems(Update, apply_camera_transform);
    app.add_plugins((LevelMechanicsPlugin, JustPressedCachePlugin));
    app.run();
}

fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(30.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::default().looking_at(-Vec3::Y, Vec3::Z),
    ));
}

/// Updates the camera trasfrom from the the [`CameraControllerFloating`] component. The
/// [`CameraControllerFloating`] itself is updated via the UI (requires "egui" features).
fn apply_camera_transform(
    camera_controller: Single<&CameraControllerFloating>,
    mut camera_query: Single<&mut Transform, With<Camera>>,
) {
    use core::ops::DerefMut;
    let CameraControllerFloating {
        looking_from: from,
        looking_to: to,
    } = &mut camera_controller.into_inner();
    let camera = camera_query.deref_mut().deref_mut();
    *camera = Transform::from_translation(from.f32()).looking_at(to.f32(), Vec3::Y);
}

fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut control_scheme_config_assets: ResMut<Assets<DemoControlSchemeConfig>>,
) {
    let mut cmd = commands.spawn(IsPlayer);
    cmd.insert(SceneRoot(asset_server.load("player.glb#Scene0")));
    cmd.insert(GltfSceneHandler {
        names_from: asset_server.load("player.glb"),
    });
    cmd.insert(CameraControllerFloating {
        looking_from: Vector3::new(0.0, 16.0, 40.0),
        looking_to: Vector3::new(0.0, 10.0, 0.0),
    });
    // The character entity must be configured as a dynamic rigid body of the physics backend.
    #[cfg(feature = "rapier3d")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
    }
    #[cfg(feature = "avian3d")]
    {
        cmd.insert(avian::RigidBody::Dynamic);
        cmd.insert(avian::Collider::capsule(0.5, 1.0));
    }

    // `TnuaController` is Tnua's main interface with the user code. Read
    // examples/src/character_control_systems/platformer_control_systems.rs to see how
    // `TnuaController` is used in this example.
    cmd.insert(TnuaController::<DemoControlScheme>::new(
        control_scheme_config_assets.add(DemoControlSchemeConfig::new_with_speed(20.0)),
    ));

    // The obstacle radar is used to detect obstacles around the player that the player can use
    // for environment actions (e.g. climbing). The physics backend integration plugin is
    // responsible for generating the collider in a child object. The collider is a cylinder around
    // the player character (it needs to be a little bigger than the character's collider),
    // configured so that it'll generate collision data without generating forces for the actual
    // physics simulation.
    cmd.insert(TnuaObstacleRadar::new(1.0, 3.0));

    // We use the blip reuse avoidance helper to avoid initiating actions on obstacles we've just
    // finished an action with.
    cmd.insert(TnuaBlipReuseAvoidance::<DemoControlScheme>::default());

    cmd.insert(CharacterMotionConfigForPlatformerDemo {
        dimensionality: Dimensionality::Dim3,
        actions_in_air: 1,
        dash_distance: 10.0,
        one_way_platforms_min_proximity: 1.0,
        falling_through: FallingThroughControlScheme::SingleFall,
    });

    // An entity's Tnua behavior can be toggled individually with this component, if inserted.
    cmd.insert(TnuaToggle::default());

    // This is an helper component for deciding which animation to play. Tnua itself does not
    // actually interact with `TnuaAnimatingState` - it's there so that animating systems could use
    // the information from `TnuaController` to animate the character.
    //
    // Read examples/src/character_animating_systems/platformer_animating_systems.rs to see how
    // `TnuaAnimatingState` is used in this example.
    cmd.insert(TnuaAnimatingState::<AnimationState>::default());

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
                    ("no", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.remove::<TnuaRapier3dSensorShape>();
                        #[cfg(feature = "avian3d")]
                        cmd.remove::<TnuaAvian3dSensorShape>();
                    }),
                    ("flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(
                            bevy_rapier3d::parry::shape::SharedShape::cylinder(0.0, 0.49),
                        ));
                        #[cfg(feature = "avian3d")]
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.49, 0.0)));
                    }),
                    ("flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(
                            bevy_rapier3d::parry::shape::SharedShape::cylinder(0.0, 0.5),
                        ));
                        #[cfg(feature = "avian3d")]
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.5, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(
                            bevy_rapier3d::parry::shape::SharedShape::cylinder(0.0, 0.51),
                        ));
                        #[cfg(feature = "avian3d")]
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.51, 0.0)));
                    }),
                    ("ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(
                            bevy_rapier3d::parry::shape::SharedShape::ball(0.49),
                        ));
                        #[cfg(feature = "avian3d")]
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::sphere(0.49)));
                    }),
                    ("ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(
                            bevy_rapier3d::parry::shape::SharedShape::ball(0.5),
                        ));
                        #[cfg(feature = "avian3d")]
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::sphere(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", true, |mut cmd, lock_tilt| {
                // Tnua will automatically apply angular impulses/forces to fix the tilt and make
                // the character stand upward, but it is also possible to just let the physics
                // engine prevent rotation (other than around the Y axis, for turning)
                if lock_tilt {
                    #[cfg(feature = "rapier3d")]
                    cmd.insert(
                        rapier::LockedAxes::ROTATION_LOCKED_X
                            | rapier::LockedAxes::ROTATION_LOCKED_Z,
                    );
                    #[cfg(feature = "avian3d")]
                    cmd.insert(avian::LockedAxes::new().lock_rotation_x().lock_rotation_z());
                } else {
                    #[cfg(feature = "rapier3d")]
                    cmd.insert(rapier::LockedAxes::empty());
                    #[cfg(feature = "avian3d")]
                    cmd.insert(avian::LockedAxes::new());
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
                    #[cfg(feature = "avian3d")]
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

    // The ghost sensor is used for detecting ghost platforms - platforms configured in the physics
    // backend to not contact with the character (or detect the contact but not apply physical
    // forces based on it) and marked with the `TnuaGhostPlatform` component. These can then be
    // used as one-way platforms.
    cmd.insert(TnuaGhostOverwrites::<DemoControlScheme>::default());

    // This helper is used to operate the ghost sensor and ghost platforms and implement
    // fall-through behavior where the player can intentionally fall through a one-way platform.
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());

    // This helper keeps track of air actions like jumps or air dashes.
    cmd.insert(TnuaSimpleAirActionsCounter::<DemoControlScheme>::default());

    #[cfg(feature = "egui")]
    cmd.insert((
        tnua_demos_crate::ui::TrackedEntity("Player".to_owned()),
        PlotSource::default(),
        InfoSource::default(),
    ));
}
