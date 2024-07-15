pub mod app_setup_options;
pub mod character_animating_systems;
pub mod character_control_systems;
pub mod level_mechanics;
pub mod levels_setup;
pub mod ui;
pub mod util;

#[cfg(all(feature = "avian2d/parry-f32", feature = "f64"))]
compile_error!(
    "Default Feature (f32) and f64 are mutually exclusive and cannot be enabled together"
);
