use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_tnua::{TnuaProximitySensor, TnuaRapier3dPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugin(RapierDebugRenderPlugin::default());
    app.add_plugin(TnuaRapier3dPlugin);
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_level);
    app.add_startup_system(setup_player);
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 9.0, 30.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });
}

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut cmd = commands.spawn();
    cmd.insert_bundle(PbrBundle {
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
        let mut cmd = commands.spawn();
        cmd.insert_bundle(PbrBundle {
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
    let mut cmd = commands.spawn();
    cmd.insert_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Capsule {
            radius: 0.5,
            rings: 10,
            depth: 1.0,
            latitudes: 10,
            longitudes: 10,
            uv_profile: shape::CapsuleUvProfile::Aspect,
        })),
        material: materials.add(Color::YELLOW.into()),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..Default::default()
    });
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(Collider::capsule_y(0.5, 0.5));
    cmd.insert(TnuaProximitySensor::new(Vec3::ZERO, -1.0 * Vec3::Y));
}
