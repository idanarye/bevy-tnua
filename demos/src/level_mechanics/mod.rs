mod moving_platform;

use bevy::prelude::*;

pub use moving_platform::MovingPlatform;

pub struct LevelMechanicsPlugin;

impl Plugin for LevelMechanicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(moving_platform::MovingPlatformPlugin);
    }
}
