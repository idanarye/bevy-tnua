use bevy::prelude::*;
use bevy_tnua::math::{AdjustPrecision, TargetFloat, TargetVec3};

pub struct MovingPlatformPlugin;

impl Plugin for MovingPlatformPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "rapier2d")]
        app.add_systems(
            Update,
            MovingPlatform::make_system(
                |velocity: &mut bevy_rapier2d::prelude::Velocity, linvel: TargetVec3| {
                    velocity.linvel = linvel.truncate();
                },
            ),
        );
        #[cfg(feature = "rapier3d")]
        app.add_systems(
            Update,
            MovingPlatform::make_system(
                |velocity: &mut bevy_rapier3d::prelude::Velocity, linvel: TargetVec3| {
                    velocity.linvel = linvel;
                },
            ),
        );
        #[cfg(feature = "xpbd2d")]
        app.add_systems(
            Update,
            MovingPlatform::make_system(
                |velocity: &mut bevy_xpbd_2d::prelude::LinearVelocity, linvel: TargetVec3| {
                    velocity.0 = linvel.truncate();
                },
            ),
        );
        #[cfg(feature = "xpbd3d")]
        app.add_systems(
            Update,
            MovingPlatform::make_system(
                |velocity: &mut bevy_xpbd_3d::prelude::LinearVelocity, linvel: TargetVec3| {
                    velocity.0 = linvel;
                },
            ),
        );
    }
}

#[derive(Component)]
pub struct MovingPlatform {
    pub current_leg: usize,
    pub speed: TargetFloat,
    pub locations: Vec<TargetVec3>,
}

impl MovingPlatform {
    pub fn new(speed: TargetFloat, locations: &[TargetVec3]) -> Self {
        Self {
            current_leg: 0,
            speed,
            locations: locations.to_owned(),
        }
    }

    fn make_system<V: Component>(
        mut updater: impl 'static + Send + Sync + FnMut(&mut V, TargetVec3),
    ) -> bevy::ecs::schedule::SystemConfigs {
        (move |time: Res<Time>,
               mut query: Query<(&mut MovingPlatform, &GlobalTransform, &mut V)>| {
            for (mut moving_platform, transform, mut velocity) in query.iter_mut() {
                let current = transform.translation().adjust_precision();
                let target = moving_platform.locations[moving_platform.current_leg];
                let vec_to = target - current;
                updater(
                    velocity.as_mut(),
                    vec_to.normalize_or_zero() * moving_platform.speed,
                );
                if vec_to.length()
                    <= time.delta_seconds().adjust_precision() * moving_platform.speed
                {
                    moving_platform.current_leg =
                        (moving_platform.current_leg + 1) % moving_platform.locations.len();
                }
            }
        })
        .into_configs()
    }
}
