use bevy::{color::palettes::css, prelude::*};
use bevy_tnua::math::Vector3;

//#[cfg(feature = "avian3d")]
//use avian3d::{prelude as avian, prelude::*};
//#[cfg(feature = "rapier3d")]
//use bevy_rapier3d::{prelude as rapier, prelude::*};

use crate::level_mechanics::{Cannon, CannonBullet, PushEffect, TimeToDespawn};

use super::{
    helper::{LevelSetupHelper3d, LevelSetupHelper3dEntityCommandsExtension},
    PositionPlayer,
};

pub fn setup_level(mut helper: LevelSetupHelper3d) {
    helper.spawn(PositionPlayer::from(Vec3::new(0.0, 10.0, 0.0)));

    helper.spawn_floor(css::WHITE);

    let bullet_mesh = helper.meshes.add(Sphere { radius: 0.2 });
    let bullet_material = helper.materials.add(Color::from(css::SILVER));

    helper
        .with_color(css::RED)
        .spawn_mesh_without_physics(
            "Cannon",
            Transform::from_xyz(10.0, 2.0, 0.0)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ConicalFrustum {
                radius_top: 0.2,
                radius_bottom: 0.5,
                height: 2.0,
            },
        )
        .insert(Cannon {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            cmd: Box::new(move |cmd| {
                cmd.make_kinematic_with_linear_velocity(-20.0 * Vector3::X);
                cmd.make_sensor();
                cmd.add_ball_collider(0.2);
                cmd.insert(Mesh3d(bullet_mesh.clone()));
                cmd.insert(MeshMaterial3d(bullet_material.clone()));
                cmd.insert(TimeToDespawn::from_seconds(10.0));
                cmd.insert(CannonBullet::new_with_effect(|cmd| {
                    cmd.insert(PushEffect::Impulse(-20.0 * Vector3::X));
                }));
            }),
        });
}
