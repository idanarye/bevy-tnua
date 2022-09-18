use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    app.add_plugin(RapierDebugRenderPlugin::default());
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_level);
    app.add_startup_system(setup_player);
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 2.0, 10.0)
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
        material: materials.add(Color::WHITE.into()),
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..Default::default()
    });
    cmd.insert(RigidBody::Dynamic);
    cmd.insert(Collider::capsule_y(0.5, 0.5));
}
