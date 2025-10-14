use bevy::{
    color::palettes::css,
    ecs::system::{EntityCommands, SystemParam},
    prelude::*,
};

#[cfg(feature = "avian2d")]
use avian2d::prelude as avian;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::prelude as rapier;

use bevy_tnua::math::Vector2;
#[allow(unused_imports)]
use bevy_tnua::math::{AsF32, Float};

use crate::levels_setup::LevelObject;

#[derive(SystemParam, Deref, DerefMut)]
pub struct LevelSetupHelper2d<'w, 's> {
    #[deref]
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
}

impl LevelSetupHelper2d<'_, '_> {
    pub fn spawn_named(&'_ mut self, name: impl ToString) -> EntityCommands<'_> {
        self.commands
            .spawn((LevelObject, Name::new(name.to_string())))
    }

    pub fn spawn_floor(&'_ mut self, color: impl Into<Color>) -> EntityCommands<'_> {
        let mut cmd = self.spawn_named("Floor");
        cmd.insert(Sprite {
            custom_size: Some(Vec2::new(128.0, 0.5)),
            color: color.into(),
            ..Default::default()
        });

        #[cfg(feature = "rapier2d")]
        cmd.insert(rapier::Collider::halfspace(Vec2::Y).unwrap());
        #[cfg(feature = "avian2d")]
        {
            cmd.insert(avian::RigidBody::Static);
            cmd.insert(avian::Collider::half_space(Vector2::Y));
        }

        cmd
    }

    pub fn spawn_rectangle(
        &'_ mut self,
        name: impl ToString,
        color: impl Into<Color>,
        transform: Transform,
        size: Vector2,
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_named(name);

        cmd.insert((
            Sprite {
                custom_size: Some(size.f32()),
                color: color.into(),
                ..Default::default()
            },
            transform,
        ));

        #[cfg(feature = "rapier2d")]
        cmd.insert(rapier::Collider::cuboid(
            0.5 * size.x.f32(),
            0.5 * size.y.f32(),
        ));
        #[cfg(feature = "avian2d")]
        {
            cmd.insert(avian::RigidBody::Static);
            cmd.insert(avian::Collider::rectangle(size.x, size.y));
        }

        cmd
    }

    pub fn spawn_compound_rectangles(
        &'_ mut self,
        name: impl ToString,
        color: impl Into<Color>,
        transform: Transform,
        parts: &[(Vector2, Float, Vector2)],
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_named(name);

        cmd.insert((
            //Sprite {
            //custom_size: Some(size.f32()),
            //color: color.into(),
            //..Default::default()
            //},
            transform,
        ));
        let color = color.into();

        cmd.with_children(|commands| {
            for (pos, rot, size) in parts.iter().copied() {
                commands.spawn((
                    Sprite {
                        custom_size: Some(size.f32()),
                        color,
                        ..Default::default()
                    },
                    Transform {
                        translation: pos.extend(0.0).f32(),
                        rotation: Quat::from_rotation_z(rot.f32()),
                        scale: Vec3::ONE,
                    },
                ));
            }
        });

        #[cfg(feature = "rapier2d")]
        cmd.insert(rapier::Collider::compound(
            parts
                .iter()
                .map(|&(pos, rot, size)| {
                    (
                        pos,
                        rot,
                        rapier::Collider::cuboid(0.5 * size.x, 0.5 * size.y),
                    )
                })
                .collect(),
        ));
        #[cfg(feature = "avian2d")]
        {
            cmd.insert(avian::RigidBody::Static);
            cmd.insert(avian::Collider::compound(
                parts
                    .iter()
                    .map(|&(pos, rot, size)| (pos, rot, avian::Collider::rectangle(size.x, size.y)))
                    .collect(),
            ));
        }

        cmd
    }

    pub fn spawn_text_circle(
        &'_ mut self,
        name: impl ToString,
        text: impl ToString,
        text_scale: Float,
        transform: Transform,
        #[allow(unused)] radius: Float,
    ) -> EntityCommands<'_> {
        let font = self.asset_server.load("FiraSans-Bold.ttf");
        let child = self
            .spawn((
                LevelObject,
                Text::new(text.to_string()),
                TextLayout::new_with_justify(Justify::Center),
                TextFont {
                    font,
                    font_size: 72.0,
                    ..default()
                },
                TextColor(css::WHITE.into()),
                Transform::from_xyz(0.0, 0.0, 1.0).with_scale(text_scale.f32() * Vec3::ONE),
            ))
            .id();
        let mut cmd = self.spawn_named(name);
        cmd.add_child(child);
        cmd.insert((
            transform,
            #[cfg(feature = "rapier2d")]
            rapier::Collider::ball(radius),
            #[cfg(feature = "avian2d")]
            (avian::RigidBody::Static, avian::Collider::circle(radius)),
        ));
        cmd
    }

    pub fn spawn_dynamic_rectangle(
        &'_ mut self,
        name: impl ToString,
        color: impl Into<Color>,
        transform: Transform,
        size: Vector2,
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_named(name);

        cmd.insert((
            Sprite {
                custom_size: Some(size.f32()),
                color: color.into(),
                ..Default::default()
            },
            transform,
        ));

        #[cfg(feature = "rapier2d")]
        {
            cmd.insert(rapier::RigidBody::Dynamic);
            cmd.insert(rapier::Collider::cuboid(
                0.5 * size.x.f32(),
                0.5 * size.y.f32(),
            ));
        }
        #[cfg(feature = "avian2d")]
        {
            cmd.insert(avian::RigidBody::Dynamic);
            cmd.insert(avian::Collider::rectangle(size.x, size.y));
        }

        cmd
    }

    pub fn spawn_circle(
        &'_ mut self,
        name: impl ToString,
        //color: impl Into<Color>,
        transform: Transform,
        #[allow(unused)] radius: Float,
    ) -> EntityCommands<'_> {
        let mut cmd = self.spawn_named(name);

        cmd.insert((
            // Sprite {
            // custom_size: Some(size.f32()),
            // color: color.into(),
            // ..Default::default()
            // },
            transform,
        ));

        cmd.insert((
            #[cfg(feature = "rapier2d")]
            rapier::Collider::ball(radius),
            #[cfg(feature = "avian2d")]
            (avian::RigidBody::Static, avian::Collider::circle(radius)),
        ));

        cmd
    }
}

pub trait LevelSetupHelper2dEntityCommandsExtension {
    fn make_kinematic(&mut self) -> &mut Self;
    fn make_sensor(&mut self) -> &mut Self;
}

impl LevelSetupHelper2dEntityCommandsExtension for EntityCommands<'_> {
    fn make_kinematic(&mut self) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian2d")]
            avian::RigidBody::Kinematic,
            #[cfg(feature = "rapier2d")]
            (
                rapier::Velocity::default(),
                rapier::RigidBody::KinematicVelocityBased,
            ),
        ))
    }

    fn make_sensor(&mut self) -> &mut Self {
        self.insert((
            #[cfg(feature = "avian2d")]
            avian::Sensor,
            #[cfg(feature = "rapier2d")]
            rapier::Sensor,
        ))
    }
}
