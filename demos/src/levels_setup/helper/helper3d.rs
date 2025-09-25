use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    prelude::*,
};

#[cfg(feature = "avian3d")]
use avian3d::prelude as avian;
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::prelude as rapier;

use bevy_tnua::math::{AsF32, Float, Quaternion, Vector3};

use crate::levels_setup::LevelObject;

#[derive(SystemParam, Deref, DerefMut)]
pub struct LevelSetupHelper3d<'w, 's> {
    #[deref]
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
}

impl<'w, 's> LevelSetupHelper3d<'w, 's> {
    pub fn spawn_named(&'_ mut self, name: impl ToString) -> EntityCommands<'_> {
        self.commands
            .spawn((LevelObject, Name::new(name.to_string())))
    }

    pub fn spawn_floor(&'_ mut self, color: impl Into<Color>) -> EntityCommands<'_> {
        let mesh = self
            .meshes
            .add(Plane3d::default().mesh().size(128.0, 128.0));
        let material = self.materials.add(color.into());
        let mut cmd = self.spawn_named("Floor");
        cmd.insert((Mesh3d(mesh), MeshMaterial3d(material)));

        #[cfg(feature = "rapier3d")]
        cmd.insert(rapier::Collider::halfspace(Vec3::Y).unwrap());
        #[cfg(feature = "avian3d")]
        {
            cmd.insert(avian::RigidBody::Static);
            cmd.insert(avian::Collider::half_space(Vector3::Y));
        }

        cmd
    }

    pub fn with_material<'a>(
        &'a mut self,
        material: impl Into<StandardMaterial>,
    ) -> LevelSetupHelper3dWithMaterial<'a, 'w, 's> {
        let material = self.materials.add(material);
        LevelSetupHelper3dWithMaterial {
            parent: self,
            material,
        }
    }

    pub fn with_color<'a>(
        &'a mut self,
        color: impl Into<Color>,
    ) -> LevelSetupHelper3dWithMaterial<'a, 'w, 's> {
        self.with_material(color.into())
    }

    pub fn spawn_scene_cuboid(
        &'_ mut self,
        name: impl ToString,
        path: impl ToString,
        transform: Transform,
        #[allow(unused)] size: Vector3,
    ) -> EntityCommands<'_> {
        let scene = self.asset_server.load(path.to_string());
        let mut cmd = self.spawn_named(name);

        cmd.insert((SceneRoot(scene), transform));

        #[cfg(feature = "rapier3d")]
        cmd.insert(rapier::Collider::cuboid(
            0.5 * size.x.f32(),
            0.5 * size.y.f32(),
            0.5 * size.z.f32(),
        ));
        #[cfg(feature = "avian3d")]
        {
            cmd.insert(avian::RigidBody::Static);
            cmd.insert(avian::Collider::cuboid(size.x, size.y, size.z));
        }

        cmd
    }
}

pub struct LevelSetupHelper3dWithMaterial<'a, 'w, 's> {
    parent: &'a mut LevelSetupHelper3d<'w, 's>,
    material: Handle<StandardMaterial>,
}

