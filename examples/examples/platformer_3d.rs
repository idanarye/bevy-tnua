use bevy::prelude::*;
#[cfg(feature = "rapier")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
use bevy_tnua::builtins::TnuaBuiltinCrouch;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaAnimatingState, TnuaGhostPlatform, TnuaGhostSensor, TnuaToggle};
#[cfg(feature = "rapier")]
use bevy_tnua_rapier3d::*;
#[cfg(feature = "xpbd")]
use bevy_tnua_xpbd3d::*;
#[cfg(feature = "xpbd")]
use bevy_xpbd_3d::{prelude as xpbd, prelude::*};

use tnua_examples_crate::character_animating_systems::platformer_animating_systems::{
    animate_platformer_character, AnimationState,
};
use tnua_examples_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerExample,
};
use tnua_examples_crate::character_control_systems::Dimensionality;
use tnua_examples_crate::ui::{CommandAlteringSelectors, ExampleUiPhysicsBackendActive};
use tnua_examples_crate::ui_plotting::PlotSource;
use tnua_examples_crate::util::animating::{animation_patcher_system, GltfSceneHandler};
use tnua_examples_crate::{FallingThroughControlScheme, MovingPlatform};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    #[cfg(feature = "rapier")]
    {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(TnuaRapier3dPlugin);
    }
    #[cfg(feature = "xpbd")]
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
    app.add_systems(Startup, setup_level);
    app.add_systems(Startup, setup_player);
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
    );
    app.add_systems(Update, animation_patcher_system);
    app.add_systems(Update, animate_platformer_character);
    #[cfg(feature = "rapier")]
    {
        app.add_systems(Update, update_plot_data_from_rapier);
        app.add_systems(
            Update,
            MovingPlatform::make_system(|velocity: &mut Velocity, linvel: Vec3| {
                velocity.linvel = linvel;
            })
            .before(TnuaPipelineStages::Sensors),
        );
        app.add_systems(Update, update_rapier_physics_active);
    }
    #[cfg(feature = "xpbd")]
    {
        app.add_systems(Update, update_plot_data_from_xpbd);
        app.add_systems(
            Update,
            MovingPlatform::make_system(|velocity: &mut LinearVelocity, linvel: Vec3| {
                velocity.0 = linvel;
            })
            .before(TnuaPipelineStages::Sensors),
        );
        app.add_systems(Update, update_xpbd_physics_active);
    }
    app.run();
}

#[cfg(feature = "rapier")]
fn update_rapier_physics_active(
    mut rapier_config: ResMut<RapierConfiguration>,
    setting_from_ui: Res<ExampleUiPhysicsBackendActive>,
) {
    rapier_config.physics_pipeline_active = setting_from_ui.0;
}

#[cfg(feature = "rapier")]
fn update_plot_data_from_rapier(mut query: Query<(&mut PlotSource, &Transform, &Velocity)>) {
    for (mut plot_source, transform, velocity) in query.iter_mut() {
        plot_source.set(&[
            &[("Y", transform.translation.y), ("vel-Y", velocity.linvel.y)],
            &[("X", transform.translation.x), ("vel-X", velocity.linvel.x)],
        ]);
    }
}

#[cfg(feature = "xpbd")]
#[derive(PhysicsLayer)]
enum LayerNames {
    Player,
    FallThrough,
    PhaseThrough,
}

#[cfg(feature = "xpbd")]
fn update_xpbd_physics_active(
    mut physics_time: ResMut<Time<Physics>>,
    setting_from_ui: Res<ExampleUiPhysicsBackendActive>,
) {
    if setting_from_ui.0 {
        physics_time.unpause();
    } else {
        physics_time.pause();
    }
}

