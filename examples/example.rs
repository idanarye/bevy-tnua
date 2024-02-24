use bevy::prelude::*;

use bevy_xpbd_3d::prelude::*;

use bevy_tnua::prelude::*;
use bevy_tnua_xpbd3d::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            // We need both Tnua's main controller plugin, and the plugin to connect to the physics
            // backend (in this case XBPD-3D)
            TnuaControllerPlugin,
            TnuaXpbd3dPlugin,
        ))
        .add_systems(
            Startup,
            (setup_camera_and_lights, setup_level, setup_player),
        )
        .add_systems(Update, apply_controls.in_set(TnuaUserControlsSystemSet))
        .run();
}

// No Tnua-related setup here - this is just normal Bevy stuff.
fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 16.0, 40.0)
            .looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });

    // A directly-down light to tell where the player is going to land.
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

// No Tnua-related setup here - this is just normal Bevy (and XPBD) stuff.
fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the ground.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(128.0, 128.0)),
            material: materials.add(Color::WHITE),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::halfspace(Vec3::Y),
    ));

    // Spawn a little platform for the player to jump on.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(4.0, 1.0, 4.0)),
            material: materials.add(Color::GRAY),
            transform: Transform::from_xyz(-6.0, 2.0, 0.0),
            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(4.0, 1.0, 4.0),
    ));
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d {
                radius: 0.5,
                half_length: 0.5,
            }),
            material: materials.add(Color::CYAN),
            transform: Transform::from_xyz(0.0, 2.0, 0.0),
            ..Default::default()
        },
        // The player character needs to be configured as a dynamic rigid body of the physics
        // engine.
        RigidBody::Dynamic,
        Collider::capsule(1.0, 0.5),
        // This bundle holds the main components.
        TnuaControllerBundle::default(),
        // A sensor shape is not strictly necessary, but without it we'll get weird results.
        TnuaXpbd3dSensorShape(Collider::cylinder(0.0, 0.49)),
        // Tnua can fix the rotation, but the character will still get rotated before it can do so.
        // By locking the rotation we can prevent this.
        LockedAxes::ROTATION_LOCKED,
    ));
    // NOTE: if this was Rapier, we'd also need `TnuaRapier3dIOBundle`. XPBD does not need it.
}

fn apply_controls(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut TnuaController>) {
    let Ok(mut controller) = query.get_single_mut() else {
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
        // The `float_height` must be greater (even if by little) from the distance between the
        // character's center and the lowest point of its collider.
        float_height: 1.5,
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