impl LevelSetupHelper3dWithMaterial<'_, '_, '_> {
    pub fn spawn_mesh_without_physics(
        &'_ mut self,
        name: impl ToString,
        transform: Transform,
        mesh: impl Into<Mesh>,
    ) -> EntityCommands<'_> {
        let mesh = self.parent.meshes.add(mesh);
        let mut cmd = self.parent.spawn_named(name);
        cmd.insert((
            Mesh3d(mesh),
            MeshMaterial3d(self.material.clone()),
            transform,
        ));
        cmd
    }

    pub fn spawn_cuboid(
        &'_ mut self,
        name: impl ToString,
        transform: Transform,
        size: Vector3,
    ) -> EntityCommands<'_> {
        let mut cmd =
            self.spawn_mesh_without_physics(name, transform, Cuboid::from_size(size.f32()));

        cmd.insert((
            #[cfg(feature = "rapier3d")]
            rapier::Collider::cuboid(0.5 * size.x.f32(), 0.5 * size.y.f32(), 0.5 * size.z.f32()),
            #[cfg(feature = "avian3d")]
            (
                avian::RigidBody::Static,
                avian::Collider::cuboid(size.x, size.y, size.z),
            ),
        ));

        cmd
    }

    pub fn spawn_compound_cuboids(
        &'_ mut self,
        name: impl ToString,
        transform: Transform,
        parts: &[(Vector3, Quaternion, Vector3)],
    ) -> EntityCommands<'_> {
        let child_entity_ids = parts
            .iter()
            .map(|&(pos, rot, size)| {
                self.parent
                    .commands
                    .spawn((
                        Transform {
                            translation: pos.f32(),
                            rotation: rot.f32(),
                            scale: Vec3::ONE,
                        },
                        Mesh3d(self.parent.meshes.add(Cuboid::from_size(size.f32()))),
                        MeshMaterial3d(self.material.clone()),
                    ))
                    .id()
            })
            .collect::<Vec<_>>();

        let mut cmd = self.parent.spawn_named(name);
        cmd.insert(transform);
        cmd.add_children(&child_entity_ids);
        // self.spawn_mesh_without_physics(name, transform, Cuboid::from_size(size.f32()));

        cmd.insert((
            #[cfg(feature = "rapier3d")]
            rapier::Collider::compound(
                parts
                    .iter()
                    .map(|&(pos, rot, size)| {
                        (
                            pos,
                            rot,
                            rapier::Collider::cuboid(
                                0.5 * size.x.f32(),
                                0.5 * size.y.f32(),
                                0.5 * size.z.f32(),
                            ),
                        )
                    })
                    .collect(),
            ),
            #[cfg(feature = "avian3d")]
            (
                avian::RigidBody::Static,
                avian::Collider::compound(
                    parts
                        .iter()
                        .map(|&(pos, rot, size)| {
                            (pos, rot, avian::Collider::cuboid(size.x, size.y, size.z))
                        })
                        .collect(),
                ),
            ),
        ));

        cmd
    }

    pub fn spawn_cylinder(
        &'_ mut self,
        name: impl ToString,
        transform: Transform,
        radius: Float,
        half_height: Float,
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_mesh_without_physics(
            name,
            transform,
            Cylinder {
                radius: radius.f32(),
                half_height: half_height.f32(),
            },
        );

        cmd.insert((
            #[cfg(feature = "rapier3d")]
            rapier::Collider::cylinder(half_height, radius),
            #[cfg(feature = "avian3d")]
            (
                avian::RigidBody::Static,
                avian::Collider::cylinder(radius, 2.0 * half_height),
            ),
        ));

        cmd
    }

    pub fn spawn_dynamic_ball(
        &'_ mut self,
        name: impl ToString,
        transform: Transform,
        radius: Float,
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_mesh_without_physics(
            name,
            transform,
            Sphere {
                radius: radius.f32(),
            },
        );

        cmd.insert((
            #[cfg(feature = "rapier3d")]
            (rapier::RigidBody::Dynamic, rapier::Collider::ball(radius)),
            #[cfg(feature = "avian3d")]
            (avian::RigidBody::Dynamic, avian::Collider::sphere(radius)),
        ));

        cmd
    }

    pub fn spawn_ball(
        &'_ mut self,
        name: impl ToString,
        transform: Transform,
        radius: Float,
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_mesh_without_physics(
            name,
            transform,
            Sphere {
                radius: radius.f32(),
            },
        );

        cmd.insert((
            #[cfg(feature = "rapier3d")]
            rapier::Collider::ball(radius),
            #[cfg(feature = "avian3d")]
            (avian::RigidBody::Static, avian::Collider::sphere(radius)),
        ));

        cmd
    }
}

pub trait LevelSetupHelper3dEntityCommandsExtension {
    fn make_kinematic(&mut self) -> &mut Self;
    fn make_kinematic_with_linear_velocity(&mut self, velocity: Vector3) -> &mut Self;
    fn make_kinematic_with_angular_velocity(&mut self, angvel: Vector3) -> &mut Self;
    fn add_ball_collider(&mut self, radius: Float) -> &mut Self;
    fn make_sensor(&mut self) -> &mut Self;
}

impl LevelSetupHelper3dEntityCommandsExtension for EntityCommands<'_> {
    fn make_kinematic(&mut self) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian3d")]
            avian::RigidBody::Kinematic,
            #[cfg(feature = "rapier3d")]
            (
                rapier::Velocity::default(),
                rapier::RigidBody::KinematicVelocityBased,
            ),
        ))
    }

    fn make_kinematic_with_linear_velocity(
        &mut self,
        #[allow(unused)] velocity: Vector3,
    ) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian3d")]
            (avian::LinearVelocity(velocity), avian::RigidBody::Kinematic),
            #[cfg(feature = "rapier3d")]
            (
                rapier::Velocity::linear(velocity),
                rapier::RigidBody::KinematicVelocityBased,
            ),
        ))
    }

    fn make_kinematic_with_angular_velocity(
        &mut self,
        #[allow(unused)] angvel: Vector3,
    ) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian3d")]
            (avian::AngularVelocity(angvel), avian::RigidBody::Kinematic),
            #[cfg(feature = "rapier3d")]
            (
                rapier::Velocity::angular(angvel),
                rapier::RigidBody::KinematicVelocityBased,
            ),
        ))
    }

    fn add_ball_collider(&mut self, #[allow(unused)] radius: Float) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian3d")]
            avian::Collider::sphere(radius),
            #[cfg(feature = "rapier3d")]
            rapier::Collider::ball(radius),
        ))
    }

    fn make_sensor(&mut self) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian3d")]
            avian::Sensor,
            #[cfg(feature = "rapier3d")]
            rapier::Sensor,
        ))
    }
}
