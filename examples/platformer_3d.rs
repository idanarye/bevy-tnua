mod common;

use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::EguiContexts;
use bevy_rapier3d::prelude::*;
use bevy_tnua::control_helpers::TnuaSimpleFallThroughPlatformsHelper;
use bevy_tnua::controller::{TnuaController, TnuaPlatformerPlugin2};
use bevy_tnua::{
    tnua_action, tnua_basis, TnuaFreeFallBehavior, TnuaGhostPlatform, TnuaGhostSensor,
    TnuaKeepCrouchingBelowObstacles, TnuaMotor, TnuaPipelineStages, TnuaPlatformerConfig,
    TnuaProximitySensor, TnuaRapier3dIOBundle, TnuaRapier3dPlugin, TnuaRapier3dSensorShape,
    TnuaRigidBodyTracker, TnuaToggle, TnuaUserControlsSystemSet,
};

use self::common::ui::{CommandAlteringSelectors, ExampleUiPhysicsBackendActive};
use self::common::ui_plotting::PlotSource;
use self::common::{FallingThroughControlScheme, MovingPlatform};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugins(TnuaRapier3dPlugin);
    app.add_plugins(TnuaPlatformerPlugin2);
    app.add_plugins(common::ui::ExampleUi);
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

    if true {
        return;
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
    cmd.insert(TnuaController::default());
    cmd.insert(TnuaPlatformerConfig {
        full_speed: 20.0,
        full_jump_height: 4.0,
        up: Vec3::Y,
        forward: -Vec3::Z,
        float_height: 2.0,
        cling_distance: 1.0,
        spring_strengh: 400.0,
        spring_dampening: 1.2,
        acceleration: 60.0,
        air_acceleration: 20.0,
        coyote_time: 0.15,
        jump_input_buffer_time: 0.2,
        held_jump_cooldown: None,
        upslope_jump_extra_gravity: 30.0,
        jump_takeoff_extra_gravity: 30.0,
        jump_takeoff_above_velocity: 2.0,
        jump_fall_extra_gravity: 20.0,
        jump_shorten_extra_gravity: 60.0,
        jump_peak_prevention_at_upward_velocity: 1.0,
        jump_peak_prevention_extra_gravity: 20.0,
        free_fall_behavior: TnuaFreeFallBehavior::LikeJumpShorten,
        tilt_offset_angvel: 5.0,
        tilt_offset_angacl: 500.0,
        turning_angvel: 10.0,
        height_change_impulse_for_duration: 0.02,
        height_change_impulse_limit: 40.0,
    });
    cmd.insert((
        TnuaMotor::default(),
        TnuaRigidBodyTracker::default(),
        TnuaProximitySensor::default(),
    ));
    cmd.insert(TnuaToggle::default());
    cmd.insert(TnuaKeepCrouchingBelowObstacles::new(1.5, |cmd| {
        cmd.insert(TnuaRapier3dSensorShape(Collider::cylinder(0.0, 0.5)));
    }));
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(FallingThroughControlScheme::default());
    // cmd.insert(TnuaAnimatingState::<AnimationState>::default());
    // cmd.insert(TnuaPlatformerAnimatingOutput::default());
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
        &TnuaPlatformerConfig,
        &mut TnuaController,
        &TnuaKeepCrouchingBelowObstacles,
        &mut TnuaProximitySensor,
        &TnuaGhostSensor,
        &mut TnuaSimpleFallThroughPlatformsHelper,
        &FallingThroughControlScheme,
    )>,
) {
    if egui_context.ctx_mut().wants_keyboard_input() {
        // for (mut controls, ..) in query.iter_mut() {
        // *controls = Default::default();
        // }
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

    // let turn_in_place = keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

    // let crouch_buttons = [KeyCode::ControlLeft, KeyCode::ControlRight];
    // let crouch = keyboard.any_pressed(crouch_buttons);
    // let crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);

    for (
        config,
        mut controller,
        _keep_crouching,
        mut _sensor,
        _ghost_sensor,
        mut _fall_through_helper,
        _falling_through_control_scheme,
    ) in query.iter_mut()
    {
        controller.basis(
            "walk",
            tnua_basis::Walk {
                desired_velocity: direction * config.full_speed,
                float_height: config.float_height,
                cling_distance: config.cling_distance,
                up: Vec3::Y,
                spring_strengh: config.spring_strengh,
                spring_dampening: config.spring_dampening,
                height_change_impulse_for_duration: config.height_change_impulse_for_duration,
                height_change_impulse_limit: config.height_change_impulse_limit,
                acceleration: config.acceleration,
                air_acceleration: config.air_acceleration,
                coyote_time: config.coyote_time,
                free_fall_extra_gravity: match config.free_fall_behavior {
                    TnuaFreeFallBehavior::ExtraGravity(extra_gravity) => extra_gravity,
                    TnuaFreeFallBehavior::LikeJumpShorten => config.jump_shorten_extra_gravity,
                    TnuaFreeFallBehavior::LikeJumpFall => config.jump_fall_extra_gravity,
                },
            },
        );

        if jump {
            controller.action(
                "jump",
                tnua_action::Jump {
                    height: config.full_jump_height,
                    takeoff_extra_gravity: config.jump_takeoff_extra_gravity,
                    takeoff_above_velocity: config.jump_takeoff_above_velocity,
                    peak_prevention_at_upward_velocity: config
                        .jump_peak_prevention_at_upward_velocity,
                    peak_prevention_extra_gravity: config.jump_peak_prevention_extra_gravity,
                    shorten_extra_gravity: config.jump_shorten_extra_gravity,
                    fall_extra_gravity: config.jump_fall_extra_gravity,
                },
            );
        }
        // let crouch = falling_through_control_scheme.perform_and_check_if_still_crouching(
        // crouch,
        // crouch_just_pressed,
        // fall_through_helper.as_mut(),
        // sensor.as_mut(),
        // ghost_sensor,
        // 1.0,
        // );
        // let speed_factor = if crouch || keep_crouching.force_crouching_to_height < -0.5 {
        // 0.2
        // } else {
        // 1.0
        // };

        // *controls = TnuaPlatformerControls {
        // desired_velocity: if turn_in_place {
        // Vec3::ZERO
        // } else {
        // speed_factor * direction
        // },
        // desired_forward: direction.normalize(),
        // jump: jump.then(|| 1.0),
        // float_height_offset: if crouch { -0.9 } else { 0.0 },
        // };
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

// enum AnimationState {
// Standing,
// Running(f32),
// Jumping,
// Falling,
// Crouching,
// Crawling(f32),
// }

fn animate(// mut animations_handlers_query: Query<(
        // &mut TnuaAnimatingState<AnimationState>,
        // &TnuaPlatformerAnimatingOutput,
        // &AnimationsHandler,
    // )>,
    // mut animation_players_query: Query<&mut AnimationPlayer>,
) {
    // for (mut animating_state, animating_output, handler) in animations_handlers_query.iter_mut() {
    // let Ok(mut player) = animation_players_query.get_mut(handler.player_entity) else {
    // continue;
    //};
    // match animating_state.update_by_discriminant({
    // if let Some(upward_velocity) = animating_output.jumping_velocity {
    // if 0.0 < upward_velocity {
    // AnimationState::Jumping
    // } else {
    // AnimationState::Falling
    // }
    // } else {
    // let speed = animating_output.running_velocity.length();
    // let is_crouching = animating_output.standing_offset < -0.3;
    // if 0.01 < speed {
    // if is_crouching {
    // AnimationState::Crawling(0.3 * speed)
    // } else {
    // AnimationState::Running(0.1 * speed)
    // }
    // } else {
    // if is_crouching {
    // AnimationState::Crouching
    // } else {
    // AnimationState::Standing
    // }
    // }
    // }
    // }) {
    // TnuaAnimatingStateDirective::Maintain { state } => {
    // if let AnimationState::Running(speed) | AnimationState::Crawling(speed) = state {
    // player.set_speed(*speed);
    // }
    // }
    // TnuaAnimatingStateDirective::Alter {
    // old_state: _,
    // state,
    // } => match state {
    // AnimationState::Standing => {
    // player
    // .start(handler.animations["Standing"].clone_weak())
    // .set_speed(1.0)
    // .repeat();
    // }
    // AnimationState::Running(speed) => {
    // player
    // .start(handler.animations["Running"].clone_weak())
    // .set_speed(*speed)
    // .repeat();
    // }
    // AnimationState::Jumping => {
    // player
    // .start(handler.animations["Jumping"].clone_weak())
    // .set_speed(2.0);
    // }
    // AnimationState::Falling => {
    // player
    // .start(handler.animations["Falling"].clone_weak())
    // .set_speed(1.0);
    // }
    // AnimationState::Crouching => {
    // player
    // .start(handler.animations["Crouching"].clone_weak())
    // .set_speed(1.0)
    // .repeat();
    // }
    // AnimationState::Crawling(speed) => {
    // player
    // .start(handler.animations["Crawling"].clone_weak())
    // .set_speed(*speed)
    // .repeat();
    // }
    // },
    // }
    // }
}
