//! # Tnua - A Character Controller for bevy_rapier.
//!
//! Tnua ("motion" in Hebrew) is a floating character controller, which means that instead of
//! constantly touching the ground the character floats above it, which makes many aspects of the
//! motion control simpler.
//!
//! Tnua uses [Rapier](https://rapier.rs/), and supports both the 2D and 3D versions of it:
//!
//! * For 2D, enable `features = ["rapier_2d"]` and use [`TnuaRapier2dPlugin`].
//! * For 3D, enable `features = ["rapier_3d"]` and use [`TnuaRapier3dPlugin`].
//!
//! In addition to the physics backend plugin, the
//! [`TnuaControllerPlugin`](prelude::TnuaControllerPlugin) should also be added.
//!
//! A Tnua controlled character must have a dynamic rigid body, everything from
//! [`TnuaRapier2dIOBundle`]/[`TnuaRapier3dIOBundle`] (depending on the physics backend), and
//! everything from [`TnuaControllerBundle`](prelude::TnuaControllerBundle):
//! ```no_run
//! # use bevy::prelude::*;
//! # // Not importing from Rapier because there are two versions and the default features does not
//! # // enable either:
//! # type TnuaRapier3dIOBundle = ();
//! # #[derive(Component)]
//! # enum RigidBody { Dynamic }
//! # use bevy_tnua::prelude::*;
//! # let mut commands: Commands = panic!();
//! # let mut cmd = commands.spawn_empty();
//! cmd.insert(RigidBody::Dynamic);
//! cmd.insert(TnuaRapier3dIOBundle::default()); // this one depends on the physics backend
//! cmd.insert(TnuaControllerBundle::default());
//! ```
//! Typically though it'd also include a `Collider`.
//!
//! ## Optional But Recommended
//!
//! * Tnua, by default, casts a single ray to the ground. This can be a problem when the character
//!   stands on a ledge, because the ray may be past the ledge while the character's collider
//!   isn't. To avoid that, use [`TnuaRapier2dSensorShape`] or [`TnuaRapier3dSensorShape`]
//!   (depending on the physics backend) to replace the ray with a shape that resembles the
//!   collider. It is better to use a shape a little bit smaller than the collider, so that when
//!   the character presses against a wall Tnua won't think it should be lifted up when the casted
//!   shape hits that wall.
//! * Tnua will apply forces to keep the character upright, but `LockedAxes` can also be used to
//!   prevent tilting entirely (without it the tilting will be visible)
//!
//! ## Controlling the Character
//!
//! To control the character, update the [`TnuaController`](prelude::TnuaController) (added via tha
//! [`TnuaControllerBundle`](prelude::TnuaControllerBundle)) in a system. For some of the advanced
//! features to work, this system needs to be placed inside the [`TnuaUserControlsSystemSet`]
//! system set.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_tnua::prelude::*;
//! # #[derive(Component)]
//! # struct PlayerInputComponent;
//! # impl PlayerInputComponent {
//! # fn direction_vector(&self) -> Vec3 { Vec3::ZERO }
//! # fn jump_pressed(&self) -> bool { false }
//! # }
//! fn player_control_system(mut query: Query<(
//!     &mut TnuaController,
//!     &PlayerInputComponent,  // not part of Tnua - defined in user code
//! )>) {
//!     for (mut controller, player_input) in query.iter_mut() {
//!         controller.basis(TnuaBuiltinWalk {
//!             // Move in the direction the player entered, at a speed of 10.0:
//!             desired_velocity: player_input.direction_vector() * 10.0,
//!
//!             // Turn the character in the movement direction:
//!             desired_forward: player_input.direction_vector(),
//!             
//!             // Must be larger than the height of the entity's center from the bottom of its
//!             // collider, or else the character will not float and Tnua will not work properly:
//!             float_height: 2.0,
//!
//!             // TnuaBuiltinWalk has many other fields that can be configured:
//!             ..Default::default()
//!         });
//!
//!         if player_input.jump_pressed() {
//!             // The jump action must be fed as long as the player holds the button.
//!             controller.action(TnuaBuiltinJump {
//!                 // The full height of the jump, if the player does not release the button:
//!                 height: 4.0,
//!
//!                 // TnuaBuiltinJump too has other fields that can be configured:
//!                 ..Default::default()
//!             });
//!         }
//!     }
//! }
//! ```
//! Refer to the documentation of [`TnuaController`](prelude::TnuaController) for more information,
//! but essentially the _basis_ controls the general movement and the _action_ is something
//! special (jump, dash, crouch, etc.)
//!
//! ## Motion Based Animation
//!
//! [`TnuaController`](crate::prelude::TnuaController) can also be used to retreive data that can
//! be used to decide which animation to play. A useful helper for that is [`TnuaAnimatingState`].
mod animating_helper;
#[cfg(feature = "rapier_2d")]
mod backend_rapier2d;
#[cfg(feature = "rapier_3d")]
mod backend_rapier3d;
mod basis_action_traits;
pub mod builtins;
pub mod control_helpers;
pub mod controller;
mod platformer;
mod subservient_sensors;
mod util;
pub use animating_helper::{TnuaAnimatingState, TnuaAnimatingStateDirective};
pub use basis_action_traits::{TnuaAction, TnuaBasis};

#[cfg(feature = "rapier_2d")]
pub use backend_rapier2d::*;
#[cfg(feature = "rapier_3d")]
pub use backend_rapier3d::*;
pub use platformer::*;

pub mod prelude {
    pub use crate::builtins::{TnuaBuiltinJump, TnuaBuiltinWalk};
    pub use crate::controller::{TnuaController, TnuaControllerBundle, TnuaControllerPlugin};
    pub use crate::{TnuaAction, TnuaPipelineStages, TnuaUserControlsSystemSet};
    #[cfg(feature = "rapier_2d")]
    pub use crate::{TnuaRapier2dIOBundle, TnuaRapier2dPlugin, TnuaRapier2dSensorShape};
    #[cfg(feature = "rapier_3d")]
    pub use crate::{TnuaRapier3dIOBundle, TnuaRapier3dPlugin, TnuaRapier3dSensorShape};
}

mod data_for_backends;
pub use data_for_backends::*;

use bevy::prelude::*;

/// Umbrella system set for [`TnuaPipelineStages`].
///
/// The physics backends' plugins are responsible for preventing this entire system set from
/// running when the physics backend itself is paused.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct TnuaSystemSet;

/// The various stages of the Tnua pipeline.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub enum TnuaPipelineStages {
    /// Data is read from the physics backend.
    Sensors,
    /// Data is propagated through the subservient sensors.
    SubservientSensors,
    /// Tnua decieds how the entity should be manipulated.
    Logic,
    /// Forces are applied in the physiscs backend.
    Motors,
}

/// The user controls should be applied in this system set.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct TnuaUserControlsSystemSet;
