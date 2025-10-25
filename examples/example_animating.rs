// ###############################################################################################
// # The important system for this example is `handle_animating`. Everything else is just setup. #
// ###############################################################################################

use bevy::prelude::*;

use avian3d::prelude::*;

use bevy_tnua::{
    builtins::TnuaBuiltinJumpState, prelude::*, TnuaAnimatingState, TnuaAnimatingStateDirective,
};
use bevy_tnua_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(
            Startup,
            (setup_camera_and_lights, setup_level, setup_player),
        )
        .add_systems(
            FixedUpdate,
            (
                apply_controls.in_set(TnuaUserControlsSystems),
                prepare_animations,
                handle_animating,
            ),
        )
        .run();
}

// This enum projects the player's state into something we can use to decide which animation to
// play. Each variant of this enum corresponds to an animation, and the variant data can affect the
// animation's parameters.
//
// By itself this does not do much, but we can attach a `TnuaAnimatingState<AnimationState>`
// component to the player entity and use it to track the animating state.
pub enum AnimationState {
    Standing,
    Running(f32),
    Jumping,
    Falling,
}

// Bevy's animation handling is a bit manual. We'll use this struct to register the animation clips
// as nodes in the animation graph.
#[derive(Resource)]
struct AnimationNodes {
    standing: AnimationNodeIndex,
    running: AnimationNodeIndex,
    jumping: AnimationNodeIndex,
    falling: AnimationNodeIndex,
}

// No Tnua-related setup here - this is just normal Bevy stuff.
fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

    // A directly-down light to tell where the player is going to land.
    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::default().looking_at(-Vec3::Y, Vec3::Z),
    ));
}

// No Tnua-related setup here - this is just normal Bevy (and Avian) stuff.
fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(128.0, 128.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));
}

// Bevy assets are a bit weird. We'll use this resource to hold onto the model file so that we can
// extract the animation clips from it and build the animation graph.
#[derive(Resource)]
struct PlayerGltfHandle(Handle<Gltf>);

fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    // We'll need this in `prepare_animations` to build the animation graph.
    commands.insert_resource(PlayerGltfHandle(asset_server.load("player.glb")));

    commands.spawn((
        SceneRoot(asset_server.load("player.glb#Scene0")),
        Transform::from_xyz(0.0, 2.0, 0.0),
        // We'll need this in the `handle_animating` system to keep track of the players animating
        // state.
        TnuaAnimatingState::<AnimationState>::default(),
        // The player character needs to be configured as a dynamic rigid body of the physics
        // engine.
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        // This is Tnua's interface component.
        TnuaController::default(),
        // A sensor shape is not strictly necessary, but without it we'll get weird results.
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.0)),
        // Tnua can fix the rotation, but the character will still get rotated before it can do so.
        // By locking the rotation we can prevent this.
        LockedAxes::ROTATION_LOCKED.unlock_rotation_y(),
    ));
}

// No Tnua-related setup here - this is just for dealing with Bevy's animation graph.
fn prepare_animations(
    handle: Option<Res<PlayerGltfHandle>>,
    gltf_assets: Res<Assets<Gltf>>,
    mut commands: Commands,
    animation_player_query: Query<Entity, With<AnimationPlayer>>,
    mut animation_graphs_assets: ResMut<Assets<AnimationGraph>>,
) {
    let Some(handle) = handle else { return };
    let Some(gltf) = gltf_assets.get(&handle.0) else {
        return;
    };
    let Ok(animation_player_entity) = animation_player_query.single() else {
        return;
    };

    let mut graph = AnimationGraph::new();
    let root_node = graph.root;

    commands.insert_resource(AnimationNodes {
        standing: graph.add_clip(gltf.named_animations["Standing"].clone(), 1.0, root_node),
        running: graph.add_clip(gltf.named_animations["Running"].clone(), 1.0, root_node),
        jumping: graph.add_clip(gltf.named_animations["Jumping"].clone(), 1.0, root_node),
        falling: graph.add_clip(gltf.named_animations["Falling"].clone(), 1.0, root_node),
    });

    commands
        .entity(animation_player_entity)
        .insert(AnimationGraphHandle(animation_graphs_assets.add(graph)));

    // So that we won't run this again
    commands.remove_resource::<PlayerGltfHandle>();
}

fn apply_controls(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut TnuaController>) {
    let Ok(mut controller) = query.single_mut() else {
        return;
    };

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowUp) {
        direction -= Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction += Vec3::X;
    }

    // Feed the basis every frame. Even if the player doesn't move - just use `desired_velocity:
    // Vec3::ZERO`. `TnuaController` starts without a basis, which will make the character collider
    // just fall.
    controller.basis(TnuaBuiltinWalk {
        // The `desired_velocity` determines how the character will move.
        desired_velocity: direction.normalize_or_zero() * 10.0,
        desired_forward: Dir3::new(direction).ok(),
        // The `float_height` must be greater (even if by little) from the distance between the
        // character's center and the lowest point of its collider.
        float_height: 2.0,
        // `TnuaBuiltinWalk` has many other fields for customizing the movement - but they have
        // sensible defaults. Refer to the `TnuaBuiltinWalk`'s documentation to learn what they do.
        ..Default::default()
    });

    // Feed the jump action every frame as long as the player holds the jump button. If the player
    // stops holding the jump button, simply stop feeding the action.
    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            // The height is the only mandatory field of the jump button.
            height: 4.0,
            // `TnuaBuiltinJump` also has customization fields with sensible defaults.
            ..Default::default()
        });
    }
}

