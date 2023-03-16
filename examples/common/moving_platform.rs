use bevy::prelude::*;

#[derive(Component)]
pub struct MovingPlatform {
    pub current_leg: usize,
    pub speed: f32,
    pub locations: Vec<Vec3>,
}

impl MovingPlatform {
    pub fn new(speed: f32, locations: &[Vec3]) -> Self {
        Self {
            current_leg: 0,
            speed,
            locations: locations.to_owned(),
        }
    }

    pub fn make_system<V: Component>(
        mut updater: impl 'static + Send + Sync + FnMut(&mut V, Vec3),
    ) -> bevy::app::SystemAppConfig {
        (move |time: Res<Time>,
               mut query: Query<(&mut MovingPlatform, &GlobalTransform, &mut V)>| {
            for (mut moving_platform, transform, mut velocity) in query.iter_mut() {
                let current = transform.translation();
                let target = moving_platform.locations[moving_platform.current_leg];
                let vec_to = target - current;
                updater(
                    velocity.as_mut(),
                    vec_to.normalize_or_zero() * moving_platform.speed,
                );
                //info!("At {:?}, going to {:?}", current, target);
                if vec_to.length() <= time.delta_seconds() * moving_platform.speed {
                    moving_platform.current_leg =
                        (moving_platform.current_leg + 1) % moving_platform.locations.len();
                }
            }
        })
        .into_app_config()
    }
}
