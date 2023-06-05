mod common;

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_rapier2d::prelude::*;
use bevy_tnua::{
    TnuaFreeFallBehavior, TnuaGhostPlatform, TnuaGhostSensor, TnuaKeepCrouchingBelowObstacles,
    TnuaManualTurningOutput, TnuaPlatformerBundle, TnuaPlatformerConfig, TnuaPlatformerControls,
    TnuaPlatformerPlugin, TnuaProximitySensor, TnuaRapier2dIOBundle, TnuaRapier2dPlugin,
    TnuaRapier2dSensorShape, TnuaSystemSet, TnuaUserControlsSystemSet,
};

use self::common::ui::{CommandAlteringSelectors, ExampleUiTnuaActive};
use self::common::ui_plotting::PlotSource;
use self::common::MovingPlatform;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugin(RapierDebugRenderPlugin::default());
    app.add_plugin(TnuaRapier2dPlugin);
    app.add_plugin(TnuaPlatformerPlugin);
    app.add_plugin(common::ui::ExampleUi);
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_level);
    app.add_startup_system(setup_player);
    app.add_system(apply_controls.in_set(TnuaUserControlsSystemSet));
    app.add_system(apply_manual_turning);
    app.add_system(update_plot_data);
    app.add_system(MovingPlatform::make_system(
        |velocity: &mut Velocity, linvel: Vec3| {
            velocity.linvel = linvel.truncate();
        },
    ));
    app.add_startup_system(|mut cfg: ResMut<RapierConfiguration>| {
        cfg.gravity = Vec2::Y * -9.81;
    });
    app.configure_set(TnuaSystemSet.run_if(|tnua_active: Res<ExampleUiTnuaActive>| tnua_active.0));
    app.run();
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
    for y in [5.0, 8.0] {
        let mut cmd = commands.spawn_empty();
        cmd.insert(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(6.0, 1.0)),
                color: Color::PINK,
                ..Default::default()
            },
            transform: Transform::from_xyz(-20.0, y, -1.0),
            ..Default::default()
        });
        cmd.insert(Collider::cuboid(3.0, 0.5));
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

#[derive(Component)]
struct TurningVisualizer {
    x_multiplier: f32,
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(TransformBundle::from_transform(Transform::from_xyz(
        0.0, 2.0, 0.0,
    )));
    cmd.with_children(|commands| {
        let mut cmd = commands.spawn_empty();
        cmd.insert(SpriteBundle {
            sprite: Sprite {
                color: Color::YELLOW,
                custom_size: Some(Vec2::new(0.4, 0.3)),
                ..Default::default()
            },
            transform: Transform::from_xyz(0.4, 0.6, 1.0),
            ..Default::default()
        });
        cmd.insert(TurningVisualizer { x_multiplier: 0.4 });
    });
    cmd.insert(VisibilityBundle::default());
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(Collider::capsule_y(0.5, 0.5));
    cmd.insert(TnuaRapier2dIOBundle::default());
    cmd.insert(TnuaPlatformerBundle {
        config: TnuaPlatformerConfig {
            full_speed: 40.0,
            full_jump_height: 4.0,
            up: Vec3::Y,
            forward: Vec3::X,
            float_height: 2.0,
            cling_distance: 1.0,
            spring_strengh: 40.0,
            spring_dampening: 0.4,
            acceleration: 60.0,
            air_acceleration: 20.0,
            coyote_time: 0.15,
            jump_input_buffer_time: 0.2,
            held_jump_cooldown: None,
            upslope_jump_extra_gravity: 30.0,
            jump_takeoff_extra_gravity: 30.0,
            jump_takeoff_above_velocity: 2.0,
            jump_fall_extra_gravity: 20.0,
            jump_shorten_extra_gravity: 40.0,
            jump_peak_prevention_at_upward_velocity: 1.0,
            jump_peak_prevention_extra_gravity: 20.0,
            free_fall_behavior: TnuaFreeFallBehavior::LikeJumpShorten,
            tilt_offset_angvel: 5.0,
            tilt_offset_angacl: 500.0,
            turning_angvel: 10.0,
            height_change_impulse_for_duration: 0.04,
            height_change_impulse_limit: 40.0,
        },
        ..Default::default()
    });
    cmd.insert(TnuaKeepCrouchingBelowObstacles::new(1.5, |cmd| {
        cmd.insert(TnuaRapier2dSensorShape(Collider::cuboid(0.5, 0.0)));
    }));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaManualTurningOutput::default());
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
        &mut TnuaPlatformerControls,
        &TnuaKeepCrouchingBelowObstacles,
        &mut TnuaProximitySensor,
        &TnuaGhostSensor,
    )>,
) {
    if egui_context.ctx_mut().wants_keyboard_input() {
        for (mut controls, ..) in query.iter_mut() {
            *controls = Default::default();
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

    let jump = [KeyCode::Space, KeyCode::Up]
        .into_iter()
        .any(|key_code| keyboard.pressed(key_code));

    let turn_in_place = [KeyCode::LAlt, KeyCode::RAlt]
        .into_iter()
        .any(|key_code| keyboard.pressed(key_code));

    let crouch = [KeyCode::Down, KeyCode::LControl, KeyCode::RControl]
        .into_iter()
        .any(|key_code| keyboard.pressed(key_code));

    for (mut controls, keep_crouching, mut sensor, ghost_sensor) in query.iter_mut() {
        let mut crouch = crouch;
        if let Some(ghost_platform) = ghost_sensor.0.first() {
            if 1.0 <= ghost_platform.proximity {
                if crouch {
                    crouch = false;
                } else {
                    sensor.output = Some(ghost_platform.clone());
                }
            }
        }
        let speed_factor = if crouch || keep_crouching.force_crouching_to_height < -0.5 {
            0.2
        } else {
            1.0
        };

        *controls = TnuaPlatformerControls {
            desired_velocity: if turn_in_place {
                Vec3::ZERO
            } else {
                speed_factor * direction
            },
            desired_forward: direction.normalize_or_zero(),
            jump: jump.then(|| 1.0),
            float_height_offset: if crouch { -0.9 } else { 0.0 },
        };
    }
}

fn apply_manual_turning(
    query: Query<(&TnuaManualTurningOutput, &Children)>,
    mut visual_elements: Query<(&TurningVisualizer, &mut Transform)>,
) {
    for (TnuaManualTurningOutput { forward }, children) in query.iter() {
        for child in children.iter() {
            if let Ok((&TurningVisualizer { x_multiplier }, mut transform)) =
                visual_elements.get_mut(*child)
            {
                transform.translation.x = x_multiplier * forward.x;
            }
        }
    }
}