// This is the important system for this example
fn handle_animating(
    mut player_query: Query<(&TnuaController, &mut TnuaAnimatingState<AnimationState>)>,
    mut animation_player_query: Query<&mut AnimationPlayer>,
    animation_nodes: Option<Res<AnimationNodes>>,
) {
    // An actual game should match the animation player and the controller. Here we cheat for
    // simplicity and use the only controller and only player.
    let Ok((controller, mut animating_state)) = player_query.single_mut() else {
        return;
    };
    let Ok(mut animation_player) = animation_player_query.single_mut() else {
        return;
    };
    let Some(animation_nodes) = animation_nodes else {
        return;
    };

    // Here we use the data from TnuaController to determine what the character is currently doing,
    // so that we can later use that information to decide which animation to play.

    // First we look at the `action_name` to determine which action (if at all) the character is
    // currently performing:
    let current_status_for_animating = match controller.action_name() {
        // Unless you provide the action names yourself, prefer matching against the `NAME` const
        // of the `TnuaAction` trait. Once `type_name` is stabilized as `const` Tnua will use it to
        // generate these names automatically, which may result in a change to the name.
        Some(TnuaBuiltinJump::NAME) => {
            // In case of jump, we want to cast it so that we can get the concrete jump state.
            let (_, jump_state) = controller
                .concrete_action::<TnuaBuiltinJump>()
                .expect("action name mismatch");
            // Depending on the state of the jump, we need to decide if we want to play the jump
            // animation or the fall animation.
            match jump_state {
                TnuaBuiltinJumpState::NoJump => return,
                TnuaBuiltinJumpState::StartingJump { .. } => AnimationState::Jumping,
                TnuaBuiltinJumpState::SlowDownTooFastSlopeJump { .. } => AnimationState::Jumping,
                TnuaBuiltinJumpState::MaintainingJump { .. } => AnimationState::Jumping,
                TnuaBuiltinJumpState::StoppedMaintainingJump => AnimationState::Jumping,
                TnuaBuiltinJumpState::FallSection => AnimationState::Falling,
            }
        }
        // Tnua should only have the `action_name` of the actions you feed to it. If it has
        // anything else - consider it a bug.
        Some(other) => panic!("Unknown action {other}"),
        // No action name means that no action is currently being performed - which means the
        // animation should be decided by the basis.
        None => {
            // If there is no action going on, we'll base the animation on the state of the
            // basis.
            let Some((_, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>() else {
                // Since we only use the walk basis in this example, if we can't get get this
                // basis' state it probably means the system ran before any basis was set, so we
                // just stkip this frame.
                return;
            };
            if basis_state.standing_on_entity().is_none() {
                // The walk basis keeps track of what the character is standing on. If it doesn't
                // stand on anything, `standing_on_entity` will be empty - which means the
                // character has walked off a cliff and needs to fall.
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
    };

    let animating_directive = animating_state.update_by_discriminant(current_status_for_animating);

    match animating_directive {
        TnuaAnimatingStateDirective::Maintain { state } => {
            // `Maintain` means that we did not switch to a different variant, so there is no need
            // to change animations.

            // Specifically for the running animation, even when the state remains the speed can
            // still change. When it does, we simply need to update the speed in the animation
            // player.
            if let AnimationState::Running(speed) = state {
                if let Some(animation) = animation_player.animation_mut(animation_nodes.running) {
                    animation.set_speed(*speed);
                }
            }
        }
        TnuaAnimatingStateDirective::Alter {
            old_state: _,
            state,
        } => {
            // `Alter` means that we have switched to a different variant and need to play a
            // different animation.

            // First - stop the currently running animation. We don't check which one is running
            // here because we just assume it belongs to the old state, but more sophisticated code
            // can try to phase from the old animation to the new one.
            animation_player.stop_all();

            // Depending on the new state, we choose the animation to run and its parameters (here
            // they are the speed and whether or not to repeat)
            match state {
                AnimationState::Standing => {
                    animation_player
                        .start(animation_nodes.standing)
                        .set_speed(1.0)
                        .repeat();
                }
                AnimationState::Running(speed) => {
                    animation_player
                        .start(animation_nodes.running)
                        // The running animation, in particular, has a speed that depends on how
                        // fast the character is running. Note that if the speed changes while the
                        // character is still running we won't get `Alter` again - so it's
                        // important to also update the speed in `Maintain { State: Running }`.
                        .set_speed(*speed)
                        .repeat();
                }
                AnimationState::Jumping => {
                    animation_player
                        .start(animation_nodes.jumping)
                        .set_speed(2.0);
                }
                AnimationState::Falling => {
                    animation_player
                        .start(animation_nodes.falling)
                        .set_speed(1.0);
                }
            }
        }
    }
}
