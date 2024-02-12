use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::{prelude as rapier, prelude::*};
use bevy_tnua::builtins::TnuaBuiltinCrouch;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaAnimatingState, TnuaGhostSensor, TnuaToggle};
#[cfg(feature = "rapier3d")]
use bevy_tnua_rapier3d::*;
#[cfg(feature = "xpbd3d")]
use bevy_tnua_xpbd3d::*;
#[cfg(feature = "xpbd3d")]
use bevy_xpbd_3d::{prelude as xpbd, prelude::*};

use tnua_examples_crate::character_animating_systems::platformer_animating_systems::{
    animate_platformer_character, AnimationState,
};
use tnua_examples_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerExample,
    FallingThroughControlScheme, ForwardFromCamera,
};
use tnua_examples_crate::character_control_systems::Dimensionality;
#[cfg(feature = "xpbd3d")]
use tnua_examples_crate::levels_setup::for_3d_platformer::LayerNames;
use tnua_examples_crate::ui::component_alterbation::CommandAlteringSelectors;
use tnua_examples_crate::ui::plotting::PlotSource;
use tnua_examples_crate::util::animating::{animation_patcher_system, GltfSceneHandler};
use tnua_examples_crate::MovingPlatformPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "rapier3d")]
    {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());
        // To use Tnua with bevy_rapier3d, you need the `TnuaRapier3dPlugin` plugin from
        // bevy-tnua-rapier3d.
        app.add_plugins(TnuaRapier3dPlugin);
    }
    #[cfg(feature = "xpbd3d")]
    {
        app.add_plugins(PhysicsPlugins::default());
        // To use Tnua with bevy_xpbd_3d, you need the `TnuaXpbd3dPlugin` plugin from
        // bevy-tnua-xpbd3d.
        app.add_plugins(TnuaXpbd3dPlugin);
    }

    // This is Tnua's main plugin.
    app.add_plugins(TnuaControllerPlugin);

    // This plugin supports `TnuaCrouchEnforcer`, which prevents the character from standing up
    // while obstructed by an obstacle.
    app.add_plugins(TnuaCrouchEnforcerPlugin);

    app.add_plugins(tnua_examples_crate::ui::ExampleUi::<
        CharacterMotionConfigForPlatformerExample,
    >::default());
    app.add_systems(Startup, setup_camera);
    app.add_systems(
        Startup,
        tnua_examples_crate::levels_setup::for_3d_platformer::setup_level,
    );
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, (grab_ungrab_mouse, apply_camera_controls));
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
    );
    app.add_systems(Update, animation_patcher_system);
    app.add_systems(Update, animate_platformer_character);
    app.add_plugins(MovingPlatformPlugin);
    app.run();
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

    // The character entity must be configured as a dynamic rigid body of the physics backend.
    #[cfg(feature = "rapier3d")]
    {
        cmd.insert(rapier::RigidBody::Dynamic);
        cmd.insert(rapier::Collider::capsule_y(0.5, 0.5));
        // For Rapier, an "IO" bundle needs to be added so that Tnua will have all the components
        // it needs to interact with Rapier.
        cmd.insert(TnuaRapier3dIOBundle::default());
    }
    #[cfg(feature = "xpbd3d")]
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

    cmd.insert(CharacterMotionConfigForPlatformerExample {
        dimensionality: Dimensionality::Dim3,
        speed: 20.0,
        walk: TnuaBuiltinWalk {
            float_height: 2.0,
            turning_angvel: f32::INFINITY,
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

    cmd.insert(ForwardFromCamera::default());

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
                        #[cfg(feature = "xpbd3d")]
                        cmd.remove::<TnuaXpbd3dSensorShape>();
                    }),
                    ("flat (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.49,
                        )));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.49)));
                    }),
                    ("flat (exact)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.5,
                        )));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.5)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
                            0.0, 0.51,
                        )));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.51)));
                    }),
                    ("ball (underfit)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::ball(0.49)));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::ball(0.49)));
                    }),
                    ("ball (exact)", |mut cmd| {
                        #[cfg(feature = "rapier3d")]
                        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::ball(0.5)));
                        #[cfg(feature = "xpbd3d")]
                        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::ball(0.5)));
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
                    #[cfg(feature = "xpbd3d")]
                    cmd.insert(xpbd::LockedAxes::new().lock_rotation_x().lock_rotation_z());
                } else {
                    #[cfg(feature = "rapier3d")]
                    cmd.insert(rapier::LockedAxes::empty());
                    #[cfg(feature = "xpbd3d")]
                    cmd.insert(xpbd::LockedAxes::new());
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
                    #[cfg(feature = "xpbd3d")]
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

    // `TnuaCrouchEnforcer` can be used to prevent the character from standing up when obstructed.
    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vec3::Y, |cmd| {
        #[cfg(feature = "rapier3d")]
        cmd.insert(TnuaRapier3dSensorShape(rapier::Collider::cylinder(
            0.0, 0.5,
        )));
        #[cfg(feature = "xpbd3d")]
        cmd.insert(TnuaXpbd3dSensorShape(xpbd::Collider::cylinder(0.0, 0.5)));
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

    cmd.insert(tnua_examples_crate::ui::TrackedEntity("Player".to_owned()));
    cmd.insert(PlotSource::default());
}

fn grab_ungrab_mouse(
    mut egui_context: bevy_egui::EguiContexts,
    mouse_buttons: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    mut primary_window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = primary_window_query.get_single_mut() else {
        return;
    };
    if window.cursor.visible {
        if mouse_buttons.just_pressed(MouseButton::Left) {
            let ui = egui_context.ctx_mut();
            if !ui.is_pointer_over_area() {
                window.cursor.grab_mode = CursorGrabMode::Locked;
                window.cursor.visible = false;
            }
        }
    } else if keyboard.just_released(KeyCode::Escape)
        || mouse_buttons.just_pressed(MouseButton::Left)
    {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}

fn apply_camera_controls(
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_character_query: Query<(&GlobalTransform, &mut ForwardFromCamera)>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let mouse_controls_camera = primary_window_query
        .get_single()
        .map_or(false, |w| !w.cursor.visible);
    let total_delta = if mouse_controls_camera {
        mouse_motion.read().map(|event| event.delta).sum()
    } else {
        mouse_motion.clear();
        Vec2::ZERO
    };
    let turning = Quat::from_rotation_y(-0.01 * total_delta.x);
    let Ok((player_transform, mut forward_from_camera)) = player_character_query.get_single_mut()
    else {
        return;
    };
    forward_from_camera.forward = turning.mul_vec3(forward_from_camera.forward);

    for mut camera in camera_query.iter_mut() {
        camera.translation =
            player_transform.translation() + -5.0 * forward_from_camera.forward + 1.0 * Vec3::Y;
        camera.look_to(forward_from_camera.forward, Vec3::Y);
    }
}
