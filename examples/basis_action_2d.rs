mod common;

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_rapier2d::prelude::*;
use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinCrouchState};
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaGhostPlatform, TnuaGhostSensor, TnuaProximitySensor, TnuaToggle};

use self::common::tuning::CharacterMotionConfigForPlatformerExample;
use self::common::ui::{CommandAlteringSelectors, ExampleUiPhysicsBackendActive};
use self::common::ui_plotting::PlotSource;
use self::common::{FallingThroughControlScheme, MovingPlatform};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugins(RapierDebugRenderPlugin::default());
    app.add_plugins(TnuaRapier2dPlugin);
    app.add_plugins(TnuaControllerPlugin);
    app.add_plugins(TnuaCrouchEnforcerPlugin);
    app.add_plugins(common::ui::ExampleUi::<
        CharacterMotionConfigForPlatformerExample,
    >::default());
    app.add_systems(Startup, setup_camera);
    app.add_systems(Startup, setup_level);
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, apply_controls.in_set(TnuaUserControlsSystemSet));
    app.add_systems(Update, update_plot_data);
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
    app.run();
}

fn update_rapier_physics_active(
    mut rapier_config: ResMut<RapierConfiguration>,
    setting_from_ui: Res<ExampleUiPhysicsBackendActive>,
) {
    rapier_config.physics_pipeline_active = setting_from_ui.0;
}

fn update_plot_data(mut query: Query<(&mut PlotSource, &Transform, &Velocity)>) {
    for (mut plot_source, transform, velocity) in query.iter_mut() {
        plot_source.set(&[
            &[("Y", transform.translation.y), ("vel-Y", velocity.linvel.y)],
            &[("X", transform.translation.x), ("vel-X", velocity.linvel.x)],
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
    cmd.insert(Collider::halfspace(Vec2::Y).unwrap());

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
        cmd.insert(Collider::cuboid(0.5 * width, 0.5 * height));
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
        cmd.insert(Collider::cuboid(3.0, 0.25));
        cmd.insert(SolverGroups {
            memberships: Group::empty(),
            filters: Group::empty(),
        });
        cmd.insert(TnuaGhostPlatform);
    }

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(10.0, 2.0, 0.0)),
        Collider::ball(1.0),
        CollisionGroups {
            memberships: Group::GROUP_1,
            filters: Group::GROUP_1,
        },
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

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(15.0, 2.0, 0.0)),
        Collider::ball(1.0),
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

    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(20.0, 2.0, 0.0)),
        Collider::ball(1.0),
        Sensor,
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
        cmd.insert(Collider::cuboid(2.0, 0.5));
        cmd.insert(Velocity::default());
        cmd.insert(RigidBody::KinematicVelocityBased);
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
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(Collider::capsule_y(0.5, 0.5));
    cmd.insert(TnuaRapier2dIOBundle::default());
    cmd.insert(TnuaControllerBundle::default());
    cmd.insert(CharacterMotionConfigForPlatformerExample {
        speed: 40.0,
        walk: TnuaBuiltinWalk {
            float_height: 2.0,
            ..Default::default()
        },
        jump: TnuaBuiltinJump {
            height: 4.0,
            ..Default::default()
        },
        crouch: TnuaBuiltinCrouch {
            float_offset: -0.9,
            ..Default::default()
        },
    });
    cmd.insert(TnuaToggle::default());
    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vec3::Y, |cmd| {
        cmd.insert(TnuaRapier2dSensorShape(Collider::cuboid(0.5, 0.0)));
    }));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(FallingThroughControlScheme::default());
    cmd.insert({
        CommandAlteringSelectors::default()
            .with_combo(
                "Sensor Shape",
                1,
                &[
                    ("Point", |mut cmd| {
                        cmd.remove::<TnuaRapier2dSensorShape>();
                    }),
                    ("Flat (underfit)", |mut cmd| {
                        cmd.insert(TnuaRapier2dSensorShape(Collider::cuboid(0.49, 0.0)));
                    }),
                    ("Flat (exact)", |mut cmd| {
                        cmd.insert(TnuaRapier2dSensorShape(Collider::cuboid(0.5, 0.0)));
                    }),
                    ("Ball (underfit)", |mut cmd| {
                        cmd.insert(TnuaRapier2dSensorShape(Collider::ball(0.49)));
                    }),
                    ("Ball (exact)", |mut cmd| {
                        cmd.insert(TnuaRapier2dSensorShape(Collider::ball(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", false, |mut cmd, lock_tilt| {
                if lock_tilt {
                    cmd.insert(LockedAxes::ROTATION_LOCKED);
                } else {
                    cmd.insert(LockedAxes::empty());
                }
            })
            .with_checkbox(
                "Use Collision Groups",
                false,
                |mut cmd, use_collision_groups| {
                    if use_collision_groups {
                        cmd.insert(CollisionGroups {
                            memberships: Group::GROUP_2,
                            filters: Group::GROUP_2,
                        });
                    } else {
                        cmd.remove::<CollisionGroups>();
                    }
                },
            )
            .with_checkbox("Use Solver Groups", false, |mut cmd, use_solver_groups| {
                if use_solver_groups {
                    cmd.insert(SolverGroups {
                        memberships: Group::GROUP_2,
                        filters: Group::GROUP_2,
                    });
                } else {
                    cmd.remove::<SolverGroups>();
                }
            })
    });
    cmd.insert(common::ui::TrackedEntity("Player".to_owned()));
    cmd.insert(PlotSource::default());
}

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

    let turn_in_place = keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

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
    ) in query.iter_mut()
    {
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
            desired_velocity: if turn_in_place {
                Vec3::ZERO
            } else {
                direction * speed_factor * config.speed
            },
            desired_forward: direction.normalize_or_zero(),
            ..config.walk.clone()
        });

        if crouch {
            controller.action(crouch_enforcer.enforcing(config.crouch.clone()));
        }

        if jump {
            controller.action(config.jump.clone());
        }
    }
}
