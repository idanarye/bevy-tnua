pub mod app_setup_options;
pub mod character_animating_systems;
pub mod character_control_systems;
pub mod hacks;
pub mod level_mechanics;
pub mod levels_setup;
pub mod ui;
pub mod util;

#[macro_export]
macro_rules! verify_physics_backends_features {
    ( $($backend:literal),* ) => {
        {
            let chosen_backends: &[&str] = &[$(
                #[cfg(feature = $backend)]
                $backend
            ),*];
            match chosen_backends.len() {
                0 => {
                    panic!(concat!(
                            "Demo was built with no physics backends. Please build with either:",
                            $("\n * `--features ", $backend, "`"),*
                    ))
                }
                1 => {},
                _ => {
                    panic!("Demo was built with multiple physics backends: {chosen_backends:?}. Please choose only one.");
                }
            }
        }
    }
}
