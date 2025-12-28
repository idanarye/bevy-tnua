use bevy::time::Stopwatch;
use bevy::{color::palettes::css, prelude::*};

use avian3d::prelude::*;

use bevy_tnua::builtins::{
    Tnua2BuiltinCrouch, Tnua2BuiltinCrouchConfig, Tnua2BuiltinJump, Tnua2BuiltinJumpConfig,
    Tnua2BuiltinWalk, Tnua2BuiltinWalkConfig,
};
use bevy_tnua::schemes_action_state::Tnua2ActionState;
use bevy_tnua::schemes_controller::{Tnua2Controller, Tnua2ControllerPlugin};
use bevy_tnua::schemes_traits::{
    Tnua2Action, Tnua2ActionContext, Tnua2ActionDiscriminant, Tnua2ActionStateEnum, Tnua2Basis,
    TnuaScheme, TnuaSchemeConfig, UpdateInActionStateEnumResult,
};
use bevy_tnua_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            // We need both Tnua's main controller plugin, and the plugin to connect to the physics
            // backend (in this case Avian 3D)
            Tnua2ControllerPlugin::<ExampleScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(
            Startup,
            (setup_camera_and_lights, setup_level, setup_player),
        )
        .add_systems(Update, apply_controls)
        .run();
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

    // Spawn a little platform for the player to jump on.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 4.0))),
        MeshMaterial3d(materials.add(Color::from(css::GRAY))),
        Transform::from_xyz(-6.0, 2.0, 0.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 1.0, 4.0),
    ));
}

#[derive(bevy_tnua::TnuaScheme)]
#[scheme(basis = Tnua2BuiltinWalk)]
enum ExampleScheme {
    Jump(Tnua2BuiltinJump),
    Crouch(Tnua2BuiltinCrouch, HalfSpeed),
}

struct HalfSpeed;

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut control_scheme_configs: ResMut<Assets<ExampleSchemeConfig>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d {
            radius: 0.5,
            half_length: 0.5,
        })),
        MeshMaterial3d(materials.add(Color::from(css::DARK_CYAN))),
        Transform::from_xyz(0.0, 2.0, 0.0),
        // The player character needs to be configured as a dynamic rigid body of the physics
        // engine.
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        // This is Tnua's interface component.
        Tnua2Controller::<ExampleScheme>::new(control_scheme_configs.add(ExampleSchemeConfig {
            basis: Tnua2BuiltinWalkConfig {
                // The `desired_velocity` determines how the character will move.
                // The `float_height` must be greater (even if by little) from the distance between the
                // character's center and the lowest point of its collider.
                float_height: 1.5,
                // `TnuaBuiltinWalk` has many other fields for customizing the movement - but they have
                // sensible defaults. Refer to the `TnuaBuiltinWalk`'s documentation to learn what they do.
                ..Default::default()
            },
            jump: Tnua2BuiltinJumpConfig {
                // The height is the only configuration field of the jump action that has no
                // sensible default.
                height: 4.0,
                // `TnuaBuiltinJump` also has customization fields with sensible defaults.
                ..Default::default()
            },
            crouch: Tnua2BuiltinCrouchConfig {
                float_offset: -0.4,
                ..Default::default()
            },
        })),
        // A sensor shape is not strictly necessary, but without it we'll get weird results.
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.0)),
        // Tnua can fix the rotation, but the character will still get rotated before it can do so.
        // By locking the rotation we can prevent this.
        LockedAxes::ROTATION_LOCKED,
    ));
}

fn apply_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Tnua2Controller<ExampleScheme>>,
) {
    let Ok(mut controller) = query.single_mut() else {
        return;
    };
    controller.initiate_action_feeding();

    let mut direction = Vec3::ZERO;

    if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        direction -= Vec3::Z;
    }
    if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        direction += Vec3::Z;
    }
    if keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        direction -= Vec3::X;
    }
    if keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        direction += Vec3::X;
    }

    // Update the basis every frame. Even if the player doesn't move - just set `desired_motion` to
    // `Vec3::ZERO`.
    controller.basis.desired_motion = direction.normalize_or_zero();

    // Feed the jump action every frame as long as the player holds the jump button. If the player
    // stops holding the jump button, simply stop feeding the action.
    if keyboard.pressed(KeyCode::Space) {
        controller.action(ExampleScheme::Jump(Default::default()));
    }

    if keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        controller.action(ExampleScheme::Crouch(Default::default(), HalfSpeed));
    }
}
