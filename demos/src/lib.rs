pub mod app_setup_options;
pub mod character_animating_systems;
pub mod character_control_systems;
pub mod levels_setup;
mod moving_platform;
pub mod ui;
pub mod util;
pub use moving_platform::{MovingPlatform, MovingPlatformPlugin};

#[cfg(all(feature = "avian2d/parry-f32", feature = "f64"))]
compile_error!(
    "Default Feature (f32) and f64 are mutually exclusive and cannot be enabled together"
);
