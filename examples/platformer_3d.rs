mod common;

use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::*;
use bevy_tnua::builtins::{
    TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash, TnuaBuiltinJumpState,
};
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{
    TnuaAnimatingState, TnuaAnimatingStateDirective, TnuaGhostPlatform, TnuaGhostSensor,
    TnuaProximitySensor, TnuaToggle,
};

use self::common::tuning::CharacterMotionConfigForPlatformerExample;
use self::common::ui::{CommandAlteringSelectors, ExampleUiPhysicsBackendActive};
use self::common::ui_plotting::PlotSource;
use self::common::{FallingThroughControlScheme, MovingPlatform};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugins(TnuaRapier3dPlugin);
    app.add_plugins(TnuaControllerPlugin);
    app.add_plugins(TnuaCrouchEnforcerPlugin);
    app.add_plugins(common::ui::ExampleUi::<
        CharacterMotionConfigForPlatformerExample,
    >::default());
    app.add_systems(Startup, setup_camera);
    app.add_systems(Startup, setup_level);
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, apply_controls.in_set(TnuaUserControlsSystemSet));
    app.add_systems(Update, animation_patcher_system);
    app.add_systems(Update, animate);
    app.add_systems(Update, update_plot_data);
    app.add_systems(
        Update,
        MovingPlatform::make_system(|velocity: &mut Velocity, linvel: Vec3| {
            velocity.linvel = linvel;
        })
        .before(TnuaPipelineStages::Sensors),
    );
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
    cmd.insert(Collider::halfspace(Vec3::Y).unwrap());

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
        cmd.insert(Collider::cuboid(0.5 * width, 0.5 * height, 0.5 * depth));
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
        cmd.insert(Collider::cuboid(3.0, 0.25, 1.0));
        cmd.insert(SolverGroups {
            memberships: Group::empty(),
            filters: Group::empty(),
        });
        cmd.insert(TnuaGhostPlatform);
    }

    commands.spawn((
        SceneBundle {
            scene: asset_server.load("collision-groups-text.glb#Scene0"),
            transform: Transform::from_xyz(10.0, 2.0, 1.0), // .with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        },
        Collider::cuboid(2.0, 1.0, 2.0),
        CollisionGroups {
            memberships: Group::GROUP_1,
            filters: Group::GROUP_1,
        },
    ));

    commands.spawn((
        SceneBundle {
            scene: asset_server.load("solver-groups-text.glb#Scene0"),
            transform: Transform::from_xyz(15.0, 2.0, 1.0), // .with_scale(0.01 * Vec3::ONE),
            ..Default::default()
        },
        Collider::cuboid(2.0, 1.0, 2.0),
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
        Collider::cuboid(2.0, 1.0, 2.0),
        Sensor,
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
        cmd.insert(Collider::cuboid(2.0, 0.5, 2.0));
        cmd.insert(Velocity::default());
        cmd.insert(RigidBody::KinematicVelocityBased);
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
        cmd.insert(Collider::cylinder(0.5, 3.0));
        cmd.insert(Velocity::angular(Vec3::Y));
        cmd.insert(RigidBody::KinematicVelocityBased);
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
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(Collider::capsule_y(0.5, 0.5));
    cmd.insert(TnuaRapier3dIOBundle::default());
    cmd.insert(TnuaControllerBundle::default());

    cmd.insert(CharacterMotionConfigForPlatformerExample {
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
        cmd.insert(TnuaRapier3dSensorShape(Collider::cylinder(0.0, 0.5)));
    }));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(TnuaSimpleAirActionsCounter::default());
    cmd.insert(FallingThroughControlScheme::default());
    cmd.insert(TnuaAnimatingState::<AnimationState>::default());
    cmd.insert({
        CommandAlteringSelectors::default()
            .with_combo(
                "Sensor Shape",
                1,
                &[
                    ("no", |mut cmd| {
                        cmd.remove::<TnuaRapier3dSensorShape>();
                    }),
                    ("flat (underfit)", |mut cmd| {
                        cmd.insert(TnuaRapier3dSensorShape(Collider::cylinder(0.0, 0.49)));
                    }),
                    ("flat (exact)", |mut cmd| {
                        cmd.insert(TnuaRapier3dSensorShape(Collider::cylinder(0.0, 0.5)));
                    }),
                    ("ball (underfit)", |mut cmd| {
                        cmd.insert(TnuaRapier3dSensorShape(Collider::ball(0.49)));
                    }),
                    ("ball (exact)", |mut cmd| {
                        cmd.insert(TnuaRapier3dSensorShape(Collider::ball(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", true, |mut cmd, lock_tilt| {
                if lock_tilt {
                    cmd.insert(LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z);
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

    if keyboard.pressed(KeyCode::Up) {
        direction -= Vec3::Z;
    }
    if keyboard.pressed(KeyCode::Down) {
        direction += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::Left) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::Right) {
        direction += Vec3::X;
    }

    direction = direction.clamp_length_max(1.0);

    let jump = keyboard.pressed(KeyCode::Space);
    let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    let turn_in_place = keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

    let crouch_buttons = [KeyCode::ControlLeft, KeyCode::ControlRight];
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
            controller.action(TnuaBuiltinJump {
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinJump::NAME)
                    <= config.actions_in_air,
                ..config.jump.clone()
            });
        }

        if dash {
            controller.action(TnuaBuiltinDash {
                displacement: direction.normalize() * config.dash_distance,
                desired_forward: direction.normalize(),
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinDash::NAME)
                    <= config.actions_in_air,
                ..config.dash.clone()
            });
        }
    }
}

#[derive(Component)]
struct GltfSceneHandler {
    names_from: Handle<Gltf>,
}

#[derive(Component)]
pub struct AnimationsHandler {
    pub player_entity: Entity,
    pub animations: HashMap<String, Handle<AnimationClip>>,
}

fn animation_patcher_system(
    animation_players_query: Query<Entity, Added<AnimationPlayer>>,
    parents_query: Query<&Parent>,
    scene_handlers_query: Query<&GltfSceneHandler>,
    gltf_assets: Res<Assets<Gltf>>,
    mut commands: Commands,
) {
    for player_entity in animation_players_query.iter() {
        let mut entity = player_entity;
        loop {
            if let Ok(GltfSceneHandler { names_from }) = scene_handlers_query.get(entity) {
                let gltf = gltf_assets.get(names_from).unwrap();
                let mut cmd = commands.entity(entity);
                cmd.remove::<GltfSceneHandler>();
                cmd.insert(AnimationsHandler {
                    player_entity,
                    animations: gltf.named_animations.clone(),
                });
                break;
            }
            entity = if let Ok(parent) = parents_query.get(entity) {
                **parent
            } else {
                break;
            };
        }
    }
}

#[derive(Debug)]
enum AnimationState {
    Standing,
    Running(f32),
    Jumping,
    Falling,
    Crouching,
    Crawling(f32),
    Dashing,
}

#[derive(Component)]
struct Bla;

fn animate(
    mut animations_handlers_query: Query<(
        &mut TnuaAnimatingState<AnimationState>,
        &TnuaController,
        &AnimationsHandler,
    )>,
    mut animation_players_query: Query<&mut AnimationPlayer>,
) {
    for (mut animating_state, controller, handler) in animations_handlers_query.iter_mut() {
        let Ok(mut player) = animation_players_query.get_mut(handler.player_entity) else {
            continue;
        };
        match animating_state.update_by_discriminant({
            match controller.action_name() {
                Some(TnuaBuiltinJump::NAME) => {
                    let (_, jump_state) = controller
                        .concrete_action::<TnuaBuiltinJump>()
                        .expect("action name mismatch");
                    match jump_state {
                        TnuaBuiltinJumpState::NoJump => continue,
                        TnuaBuiltinJumpState::StartingJump { .. } => AnimationState::Jumping,
                        TnuaBuiltinJumpState::SlowDownTooFastSlopeJump { .. } => {
                            AnimationState::Jumping
                        }
                        TnuaBuiltinJumpState::MaintainingJump => AnimationState::Jumping,
                        TnuaBuiltinJumpState::StoppedMaintainingJump => AnimationState::Jumping,
                        TnuaBuiltinJumpState::FallSection => AnimationState::Falling,
                    }
                }
                Some(TnuaBuiltinCrouch::NAME) => {
                    let Some((_, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>()
                    else {
                        continue;
                    };
                    let speed =
                        Some(basis_state.running_velocity.length()).filter(|speed| 0.01 < *speed);
                    let is_crouching = basis_state.standing_offset < -0.4;
                    match (speed, is_crouching) {
                        (None, false) => AnimationState::Standing,
                        (None, true) => AnimationState::Crouching,
                        (Some(speed), false) => AnimationState::Running(0.1 * speed),
                        (Some(speed), true) => AnimationState::Crawling(0.1 * speed),
                    }
                }
                Some(TnuaBuiltinDash::NAME) => AnimationState::Dashing,
                Some(other) => panic!("Unknown action {other}"),
                None => {
                    let Some((_, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>()
                    else {
                        continue;
                    };
                    if basis_state.standing_on_entity().is_none() {
                        AnimationState::Falling
                    } else {
                        let speed = basis_state.running_velocity.length();
                        if 0.01 < speed {
                            AnimationState::Running(0.1 * speed)
                        } else {
                            AnimationState::Standing
                        }
                    }
                }
            }
        }) {
            TnuaAnimatingStateDirective::Maintain { state } => match state {
                AnimationState::Running(speed) | AnimationState::Crawling(speed) => {
                    player.set_speed(*speed);
                }
                AnimationState::Jumping | AnimationState::Dashing => {
                    if controller.action_flow_status().just_starting().is_some() {
                        player.seek_to(0.0);
                    }
                }
                _ => {}
            },
            TnuaAnimatingStateDirective::Alter {
                old_state: _,
                state,
            } => match state {
                AnimationState::Standing => {
                    player
                        .start(handler.animations["Standing"].clone_weak())
                        .set_speed(1.0)
                        .repeat();
                }
                AnimationState::Running(speed) => {
                    player
                        .start(handler.animations["Running"].clone_weak())
                        .set_speed(*speed)
                        .repeat();
                }
                AnimationState::Jumping => {
                    player
                        .start(handler.animations["Jumping"].clone_weak())
                        .set_speed(2.0);
                }
                AnimationState::Falling => {
                    player
                        .start(handler.animations["Falling"].clone_weak())
                        .set_speed(1.0);
                }
                AnimationState::Crouching => {
                    player
                        .start(handler.animations["Crouching"].clone_weak())
                        .set_speed(1.0)
                        .repeat();
                }
                AnimationState::Crawling(speed) => {
                    player
                        .start(handler.animations["Crawling"].clone_weak())
                        .set_speed(*speed)
                        .repeat();
                }
                AnimationState::Dashing => {
                    player
                        .start(handler.animations["Dashing"].clone_weak())
                        .set_speed(10.0);
                }
            },
        }
    }
}
