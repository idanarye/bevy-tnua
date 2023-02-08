#[cfg(feature = "rapier_2d")]
mod backend_rapier2d;
#[cfg(feature = "rapier_3d")]
mod backend_rapier3d;
mod platformer;

#[cfg(feature = "rapier_2d")]
pub use backend_rapier2d::*;
#[cfg(feature = "rapier_3d")]
pub use backend_rapier3d::*;
pub use platformer::*;

mod data_for_backends;
pub use data_for_backends::*;

use bevy::prelude::*;

#[derive(SystemLabel)]
pub enum TnuaSystemLabel {
    Sensors,
    Logic,
    Motors,
}

pub fn tnua_system_set_for_reading_sensor() -> SystemSet {
    SystemSet::new()
        .label(TnuaSystemLabel::Sensors)
        .before(TnuaSystemLabel::Logic)
}

pub fn tnua_system_set_for_computing_logic() -> SystemSet {
    SystemSet::new()
        .label(TnuaSystemLabel::Logic)
        .after(TnuaSystemLabel::Sensors)
        .before(TnuaSystemLabel::Motors)
}

pub fn tnua_system_set_for_applying_motors() -> SystemSet {
    SystemSet::new()
        .label(TnuaSystemLabel::Motors)
        .after(TnuaSystemLabel::Logic)
}
