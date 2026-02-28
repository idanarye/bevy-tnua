use bevy::{color::palettes::css, prelude::*};

use avian3d::prelude::*;

use bevy_tnua::builtins::{
    TnuaBuiltinJump, TnuaBuiltinJumpConfig, TnuaBuiltinWalk, TnuaBuiltinWalkConfig,
    TnuaBuiltinWallSlide, TnuaBuiltinWallSlideConfig,
};
use bevy_tnua::radar_lens::{TnuaBlipSpatialRelation, TnuaRadarLens};
use bevy_tnua::{TnuaObstacleRadar, prelude::*};
use bevy_tnua_avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            TnuaControllerPlugin::<ControlScheme>::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
        ))
        .add_systems(
            Startup,
            (setup_camera_and_lights, setup_level, setup_player),
        )
        .add_systems(Update, apply_controls.in_set(TnuaUserControlsSystems))
        .run();
}

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
enum ControlScheme {
    Jump(TnuaBuiltinJump),
    WallSlide(TnuaBuiltinWallSlide, Entity),
    WallJump(TnuaBuiltinJump),
}

// No Tnua-related setup here - this is just normal Bevy stuff.
fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

    // A directly-down light to tell where the player is going to land.
    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::default().looking_at(-Vec3::Y, Vec3::Z),
    ));
}

// No Tnua-related setup here - this is just normal Bevy (and Avian) stuff.
fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(128.0, 128.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));

    // Spawn walls for the player to slide and jump on
    for x in [-4.0, 4.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 20.0, 6.0))),
            MeshMaterial3d(materials.add(Color::from(css::GRAY))),
            Transform::from_xyz(x, 10.0, 0.0),
            RigidBody::Static,
            Collider::cuboid(1.0, 20.0, 6.0),
        ));
    }
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut control_scheme_configs: ResMut<Assets<ControlSchemeConfig>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d {
            radius: 0.5,
            half_length: 0.5,
        })),
        MeshMaterial3d(materials.add(Color::from(css::DARK_CYAN))),
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        TnuaController::<ControlScheme>::default(),
        TnuaConfig::<ControlScheme>(control_scheme_configs.add(ControlSchemeConfig {
            basis: TnuaBuiltinWalkConfig {
                float_height: 1.5,
                ..Default::default()
            },
            jump: TnuaBuiltinJumpConfig {
                height: 6.0,
                ..Default::default()
            },
            wall_slide: TnuaBuiltinWallSlideConfig {
                max_fall_speed: 0.5,
                max_sideways_speed: 0.0,
                ..Default::default()
            },
            wall_jump: TnuaBuiltinJumpConfig {
                height: 6.0,
                horizontal_distance: 5.0,
                ..Default::default()
            },
        })),
        TnuaObstacleRadar::new(0.6, 1.0),
        TnuaAvian3dSensorShape(Collider::cylinder(0.49, 0.0)),
        LockedAxes::ROTATION_LOCKED,
    ));
}

fn apply_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut TnuaController<ControlScheme>, &TnuaObstacleRadar)>,
    spatial_ext: TnuaSpatialExtAvian3d,
) {
    let Ok((mut controller, obstacle_radar)) = query.single_mut() else {
        return;
    };
    controller.initiate_action_feeding();

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowUp) {
        direction -= Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction -= Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction += Vec3::X;
    }

    controller.basis = TnuaBuiltinWalk {
        desired_motion: direction.normalize_or_zero(),
        ..Default::default()
    };

    let radar_lens = TnuaRadarLens::new(obstacle_radar, &spatial_ext);

    let currently_sliding_on =
        if let Some(ControlSchemeActionState::WallSlide(_, entity)) = &controller.current_action {
            Some(*entity)
        } else {
            None
        };

    for blip in radar_lens.iter_blips() {
        if let TnuaBlipSpatialRelation::Aeside(blip_direction) = blip.spatial_relation(0.5) {
            let should_slide = if currently_sliding_on == Some(blip.entity()) {
                -0.5 < blip_direction.dot(direction)
            } else {
                0.5 < blip_direction.dot(direction)
            };
            if should_slide && let Ok(normal) = Dir3::new(blip.normal_from_closest_point()) {
                controller.action(ControlScheme::WallSlide(
                    TnuaBuiltinWallSlide {
                        contact_point_with_wall: blip.closest_point().get(),
                        normal,
                        // If we had an actual model, this could be used to align its direction.
                        force_forward: None,
                    },
                    blip.entity(),
                ));
            }
        }
    }

    if keyboard.pressed(KeyCode::Space) {
        if matches!(
            controller.action_discriminant(),
            Some(ControlSchemeActionDiscriminant::Jump | ControlSchemeActionDiscriminant::WallJump)
        ) {
            controller.prolong_action();
        } else if let Some(ControlSchemeActionState::WallSlide(state, _)) =
            &controller.current_action
        {
            let wall_normal = *state.input.normal;
            controller.action(ControlScheme::WallJump(TnuaBuiltinJump {
                horizontal_displacement: Some(wall_normal),
                // Wall jumps are considered in-air because even though there is a wall - there is
                // no ground.
                allow_in_air: true,
                // If we had an actual model, this could be used to set its direction.
                force_forward: None,
            }));
        } else {
            controller.action(ControlScheme::Jump(Default::default()));
        }
    }
}