#[cfg(feature = "xpbd")]
fn update_plot_data_from_xpbd(mut query: Query<(&mut PlotSource, &Transform, &LinearVelocity)>) {
    for (mut plot_source, transform, linear_velocity) in query.iter_mut() {
        plot_source.set(&[
            &[
                ("Y", transform.translation.y),
                ("vel-Y", linear_velocity.0.y),
            ],
            &[
                ("X", transform.translation.x),
                ("vel-X", linear_velocity.0.x),
            ],
        ]);
    }
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

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {
            size: 128.0,
            subdivisions: 0,
        })),
        material: materials.add(Color::WHITE.into()),
        ..Default::default()
    });
    #[cfg(feature = "rapier")]
    cmd.insert(rapier::Collider::halfspace(Vec3::Y).unwrap());
    #[cfg(feature = "xpbd")]
    {
        cmd.insert(xpbd::RigidBody::Static);
        cmd.insert(xpbd::Collider::halfspace(Vec3::Y));
    }

    let obstacles_material = materials.add(Color::GRAY.into());
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
            mesh: meshes.add(Mesh::from(shape::Box::new(width, height, depth))),
            material: obstacles_material.clone(),
            transform,
            ..Default::default()
        });
        #[cfg(feature = "rapier")]
        cmd.insert(rapier::Collider::cuboid(
            0.5 * width,
            0.5 * height,
            0.5 * depth,
        ));
        #[cfg(feature = "xpbd")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::cuboid(width, height, depth));
        }
    }

    // Fall-through platforms
    let fall_through_obstacles_material = materials.add(Color::PINK.with_a(0.8).into());
    for y in [2.0, 4.5] {
        let mut cmd = commands.spawn_empty();
        cmd.insert(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box::new(6.0, 0.5, 2.0))),
            material: fall_through_obstacles_material.clone(),
            transform: Transform::from_xyz(6.0, y, 10.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier")]
        {
            cmd.insert(rapier::Collider::cuboid(3.0, 0.25, 1.0));
            cmd.insert(SolverGroups {
                memberships: Group::empty(),
                filters: Group::empty(),
            });
        }
        #[cfg(feature = "xpbd")]
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
        #[cfg(feature = "rapier")]
        (
            rapier::Collider::cuboid(2.0, 1.0, 2.0),
            CollisionGroups {
                memberships: Group::GROUP_1,
                filters: Group::GROUP_1,
            },
        ),
        #[cfg(feature = "xpbd")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::cuboid(4.0, 2.0, 4.0),
            CollisionLayers::new([LayerNames::PhaseThrough], [LayerNames::PhaseThrough]),
        ),
    ));

    #[cfg(feature = "rapier")]
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
        #[cfg(feature = "rapier")]
        (rapier::Collider::cuboid(2.0, 1.0, 2.0), rapier::Sensor),
        #[cfg(feature = "xpbd")]
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
            mesh: meshes.add(Mesh::from(shape::Box::new(4.0, 1.0, 4.0))),
            material: materials.add(Color::BLUE.into()),
            transform: Transform::from_xyz(-4.0, 6.0, 0.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier")]
        {
            cmd.insert(rapier::Collider::cuboid(2.0, 0.5, 2.0));
            cmd.insert(Velocity::default());
            cmd.insert(rapier::RigidBody::KinematicVelocityBased);
        }
        #[cfg(feature = "xpbd")]
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
            mesh: meshes.add(Mesh::from(shape::Cylinder {
                radius: 3.0,
                height: 1.0,
                resolution: 10,
                segments: 10,
            })),
            material: materials.add(Color::BLUE.into()),
            transform: Transform::from_xyz(-2.0, 2.0, 10.0),
            ..Default::default()
        });
        #[cfg(feature = "rapier")]
        {
            cmd.insert(rapier::Collider::cylinder(0.5, 3.0));
            cmd.insert(Velocity::angular(Vec3::Y));
            cmd.insert(rapier::RigidBody::KinematicVelocityBased);
        }
        #[cfg(feature = "xpbd")]
        {
            cmd.insert(xpbd::Collider::cylinder(1.0, 3.0));
            cmd.insert(AngularVelocity(Vec3::Y));
            cmd.insert(xpbd::RigidBody::Kinematic);
        }
    }
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
    #[cfg(feature = "rapier")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
        cmd.insert(TnuaRapier3dIOBundle::default());
    }
    #[cfg(feature = "xpbd")]
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
        #[cfg(feature = "rapier")]
        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
            0.0, 0.5,
        )));
        #[cfg(feature = "xpbd")]
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
                        #[cfg(feature = "rapier")]
                        cmd.remove::<TnuaRapier3dSensorShape>();
                        #[cfg(feature = "xpbd")]
                        cmd.remove::<TnuaXpbd3dSensorShape>();
                    }),
                    ("flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.49,
                        )));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.49)));
                    }),
                    ("flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.5,
                        )));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.5)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.51,
                        )));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.51)));
                    }),
                    ("ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::ball(0.49)));
                    }),
                    ("ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::ball(0.5)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::ball(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", true, |mut cmd, lock_tilt| {
                if lock_tilt {
                    #[cfg(feature = "rapier")]
                    cmd.insert(
                        rapier::LockedAxes::ROTATION_LOCKED_X
                            | rapier::LockedAxes::ROTATION_LOCKED_Z,
                    );
                    #[cfg(feature = "xpbd")]
                    cmd.insert(xpbd::LockedAxes::new().lock_rotation_x().lock_rotation_z());
                } else {
                    #[cfg(feature = "rapier")]
                    cmd.insert(rapier::LockedAxes::empty());
                    #[cfg(feature = "xpbd")]
                    cmd.insert(xpbd::LockedAxes::new());
                }
            })
            .with_checkbox(
                "Phase Through Collision Groups",
                true,
                |mut cmd, use_collision_groups| {
                    #[cfg(feature = "rapier")]
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
                    #[cfg(feature = "xpbd")]
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
        #[cfg(feature = "rapier")]
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
