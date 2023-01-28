mod common;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_tnua::{
    TnuaMotor, TnuaPlatformerConfig, TnuaPlatformerControls, TnuaPlatformerPlugin,
    TnuaProximitySensor, TnuaRapier2dPlugin,
};

use self::common::ui::SpeedControl;

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
    app.add_system(apply_controls);
    app.add_startup_system(|mut cfg: ResMut<RapierConfiguration>| {
        cfg.gravity = Vec2::Y * -9.81;
    });
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(0.0, 9.0, 30.0)
            .with_scale((0.05 * Vec2::ONE).extend(1.0))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });
}

fn setup_level(mut commands: Commands) {
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
        ([4.0, 1.0], Transform::from_xyz(3.0, 1.0, 0.0)),
        (
            [6.0, 0.1],
            Transform::from_xyz(-3.0, 1.0, 0.0).with_rotation(Quat::from_rotation_z(-0.6)),
        ),
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
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(TransformBundle::from_transform(Transform::from_xyz(
        0.0, 10.0, 0.0,
    )));
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(LockedAxes::ROTATION_LOCKED); // todo: fix with torque
    cmd.insert(Velocity::default());
    cmd.insert(Collider::capsule_y(0.5, 0.5));
    cmd.insert(TnuaProximitySensor {
        cast_origin: Vec3::ZERO,
        cast_direction: -Vec3::Y,
        cast_range: 3.0,
        velocity: Vec3::ZERO,
        output: None,
    });
    cmd.insert(TnuaMotor::default());
    cmd.insert(TnuaPlatformerConfig {
        spring_strengh: 100.0,
        spring_dampening: 10.0,
        acceleration: 20.0,
    });
    cmd.insert(TnuaPlatformerControls::new_floating_at(2.0));
    cmd.insert(common::ui::TrackedEntity("Player".to_owned()));
    cmd.insert(SpeedControl(10.0));
}

fn apply_controls(
    mut query: Query<(&mut TnuaPlatformerControls, &SpeedControl)>,
    keyboard: Res<Input<KeyCode>>,
) {
    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::Left) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::Right) {
        direction += Vec3::X;
    }

    for (mut controls, &SpeedControl(speed)) in query.iter_mut() {
        controls.move_direction = direction * speed;
    }
}
