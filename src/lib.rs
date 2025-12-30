//! # Tnua - A Character Controller for Bevy.
//!
//! Tnua ("motion" in Hebrew) is a floating character controller, which means that instead of
//! constantly touching the ground the character floats above it, which makes many aspects of the
//! motion control simpler.
//!
//! Tnua can use [Rapier](https://rapier.rs/) or [Avian](https://github.com/Jondolf/avian), and
//! supports both the 2D and 3D versions of both with integration crates:
//!
//! * For Rapier 2D, add the [bevy-tnua-rapier2d](https://crates.io/crates/bevy-tnua-rapier2d) crate.
//! * For Rapier 3D, add the [bevy-tnua-rapier3d](https://crates.io/crates/bevy-tnua-rapier3d) crate.
//! * For Avian 2D, add the [bevy-tnua-avian2d](https://crates.io/crates/bevy-tnua-avian2d) crate.
//! * For Avian 3D, add the [bevy-tnua-avian3d](https://crates.io/crates/bevy-tnua-avian3d) crate.
//! * Third party integration crates. Such crates should depend on
//!   [bevy-tnua-physics-integration-layer](https://crates.io/crates/bevy-tnua-physics-integration-layer)
//!   and not the main bevy-tnua crate.
//!
//! Each physics integration crate has basic usage instructions for adding it in its documentation.
//!
//! When using a physics backend with double precision (like Avian with the `f64` flag), the `f64`
//! flag should be added to all the Tnua crates. This applies to double precision data that gets
//! defined by the physics backend - Bevy itself will still use single precision, and this is the
//! precision the position and rotation will use.
//!
//! In addition to the physics integration plugin, the
//! [`TnuaControllerPlugin`](prelude::TnuaControllerPlugin) should also be added.
//!
//! Some physics backends support running in different schedules (e.g. `FixedUpdate` to make the
//! simulation deterministic). When using this feature, the physics integration plugin,
//! `TnuaControllerPlugin`, and any other Tnua plugin that supports it (such as
//! [`TnuaCrouchEnforcer`](crate::control_helpers::TnuaCrouchEnforcer)) must also be registered in
//! that schedule, using their `::new()` method instead of `::default()`. The player controls
//! systems must also be registered under that same schedule (instead of under `Update`, which is
//! where it should usually be registered)
//!
//! A Tnua controlled character must have a dynamic rigid body, everything from
//! `Tnua<physics-backend>IOBundle` (e.g. - for Rapier 3D, use `TnuaRapier3dIOBundle`), and a
//! [`TnuaController`](prelude::TnuaController) (and its automatically added required component):
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
//! cmd.insert(TnuaController::default());
//! ```
//! Typically though it'd also include a `Collider`.
//!
//! ## Optional But Recommended
//!
//! * Tnua, by default, casts a single ray to the ground. This can be a problem when the character
//!   stands on a ledge, because the ray may be past the ledge while the character's collider
//!   isn't. To avoid that, use `Tnua<physics-backend>SensorShape` (e.g. - for Rapier 3D, use
//!   `TnuaRapier3dSensorShape`) to replace the ray with a shape that resembles the collider. It is
//!   better to use a shape a little bit smaller than the collider, so that when the character
//!   presses against a wall Tnua won't think it should be lifted up when the casted shape hits
//!   that wall.
//! * Tnua will apply forces to keep the character upright, but it is also possible to lock
//!   rotation so that there would be no tilting at all. This is done by Tnua itself - it has to be
//!   done by the physics engine. Both Rapier and Avian can do it using a component called
//!   `LockedAxes`. When using it in 3D in combination of rotation controls (such as
//!   [`TnuaBuiltinWalk::desired_forward`](builtins::TnuaBuiltinWalk::desired_forward)) make sure
//!   to only lock the X and Z axess, so that Tnua could rotate the character around the Y axis.
//!
//! ## Controlling the Character
//!
//! To control the character, update the [`TnuaController`](prelude::TnuaController) by feeding it
//! a [basis](TnuaBasis) and zero or more [actions](TnuaAction). For some of the advanced features
//! to work, the system that does this needs to be placed inside the [`TnuaUserControlsSystems`]
//! system set.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_tnua::prelude::*;
//! # use bevy_tnua::math::Vector3;
//! # #[derive(Component)]
//! # struct PlayerInputComponent;
//! # impl PlayerInputComponent {
//! # fn direction_vector(&self) -> Vector3 { Vector3::ZERO }
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
//!             desired_forward: Dir3::new(player_input.direction_vector()).ok(),
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
pub mod action_state;
mod animating_helper;
mod basis_action_traits;
pub mod basis_capabilities;
pub mod builtins;
pub mod control_helpers;
pub mod controller;
pub mod radar_lens;
pub mod util;
pub use animating_helper::{TnuaAnimatingState, TnuaAnimatingStateDirective};
pub use basis_action_traits::{
    TnuaAction, TnuaActionContext, TnuaActionDiscriminant, TnuaBasis, TnuaScheme, TnuaSchemeConfig,
    TnuaUpdateInActionStateResult,
};
pub use basis_action_traits::{
    TnuaActionInitiationDirective, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus,
    TnuaActionState, TnuaBasisContext, TnuaConfigModifier,
};
pub use bevy_tnua_macros::TnuaScheme;
pub use controller::{TnuaController, TnuaControllerPlugin};

pub mod prelude {
    pub use crate::TnuaScheme;
    pub use crate::{
        TnuaController, TnuaControllerPlugin, TnuaPipelineSystems, TnuaUserControlsSystems,
    };
}

pub use bevy_tnua_physics_integration_layer::data_for_backends::*;
pub use bevy_tnua_physics_integration_layer::*;

use bevy::prelude::*;

/// The user controls should be applied in this system set.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct TnuaUserControlsSystems;
