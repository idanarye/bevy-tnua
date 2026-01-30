use bevy::{color::palettes::css, prelude::*};

use avian3d::prelude::*;

use bevy_tnua::builtins::{
    TnuaBuiltinDash, TnuaBuiltinDashConfig, TnuaBuiltinJump, TnuaBuiltinJumpConfig,
    TnuaBuiltinWalk, TnuaBuiltinWalkConfig,
};
use bevy_tnua::control_helpers::{TnuaActionSlots, TnuaActionsCounter, TnuaAirActionsPlugin};
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            TnuaControllerPlugin::<ControlScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
            // This plugin updates `TnuaActionsCounter<AirActionSlots>`, which allows
            // `apply_controls` to use it to determine if the player is allowed to perfrom an
            // action midair.
            TnuaAirActionsPlugin::<AirActionSlots>::new(FixedUpdate),
        ))
        .add_systems(
            Startup,
            (setup_camera_and_lights, setup_level, setup_player),
        )
        .add_systems(Update, apply_controls.in_set(TnuaUserControlsSystems))
        .run();
}

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
enum ControlScheme {
    Jump(TnuaBuiltinJump),
    Dash(TnuaBuiltinDash),
}

// This struct defines how air actions a character controlled by `ControlScheme` are counted. It
// can contain multiple `usize` fields (and no other fields!), each counting one kind of air
// action.
#[derive(TnuaActionSlots)]
// Must define the scheme it refers to.
#[slots(scheme = ControlScheme)]
struct AirActionSlots {
    // Each slot needs to define which actions use it. Multiple actions can use the same slot, in
    // which case they will be counted together. This means that if Jump and Dash were on the same
    // slot, and the limit was 1, the player could do one air jump or one air dash - but not both
    // during the same jump.
    #[slots(Jump)]
    jump: usize,
    #[slots(Dash)]
    dash: usize,
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

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut control_scheme_configs: ResMut<Assets<ControlSchemeConfig>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d {
            radius: 0.5,
            half_length: 0.5,
        })),
        MeshMaterial3d(materials.add(Color::from(css::DARK_CYAN))),
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        TnuaController::<ControlScheme>::default(),
        TnuaConfig::<ControlScheme>(control_scheme_configs.add(ControlSchemeConfig {
            basis: TnuaBuiltinWalkConfig {
                float_height: 1.5,
                ..Default::default()
            },
            jump: TnuaBuiltinJumpConfig {
                height: 6.0,
                ..Default::default()
            },
            dash: TnuaBuiltinDashConfig {
                horizontal_distance: 10.0,
                brake_to_speed: 0.0,
                ..Default::default()
            },
        })),
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.0)),
        LockedAxes::ROTATION_LOCKED,
    ));
}

fn apply_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &mut TnuaController<ControlScheme>,
        // We need this to determine whether or not the character is allowed to perform an air
        // action.
        &mut TnuaActionsCounter<AirActionSlots>,
    )>,
) {
    let Ok((mut controller, air_actions)) = query.single_mut() else {
        return;
    };
    controller.initiate_action_feeding();

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

    // Set the basis every frame. Even if the player doesn't move - just use `desired_velocity:
    // Vec3::ZERO` to reset the previous frame's input.
    controller.basis = TnuaBuiltinWalk {
        // The `desired_motion` determines how the character will move.
        desired_motion: direction.normalize_or_zero(),
        // The other field is `desired_forward` - but since the character model is a capsule we
        // don't care the direction its "forward" is pointing.
        ..Default::default()
    };

    // Feed the jump action every frame as long as the player holds the jump button. If the player
    // stops holding the jump button, simply stop feeding the action.
    if keyboard.pressed(KeyCode::Space) {
        controller.action(ControlScheme::Jump(TnuaBuiltinJump {
            // Tell the action whether or not it is allowed to run mid-air.
            allow_in_air: air_actions
                // Use this intstead of accessing the slot directly, because it does not count the
                // current jump. This is important because the Jump action needs to be fed for as
                // long as the player holds the button, otherwise Tnua will shorten the jump.
                .count_for(ControlSchemeActionDiscriminant::Jump)
                // `TnuaActionsCounter::get` returns the nubmer of air actions already performed in
                // that slot. It is up to the user control system to determine what this number
                // means. Usually it is enough to compare it against the number of actions allowed,
                // but it can also be used for more complex logic (e.g. reduce the height of the
                // second jump)
                < 1,
            ..Default::default()
        }));
    }

    if direction != Vec3::ZERO && keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        controller.action(ControlScheme::Dash(TnuaBuiltinDash {
            displacement: direction.normalize_or_zero(),
            allow_in_air: air_actions
                // For the Dash we pass a differnt discriminant, and because we defined a different
                // slot for it it will be counted separately from the Jump. This means the player
                // can perform one air jump and one air dash during each jump.
                .count_for(ControlSchemeActionDiscriminant::Dash)
                < 1,
            ..Default::default()
        }));
    }
}
