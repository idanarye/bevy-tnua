use bevy::pbr::prelude::MeshMaterial3d;
use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector3;

//#[cfg(feature = "avian3d")]
//use avian3d::{prelude as avian, prelude::*};
//#[cfg(feature = "rapier3d")]
//use bevy_rapier3d::{prelude as rapier, prelude::*};

use crate::level_mechanics::{Cannon, CannonBullet, PushEffect, TimeToDespawn};
use crate::levels_setup::LevelObject;
use crate::levels_setup::helper::helper3d::{
    an_invisible_ball, make_kinematic_with_linear_velocity, make_sensor, the_floor,
};

use super::PositionPlayer;

pub fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bullet_mesh = meshes.add(Sphere { radius: 0.2 });
    let bullet_material = materials.add(Color::from(css::SILVER));

    commands.spawn_scene_list(bsn_list! {
        PositionPlayer(Vec3::new(0.0, 10.0, 0.0))
        --
        @the_floor(css::WHITE)
        LevelObject
        --
        #Cannon
        Transform::from_xyz(10.0, 2.0, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
        MeshMaterial3d<StandardMaterial>(asset_value(Color::from(css::RED)))
        Mesh3d(asset_value(ConicalFrustum {
            radius_top: 0.2,
            radius_bottom: 0.5,
            height: 2.0,
        }))
        template(move |_| {
            let bullet_mesh = bullet_mesh.clone();
            let bullet_material = bullet_material.clone();
            Ok(Cannon {
                timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                cmd: Box::new(move |cmd| {
                    cmd.apply_scene(bsn! {
                        @an_invisible_ball(0.2)
                        @make_kinematic_with_linear_velocity(-20.0 * Vector3::X)
                        @make_sensor()
                        Mesh3d({bullet_mesh.clone()})
                        MeshMaterial3d<StandardMaterial>({bullet_material.clone()})
                        TimeToDespawn::from_seconds(10.0)
                        template(|_| Ok(CannonBullet::new_with_effect(|cmd| {
                            cmd.insert(PushEffect::Impulse(-20.0 * Vector3::X));
                        })))
                    });
                }),
            })
        })
    });
}
