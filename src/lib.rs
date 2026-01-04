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
//! In addition to the physics integration plugin, the [`TnuaControllerPlugin`] should also be
//! added.
//!
//! Some physics backends support running in different schedules (e.g. `FixedUpdate` to make the
//! simulation deterministic). When using this feature, the physics integration plugin,
//! `TnuaControllerPlugin`, and any other Tnua plugin that supports it  must also be registered in
//! that schedule, using their `::new()` method instead of `::default()`.
//!
//! ## Defining the control scheme
//!
//! The range of movement actions available to the character controller is defined by the _control
//! scheme_. The controle scheme is an enum that derives [`TnuaScheme`]. It needs to use an
//! attribute to define the _basis_ - a constant mode of movement that character is always going to
//! be in. Simple games prboably want to use [`TnuaBuiltinWalk`](crate::builtins::TnuaBuiltinWalk),
//! which defines a simple floating character that can be moved around with a simple vector and can
//! be told to face a direcetion.
//!
//! The enum's variants define the actions - various movement commands on top of the basis, like
//! jumping, crouching, climbing, etc. The variants need to be tuple variants, with the action's
//! type as the first tuple member of each variants.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use bevy_tnua::prelude::*;
//! #[derive(TnuaScheme)]
//! #[scheme(basis = TnuaBuiltinWalk)]
//! enum ControlScheme {
//!     Jump(TnuaBuiltinJump),
//!     // more actions can be defined as more variants
//! }
//! ```
//!
//! For more avaialbe attributes, see [the `TnuaScheme` derive macro](bevy_tnua_macros::TnuaScheme)
//! documentation
//!
//! ## Spawning the character controller
//!
//! A Tnua controlled character must have a dynamic rigid body, a [`TnuaConfig`]. The controller is
//! parameterized by the control scheme and needs a configuration (based on the control scheme) as
//! an asset handle:
//! ```no_run
//! # use bevy::prelude::*;
//! # // Not importing from Rapier because there are two versions and the default features does not
//! # // enable either:
//! # #[derive(Component)]
//! # enum RigidBody { Dynamic }
//! # use bevy_tnua::prelude::*;
//! # use bevy_tnua::builtins::{TnuaBuiltinWalkConfig, TnuaBuiltinJumpConfig};
//! # let mut commands: Commands = panic!();
//! # let mut cmd = commands.spawn_empty();
//! # #[derive(TnuaScheme)] #[scheme(basis = TnuaBuiltinWalk)] enum ControlScheme {Jump(TnuaBuiltinJump)}
//! # let control_scheme_configs: Assets<ControlSchemeConfig> = panic!();
//! cmd.insert(RigidBody::Dynamic);
//! cmd.insert(TnuaController::<ControlScheme>::default());
//! cmd.insert(TnuaConfig::<ControlScheme>(
//!     // This example creates the configuration by code and injects it to the Assets resource,
//!     // but a proper game will probably want to load it from an asset file.
//!     control_scheme_configs.add(ControlSchemeConfig {
//!         // The basis' configuration is alwayts named `basis`:
//!         basis: TnuaBuiltinWalkConfig {
//!             // Must be larger than the height of the entity's center from the bottom of its
//!             // collider, or else the character will not float and Tnua will not work properly:
//!             float_height: 2.0,
//!
//!             // TnuaBuiltinWalkConfig has many other fields that can be configured:
//!             ..Default::default()
//!         },
//!         // Actions' configurations are named after the variants defining the actions:
//!         jump: TnuaBuiltinJumpConfig {
//!             // The full height of the jump, if the player does not release the button:
//!             height: 4.0,
//!
//!             // TnuaBuiltinJumpConfig too has other fields that can be configured:
//!             ..Default::default()
//!         },
//!     })
//! ));
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
//! To control the character, update the [`TnuaController`] by feeding it a [basis](TnuaBasis) and
//! zero or more [actions](TnuaAction). For some of the advanced features to work, the system that
//! does this needs to be placed inside the [`TnuaUserControlsSystems`] system set.
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
//! # #[derive(TnuaScheme)] #[scheme(basis = TnuaBuiltinWalk)] enum ControlScheme {Jump(TnuaBuiltinJump)}
//! fn player_control_system(mut query: Query<(
//!     &mut TnuaController<ControlScheme>,
//!     &PlayerInputComponent,  // not part of Tnua - defined in user code
//! )>) {
//!     for (mut controller, player_input) in query.iter_mut() {
//!         controller.basis = TnuaBuiltinWalk {
//!             // Move in the direction the player entered:
//!             desired_motion: player_input.direction_vector(),
//!
//!             // Turn the character in the movement direction:
//!             desired_forward: Dir3::new(player_input.direction_vector()).ok(),
//!         };
//!
//!         if player_input.jump_pressed() {
//!             // The jump action must be fed as long as the player holds the button.
//!             controller.action(ControlScheme::Jump(Default::default()));
//!         }
//!     }
//! }
//! ```
//! Refer to the documentation of [`TnuaController`] for more information, but essentially the
//! _basis_ controls the general movement and the _action_ is something special (jump, dash,
//! crouch, etc.)
//!
//! ## Motion Based Animation
//!
//! [`TnuaController`] can also be used to retreive data that can be used to decide which animation
//! to play. A useful helper for that is [`TnuaAnimatingState`].
pub mod action_state;
mod animating_helper;
mod basis_action_traits;
#[doc(hidden)]
pub use serde;
pub mod basis_capabilities;
pub mod builtins;
pub mod control_helpers;
pub mod controller;
pub mod ghost_overrides;
pub mod radar_lens;
pub mod sensor_sets;
pub mod util;
pub use animating_helper::{TnuaAnimatingState, TnuaAnimatingStateDirective};
pub use basis_action_traits::{
    TnuaAction, TnuaActionContext, TnuaActionDiscriminant, TnuaBasis, TnuaBasisAccess, TnuaScheme,
    TnuaSchemeConfig, TnuaUpdateInActionStateResult,
};
pub use basis_action_traits::{
    TnuaActionInitiationDirective, TnuaActionLifecycleDirective, TnuaActionLifecycleStatus,
    TnuaActionState, TnuaBasisContext, TnuaConfigModifier,
};
pub use bevy_tnua_macros::TnuaScheme;
pub use controller::{TnuaConfig, TnuaController, TnuaControllerPlugin};
pub use ghost_overrides::TnuaGhostOverwrites;
pub use sensor_sets::TnuaSensorsEntities;

pub mod prelude {
    pub use crate::TnuaScheme;
    pub use crate::builtins::{TnuaBuiltinJump, TnuaBuiltinWalk};
    pub use crate::{
        TnuaConfig, TnuaController, TnuaControllerPlugin, TnuaPipelineSystems,
        TnuaUserControlsSystems,
    };
}

pub use bevy_tnua_physics_integration_layer::data_for_backends::*;
pub use bevy_tnua_physics_integration_layer::*;

use bevy::prelude::*;

/// The user controls should be applied in this system set.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct TnuaUserControlsSystems;
