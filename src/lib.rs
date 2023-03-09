mod animating_helper;
#[cfg(feature = "rapier_2d")]
mod backend_rapier2d;
#[cfg(feature = "rapier_3d")]
mod backend_rapier3d;
mod platformer;
pub use animating_helper::{TnuaAnimatingState, TnuaAnimatingStateDirective};

#[cfg(feature = "rapier_2d")]
pub use backend_rapier2d::*;
#[cfg(feature = "rapier_3d")]
pub use backend_rapier3d::*;
pub use platformer::*;

mod data_for_backends;
pub use data_for_backends::*;

use bevy::prelude::*;

#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum TnuaSystemSet {
    Sensors,
    Logic,
    Motors,
}
