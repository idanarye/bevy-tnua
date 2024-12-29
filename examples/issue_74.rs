use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::prelude::*;
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (input, test).in_set(TnuaUserControlsSystemSet))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        RigidBody::Dynamic,
        Collider::capsule(0.5, 2.),
        TnuaController::default(),
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.)),
        LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        Transform::from_xyz(0., 2., 0.),
        // Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
    ));
    commands.spawn((
        Name::new("ground"),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));
    commands.spawn((
        Name::new("cube"),
        RigidBody::Dynamic,
        Collider::cuboid(3., 3., 3.),
        Transform::from_xyz(6., 1.5, 0.),
    ));
}

fn input(
    kb: Res<ButtonInput<KeyCode>>,
    mut control: Query<&mut TnuaController>,
    mut last_dir: Local<Vec3>,
) {
    if let Ok(mut control) = control.get_single_mut() {
        let a = |pos, neg| {
            0. + if kb.pressed(pos) { 1. } else { 0. } + if kb.pressed(neg) { -1. } else { 0. }
        };
        let dir = Vec3::new(a(KeyCode::KeyD, KeyCode::KeyA), 0., a(KeyCode::KeyS, KeyCode::KeyW));
        if dir != Vec3::ZERO {
            *last_dir = dir;
        }
        control.basis(TnuaBuiltinWalk {
            desired_velocity: dir.normalize_or_zero() * 10.,
            desired_forward: Some(
                Dir3::new(dir).unwrap_or(Dir3::new(*last_dir).unwrap_or(Dir3::Z)),
            ),
            float_height: 1.5,
            coyote_time: 0.05,
            ..default()
        });
        if kb.pressed(KeyCode::Space) {
            control.action(TnuaBuiltinJump {
                height: 4.,
                ..default()
            });
        }
    }
}

fn test(
    control: Query<&TnuaController>,
    cube: Query<(&Name, &LinearVelocity), Changed<LinearVelocity>>,
) {
    if let Some((_, walk)) = control.single().concrete_basis::<TnuaBuiltinWalk>() {
        if walk.running_velocity.x < 0. && walk.running_velocity.x.abs() > 0.1 {
            dbg!(walk.running_velocity.x);
        }
    }
    if let Some(cube) = cube.iter().find(|(n, _)| n.as_str() == "cube") {
        if cube.1.x.abs() > 0.1 {
            dbg!(cube.1.x);
        }
    }
}
