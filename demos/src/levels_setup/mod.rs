pub mod compound_colliders_2d;
pub mod compound_colliders_3d;
pub mod dynamic_bodies_2d;
pub mod dynamic_bodies_3d;
pub mod for_2d_platformer;
pub mod for_3d_platformer;
mod helper;
pub mod jungle_gym;
pub mod jungle_gym_2d;
pub mod level_switching;
pub mod planet_2d;
pub mod planet_3d;
pub mod pushback_3d;

pub use level_switching::{IsPlayer, LevelObject, PositionPlayer};

use self::level_switching::LevelSwitchingPlugin;

pub fn levels_for_2d(plugin: &mut LevelSwitchingPlugin) {
    plugin.add("Default", for_2d_platformer::setup_level);
    plugin.add("CompoundColliders", compound_colliders_2d::setup_level);
    plugin.add("DynamicBodies", dynamic_bodies_2d::setup_level);
    plugin.add("JungleGym", jungle_gym_2d::setup_level);
    plugin.add("Planet", planet_2d::setup_level);
}

pub fn levels_for_3d(plugin: &mut LevelSwitchingPlugin) {
    plugin.add("Default", for_3d_platformer::setup_level);
    plugin.add("Pushback", pushback_3d::setup_level);
    plugin.add("CompoundColliders", compound_colliders_3d::setup_level);
    plugin.add("DynamicBodies", dynamic_bodies_3d::setup_level);
    plugin.add("JungleGym", jungle_gym::setup_level);
    plugin.add("Planet", planet_3d::setup_level);
}
