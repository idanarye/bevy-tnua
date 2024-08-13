mod cannon;
mod moving_platform;
mod push_effect;
mod time_to_despawn;

use bevy::prelude::*;

pub use cannon::{Cannon, CannonBullet};
pub use moving_platform::MovingPlatform;
pub use time_to_despawn::TimeToDespawn;

pub struct LevelMechanicsPlugin;

impl Plugin for LevelMechanicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(moving_platform::MovingPlatformPlugin);
        app.add_plugins(cannon::CannonPlugin);
        app.add_plugins(push_effect::PushEffectPlugin);
        app.add_plugins(time_to_despawn::TimeToDespawnPlugin);
    }
}
