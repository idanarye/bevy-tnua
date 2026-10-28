use bevy::{color::palettes::css, prelude::*};

#[cfg(feature = "avian2d")]
use avian2d::prelude as avian;
#[cfg(feature = "rapier2d")]
use bevy_rapier2d::prelude as rapier;

use bevy_tnua::math::Vector2;
#[allow(unused_imports)]
use bevy_tnua::math::{AsF32, Float};

use crate::levels_setup::LevelObject;

pub fn the_floor(color: impl Into<Color>) -> impl Scene {
    bsn! {
        #Floor
        Sprite {
            custom_size: Vec2::new(128.0, 0.5),
            color: color,
        }
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Collider::halfspace(Vec2::Y).unwrap()
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Static
                    avian::Collider::half_space(Vec2::Y)
            }
        }
    }
}

pub fn a_rectangle(color: impl Into<Color>, size: Vector2) -> impl Scene {
    bsn! {
        Sprite {
            custom_size: {size.f32()},
            color: color,
        }

        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Collider::cuboid(
                    0.5 * size.x.f32(),
                    0.5 * size.y.f32(),
                )
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::rectangle(size.x.f32(), size.y.f32())
            }
        }
    }
}

pub fn some_compound_rectangles<const N: usize>(
    color: impl Into<Color>,
    parts: [(Vector2, Float, Vector2); N],
) -> impl Scene {
    let color = color.into();
    bsn! {
        Children [{
            parts.iter().copied().map(|(pos, rot, size)| {
                bsn! {
                    Sprite {
                        custom_size: {size.f32()},
                        color,
                    }
                    Transform {
                        translation: {pos.extend(0.0).f32()},
                        rotation: Quat::from_rotation_z(rot.f32()),
                        scale: Vec3::ONE,
                    }
                }
            }).collect::<Vec<_>>()
        }]
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Collider::compound(
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
                )
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::compound(
                    parts
                    .iter()
                    .map(|&(pos, rot, size)| {
                        (
                            avian::Position(pos),
                            rot.f32(),
                            avian::Collider::rectangle(size.x.f32(), size.y.f32()),
                        )
                    })
                    .collect(),
                )
            }
        }
    }
}

pub fn a_text_circle(
    text: impl ToString,
    text_scale: Float,
    #[allow(unused)] radius: Float,
) -> impl Scene {
    let text = text.to_string();
    bsn! {
        Children [
            LevelObject
            Text::new(text)
            TextLayout::justify(Justify::Center)
            TextFont {
                // font: {FontSource::Family("FiraSans-Bold.ttf".into())},
                font_size: FontSize::Px(72.0),
            }
            TextColor(css::WHITE)
            Transform::from_xyz(0.0, 0.0, 1.0).with_scale(text_scale.f32() * Vec3::ONE)
        ]
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Collider::ball(radius)
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::circle(radius.f32())
            }
        }
    }
}

pub fn a_circle(#[allow(unused)] radius: Float) -> impl Scene {
    bsn! {
        @{
        #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Collider::ball(radius)
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::circle(radius.f32())
            },
        }
    }
}

pub fn make_kinematic() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Kinematic
            }
        }
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Velocity
                rapier::RigidBody::KinematicVelocityBased
            }
        }
    }
}

pub fn make_dynamic() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::RigidBody::Dynamic
            }
        }
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Velocity
                rapier::RigidBody::Dynamic
            }
        }
    }
}

pub fn make_sensor() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::Sensor
            }
        }
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::Sensor
            }
        }
    }
}

pub fn make_intangible() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "rapier2d")]
            bsn! {
                rapier::SolverGroups {
                    memberships: rapier::Group::empty(),
                    filters: rapier::Group::empty(),
                }
            }
        }
        @{
            #[cfg(feature = "avian2d")]
            bsn! {
                avian::CollisionLayers::new(
                    [crate::levels_setup::for_2d_platformer::LayerNames::FallThrough],
                    [crate::levels_setup::for_2d_platformer::LayerNames::FallThrough],
                )
            }
        }
    }
}
