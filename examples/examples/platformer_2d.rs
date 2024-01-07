use bevy::prelude::*;
use bevy_egui::EguiContexts;
#[cfg(feature = "rapier")]
use bevy_rapier2d::{prelude as rapier, prelude::*};
use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash};
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaGhostPlatform, TnuaGhostSensor, TnuaProximitySensor, TnuaToggle};
#[cfg(feature = "rapier")]
use bevy_tnua_rapier2d::*;
#[cfg(feature = "xpbd")]
use bevy_tnua_xpbd2d::*;
#[cfg(feature = "xpbd")]
use bevy_xpbd_2d::{prelude as xpbd, prelude::*};

use tnua_examples_crate::tuning::CharacterMotionConfigForPlatformerExample;
use tnua_examples_crate::ui::{CommandAlteringSelectors, ExampleUiPhysicsBackendActive};
use tnua_examples_crate::ui_plotting::PlotSource;
use tnua_examples_crate::{FallingThroughControlScheme, MovingPlatform};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    #[cfg(feature = "rapier")]
    {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        app.add_plugins(RapierDebugRenderPlugin::default());
        app.add_plugins(TnuaRapier2dPlugin);
    }
    #[cfg(feature = "xpbd")]
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
    app.add_systems(Startup, setup_level);
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, apply_controls.in_set(TnuaUserControlsSystemSet));
    #[cfg(feature = "rapier")]
    {
        app.add_systems(Update, update_plot_data_from_rapier);
        app.add_systems(
            Update,
            MovingPlatform::make_system(|velocity: &mut Velocity, linvel: Vec3| {
                velocity.linvel = linvel.truncate();
            })
            .before(TnuaPipelineStages::Sensors),
        );
        app.add_systems(Startup, |mut cfg: ResMut<RapierConfiguration>| {
            cfg.gravity = Vec2::Y * -9.81;
        });
        app.add_systems(Update, update_rapier_physics_active);
    }
    #[cfg(feature = "xpbd")]
    {
        app.add_systems(Update, update_plot_data_from_xpbd);
        app.add_systems(
            Update,
            MovingPlatform::make_system(|velocity: &mut LinearVelocity, linvel: Vec3| {
                velocity.0 = linvel.truncate();
            })
            .before(TnuaPipelineStages::Sensors),
        );
        app.add_systems(Startup, |mut gravity: ResMut<Gravity>| {
            gravity.0 = Vec2::Y * -9.81;
        });
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

fn setup_level(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(128.0, 0.5)),
            color: Color::GRAY,
            ..Default::default()
        },
        ..Default::default()
    });
    #[cfg(feature = "rapier")]
    cmd.insert(rapier::Collider::halfspace(Vec2::Y).unwrap());
    #[cfg(feature = "xpbd")]
    {
        cmd.insert(xpbd::RigidBody::Static);
        cmd.insert(xpbd::Collider::halfspace(Vec2::Y));
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
        #[cfg(feature = "rapier")]
        cmd.insert(rapier::Collider::cuboid(0.5 * width, 0.5 * height));
        #[cfg(feature = "xpbd")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::cuboid(width, height));
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
        #[cfg(feature = "rapier")]
        {
            cmd.insert(rapier::Collider::cuboid(3.0, 0.25));
            cmd.insert(SolverGroups {
                memberships: Group::empty(),
                filters: Group::empty(),
            });
        }
        #[cfg(feature = "xpbd")]
        {
            cmd.insert(xpbd::RigidBody::Static);
            cmd.insert(xpbd::Collider::cuboid(6.0, 0.5));
            cmd.insert(CollisionLayers::new(
                [LayerNames::FallThrough],
                [LayerNames::FallThrough],
            ));
        }
        cmd.insert(TnuaGhostPlatform);
    }

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(10.0, 2.0, 0.0)),
        #[cfg(feature = "rapier")]
        (
            rapier::Collider::ball(1.0),
            CollisionGroups {
                memberships: Group::GROUP_1,
                filters: Group::GROUP_1,
            },
        ),
        #[cfg(feature = "xpbd")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::ball(1.0),
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
        .with_alignment(TextAlignment::Center),
        transform: Transform::from_xyz(10.0, 2.0, 1.0).with_scale(0.01 * Vec3::ONE),
        ..Default::default()
    });

    #[cfg(feature = "rapier")]
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
            .with_alignment(TextAlignment::Center),
            transform: Transform::from_xyz(15.0, 2.0, 1.0).with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        });
    }

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(20.0, 2.0, 0.0)),
        #[cfg(feature = "rapier")]
        (rapier::Collider::ball(1.0), rapier::Sensor),
        #[cfg(feature = "xpbd")]
        (
            xpbd::RigidBody::Static,
            xpbd::Collider::ball(1.0),
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
        .with_alignment(TextAlignment::Center),
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
        #[cfg(feature = "rapier")]
        {
            cmd.insert(rapier::Collider::cuboid(2.0, 0.5));
            cmd.insert(Velocity::default());
            cmd.insert(rapier::RigidBody::KinematicVelocityBased);
        }
        #[cfg(feature = "xpbd")]
        {
            cmd.insert(xpbd::Collider::cuboid(4.0, 1.0));
            cmd.insert(xpbd::RigidBody::Kinematic);
        }
        cmd.insert(MovingPlatform::new(
            4.0,
            &[
                Vec3::new(-4.0, 6.0, 0.0),
                Vec3::new(-8.0, 6.0, 0.0),
                Vec3::new(-8.0, 10.0, 0.0),
                Vec3::new(-4.0, 10.0, 0.0),
            ],
        ));
    }
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(TransformBundle::from_transform(Transform::from_xyz(
        0.0, 2.0, 0.0,
    )));
    cmd.insert(VisibilityBundle::default());
    #[cfg(feature = "rapier")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
        cmd.insert(TnuaRapier2dIOBundle::default());
    }
    #[cfg(feature = "xpbd")]
    {
        cmd.insert(xpbd::RigidBody::Dynamic);
        cmd.insert(xpbd::Collider::capsule(1.0, 0.5));
    }
    cmd.insert(TnuaControllerBundle::default());
    cmd.insert(CharacterMotionConfigForPlatformerExample {
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
        #[cfg(feature = "rapier")]
        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
        #[cfg(feature = "xpbd")]
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
                        #[cfg(feature = "rapier")]
                        cmd.remove::<TnuaRapier2dSensorShape>();
                        #[cfg(feature = "xpbd")]
                        cmd.remove::<TnuaXpbd2dSensorShape>();
                    }),
                    ("Flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.49, 0.0)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(0.99, 0.0)));
                    }),
                    ("Flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.5, 0.0)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(1.0, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::cuboid(0.51, 0.0)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::cuboid(1.01, 0.0)));
                    }),
                    ("Ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::ball(0.49)));
                    }),
                    ("Ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier")]
                        cmd.insert(TnuaRapier2dSensorShape(rapier::Collider::ball(0.5)));
                        #[cfg(feature = "xpbd")]
                        cmd.insert(TnuaXpbd2dSensorShape(xpbd::Collider::ball(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", false, |mut cmd, lock_tilt| {
                if lock_tilt {
                    #[cfg(feature = "rapier")]
                    cmd.insert(rapier::LockedAxes::ROTATION_LOCKED);
                    #[cfg(feature = "xpbd")]
                    cmd.insert(xpbd::LockedAxes::new().lock_rotation());
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
                    #[cfg(feature = "rapier")]
                    cmd.insert(SolverGroups {
                        memberships: Group::GROUP_2,
                        filters: Group::GROUP_2,
                    });
                } else {
                    #[cfg(feature = "rapier")]
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

#[allow(clippy::type_complexity)]
fn apply_controls(
    mut egui_context: EguiContexts,
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<(
        &CharacterMotionConfigForPlatformerExample,
        &mut TnuaController,
        &mut TnuaCrouchEnforcer,
        &mut TnuaProximitySensor,
        &TnuaGhostSensor,
        &mut TnuaSimpleFallThroughPlatformsHelper,
        &FallingThroughControlScheme,
        &mut TnuaSimpleAirActionsCounter,
    )>,
) {
    if egui_context.ctx_mut().wants_keyboard_input() {
        for (_, mut controller, ..) in query.iter_mut() {
            controller.neutralize_basis();
        }
        return;
    }

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::Left) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::Right) {
        direction += Vec3::X;
    }

    let jump = keyboard.any_pressed([KeyCode::Space, KeyCode::Up]);
    let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    let crouch_buttons = [KeyCode::Down, KeyCode::ControlLeft, KeyCode::ControlRight];
    let crouch = keyboard.any_pressed(crouch_buttons);
    let crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);

    for (
        config,
        mut controller,
        mut crouch_enforcer,
        mut sensor,
        ghost_sensor,
        mut fall_through_helper,
        falling_through_control_scheme,
        mut air_actions_counter,
    ) in query.iter_mut()
    {
        air_actions_counter.update(controller.as_mut());

        let crouch = falling_through_control_scheme.perform_and_check_if_still_crouching(
            crouch,
            crouch_just_pressed,
            fall_through_helper.as_mut(),
            sensor.as_mut(),
            ghost_sensor,
            1.0,
        );

        let speed_factor =
            if let Some((_, state)) = controller.concrete_action::<TnuaBuiltinCrouch>() {
                if matches!(state, TnuaBuiltinCrouchState::Rising) {
                    1.0
                } else {
                    0.2
                }
            } else {
                1.0
            };

        controller.basis(TnuaBuiltinWalk {
            desired_velocity: direction * speed_factor * config.speed,
            ..config.walk.clone()
        });

        if crouch {
            controller.action(crouch_enforcer.enforcing(config.crouch.clone()));
        }

        if jump {
            controller.action(TnuaBuiltinJump {
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinJump::NAME)
                    <= config.actions_in_air,
                ..config.jump.clone()
            });
        }

        if dash {
            controller.action(TnuaBuiltinDash {
                displacement: direction.normalize() * config.dash_distance,
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinDash::NAME)
                    <= config.actions_in_air,
                ..config.dash.clone()
            });
        }
    }
}
