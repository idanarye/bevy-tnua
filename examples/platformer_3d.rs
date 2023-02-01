mod common;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_tnua::{
    TnuaPlatformerBundle, TnuaPlatformerConfig, TnuaPlatformerControls, TnuaPlatformerPlugin,
    TnuaRapier3dPlugin,
};

use self::common::ui::ControlFactors;
use self::common::ui_plotting::PlotSource;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugin(RapierDebugRenderPlugin::default());
    app.add_plugin(TnuaRapier3dPlugin);
    app.add_plugin(TnuaPlatformerPlugin);
    app.add_plugin(common::ui::ExampleUi);
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_level);
    app.add_startup_system(setup_player);
    app.add_system(apply_controls);
    app.add_system(update_plot_data);
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
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 14.0, 30.0)
            .looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });
}

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 128.0 })),
        material: materials.add(Color::WHITE.into()),
        ..Default::default()
    });
    cmd.insert(Collider::halfspace(Vec3::Y).unwrap());

    let obstacles_material = materials.add(Color::GRAY.into());
    for ([width, height, depth], transform) in [
        ([4.0, 1.0, 2.0], Transform::from_xyz(3.0, 1.0, 0.0)),
        (
            [6.0, 0.1, 2.0],
            Transform::from_xyz(-3.0, 1.0, 0.0).with_rotation(Quat::from_rotation_z(-0.6)),
        ),
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
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut cmd = commands.spawn_empty();
    cmd.insert(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Capsule {
            radius: 0.5,
            rings: 10,
            depth: 1.0,
            latitudes: 10,
            longitudes: 10,
            uv_profile: shape::CapsuleUvProfile::Aspect,
        })),
        material: materials.add(Color::YELLOW.into()),
        transform: Transform::from_xyz(0.0, 10.0, 0.0),
        ..Default::default()
    });
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(LockedAxes::ROTATION_LOCKED); // todo: fix with torque
    cmd.insert(Velocity::default());
    cmd.insert(Collider::capsule_y(0.5, 0.5));
    cmd.insert(TnuaPlatformerBundle::new_with_config(
        TnuaPlatformerConfig {
            float_height: 2.0,
            cling_distance: 1.0,
            spring_strengh: 100.0,
            spring_dampening: 10.0,
            acceleration: 20.0,
            jump_impulse: 30.0,
            jump_height_reached_fall_speed: -1.0,
            jump_height_reached_acceleration: 50.0,
            jump_shorted_fall_speed: 10.0,
            jump_shorted_acceleration: 70.0,
        },
    ));
    cmd.insert(common::ui::TrackedEntity("Player".to_owned()));
    cmd.insert(PlotSource::default());
    cmd.insert(ControlFactors {
        speed: 10.0,
        jump_height: 4.0,
    });
}

fn apply_controls(
    mut query: Query<(&mut TnuaPlatformerControls, &ControlFactors)>,
    keyboard: Res<Input<KeyCode>>,
) {
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

    let jump = keyboard.pressed(KeyCode::Space);

    for (mut controls, &ControlFactors { speed, jump_height }) in query.iter_mut() {
        controls.move_direction = direction * speed;
        controls.jump = jump.then(|| jump_height);
    }
}
