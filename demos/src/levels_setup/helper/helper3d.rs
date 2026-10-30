use bevy::prelude::*;

#[cfg(feature = "avian3d")]
use avian3d::prelude as avian;
#[cfg(feature = "rapier3d")]
use bevy_rapier3d::prelude as rapier;

use bevy_tnua::math::{AsF32, Float, Quaternion, Vector3};

pub fn the_floor(color: impl Into<Color>) -> impl Scene {
    bsn! {
        #Floor
        Mesh3d(asset_value(Plane3d::default().mesh().size(128.0, 128.0)))
        MeshMaterial3d<StandardMaterial>(asset_value(color.into()))
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::halfspace(Vec3::Y).unwrap()
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::half_space(Vector3::Y.f32())
            }
        }
    }
}

pub fn a_cuboid(color: impl Into<Color>, size: Vector3) -> impl Scene {
    bsn! {
        Mesh3d(asset_value(Cuboid::from_size(size.f32())))
        MeshMaterial3d<StandardMaterial>(asset_value(color.into()))
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::cuboid(0.5 * size.x.f32(), 0.5 * size.y.f32(), 0.5 * size.z.f32())
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::cuboid(size.x.f32(), size.y.f32(), size.z.f32())
            }
        }
    }
}

pub fn some_compound_cuboids<const N: usize>(
    color: impl Into<Color>,
    parts: [(Vector3, Quaternion, Vector3); N],
) -> impl Scene {
    let color = color.into();
    bsn! {
        Children [{
            parts.iter().copied().map(|(pos, rot, size)| {
                bsn! {
                    Transform {
                        translation: {pos.f32()},
                        rotation: {rot.f32()},
                        scale: Vec3::ONE,
                    }
                    Mesh3d(asset_value(Cuboid::from_size(size.f32())))
                        MeshMaterial3d<StandardMaterial>(asset_value(color))
                }
            }).collect::<Vec<_>>()
        }]
        @{
            #[cfg(feature = "rapier3d")]
            bsn!{
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
                )
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::compound(
                    parts
                    .iter()
                    .map(|&(pos, rot, size)| {
                        (
                            pos,
                            rot.f32(),
                            avian::Collider::cuboid(size.x.f32(), size.y.f32(), size.z.f32()),
                        )
                    })
                    .collect(),
                )
            }
        }
    }
}

pub fn an_invisible_cuboid(#[allow(unused)] size: Vector3) -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::cuboid(
                    0.5 * size.x.f32(),
                    0.5 * size.y.f32(),
                    0.5 * size.z.f32(),
                )
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::cuboid(
                        size.x.f32(),
                        size.y.f32(),
                        size.z.f32(),
                )
            }
        }
    }
}

pub fn a_cylinder(color: impl Into<Color>, radius: Float, half_height: Float) -> impl Scene {
    bsn! {
        Mesh3d(asset_value(Cylinder {
            radius: radius.f32(),
            half_height: half_height.f32(),
        }))
        MeshMaterial3d<StandardMaterial>(asset_value(color.into()))
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::cylinder(half_height, radius)
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::cylinder(radius.f32(), 2.0 * half_height.f32())
            }
        }
    }
}

pub fn a_ball(color: impl Into<Color>, radius: Float) -> impl Scene {
    bsn! {
        Mesh3d(asset_value(Sphere {
            radius: radius.f32(),
        }))
        MeshMaterial3d<StandardMaterial>(asset_value(color.into()))
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::ball(radius)
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::sphere(radius.f32())
            }
        }
    }
}

pub fn an_invisible_ball(#[allow(unused)] radius: Float) -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Collider::ball(radius)
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Static
                avian::Collider::sphere(radius.f32())
            }
        }
    }
}

pub fn make_kinematic() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Kinematic
            }
        }
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Velocity
                    rapier::RigidBody::KinematicVelocityBased
            }
        }
    }
}

pub fn make_kinematic_with_linear_velocity(velocity: Vector3) -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::LinearVelocity({velocity.f32()})
                avian::RigidBody::Kinematic
            }
        }
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Velocity::linear(velocity)
                rapier::RigidBody::KinematicVelocityBased
            }
        }
    }
}

pub fn make_kinematic_with_angular_velocity(angvel: Vector3) -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::AngularVelocity({angvel.f32()})
                avian::RigidBody::Kinematic
            }
        }
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Velocity::angular(angvel)
                rapier::RigidBody::KinematicVelocityBased
            }
        }
    }
}

pub fn make_dynamic() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::RigidBody::Dynamic
            }
        }
        @{
            #[cfg(feature = "rapier3d")]
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
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::Sensor
            }
        }
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::Sensor
            }
        }
    }
}

pub fn make_intangible() -> impl Scene {
    bsn! {
        @{
            #[cfg(feature = "rapier3d")]
            bsn! {
                rapier::SolverGroups {
                    memberships: rapier::Group::empty(),
                    filters: rapier::Group::empty(),
                }
            }
        }
        @{
            #[cfg(feature = "avian3d")]
            bsn! {
                avian::CollisionLayers::new(
                    [crate::levels_setup::for_3d_platformer::LayerNames::FallThrough],
                    [crate::levels_setup::for_3d_platformer::LayerNames::FallThrough],
                )
            }
        }
    }
}
