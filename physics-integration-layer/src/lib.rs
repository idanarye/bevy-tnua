//! # Physics Integration Layer for bevy-tnua
//!
//! Crates that implement a physics layer integration for Tnua (like bevy-tnua-rapier or
//! bevy-tnua-avian) should depend on this crate and not on the main bevy-tnua crate. This crate
//! should update less often - only when there are changes in the integration layer (which is not
//! supposed to change as much) or when Bevy itself updates.
//!
//! To integrate a Bevy physics engine with Tnua, one should create a plugin named
//! `Tnua<physics-engine-name>Plugin`, which:
//!
//! * Configures [`TnuaSystemSet`] to not run when the physics engine is paused.
//! * Add systems, to the [`TnuaPipelineStages::Sensors`] stage, that update:
//!   * [`TnuaRigidBodyTracker`](data_for_backends::TnuaRigidBodyTracker) with the objects current
//!     kinematic status (position, rotation, velocity, angular velocity) as well as the gravity
//!     currently applied to it.
//!   * [`TnuaProximitySensor`](data_for_backends::TnuaProximitySensor) with the _first_ tangible
//!     collider within range, and [`TnuaGhostSensor`](data_for_backends::TnuaGhostSensor) with
//!     _all_ the ghost colliders found before that tangible collider.
//!     * A tangible collider is a **non-ghost** collider that physically interacts with the
//!       character's collider.
//!     * A ghost collider is a collider marked with the
//!       [`TnuaGhostPlatform`](data_for_backends::TnuaGhostPlatform) component. It may or may not
//!       physically interact with the character's collider - as long as it has the component it is
//!       considered a ghost collider.
//!     * The sensor should ignore the owner entity's collider.
//!     * If the sensor has the
//!       [`TnuaSubservientSensor`](subservient_sensors::TnuaSubservientSensor) component, the
//!       "owner entity" is defined as the `owner_entity` field from that component and not the
//!       entity the sensor component is attached to.
//!     * The detection should be done with a ray cast, unless the sensor is configured to cast a
//!       shape instead. Such configuration is done with component, defined by the integration
//!       crate, that specifies the shape to cast in a way the integration crate can pass on to the
//!       physics engine. The name of that component should be
//!       `Tnua<physics-engine-name>SensorShape`.
//!     * The detection should skip entities marked with the
//!       [`TnuaNotPlatform`](data_for_backends::TnuaNotPlatform) component.
//!
//!   The integration crate may update all these components in one system or multiple systems as it
//!   sees fit.
//!
//! * Add a system, to the [`TnuaPipelineStages::Motors`] stage, that applies all the impulses and
//!   accelerations from [`TnuaMotor`](data_for_backends::TnuaMotor) components.
//!
//!   Here, too, if it makes sense to split this work into multiple systems the integration crate
//!   may do so at its own discretion.
//!
//! * Ensure that [`TnuaSystemSet`] runs before the integration backend's systems.
//!
//! * Handle [`TnuaGravity`](data_for_backends::TnuaGravity) in whatever way the integration crate
//!   sees fit. This usually entails:
//!     * Feeding the gravity set in the `TnuaGravity` into the `TnuaRigidBodyTracker` instead of
//!       the regular global gravity.
//!     * Nullifying the regular global gravity. This can be done either by applying an opposing
//!       force or by applying an opposing force or by using some facility offered by the physics
//!       backend (e.g. both Rapier and Avian have a `GravityScale` component which can be used for
//!       that purpose)
//!     * Applying the gravity from `TnuaGravity` as a force.
//!
//!   If the physics backend has its own way to do per-rigid-body gravity, the integration crate
//!   may prefer to sync `TnuaGravity` into that mechanism instead (and should support that
//!   mechanism as the input source for `TnuaRigidBodyTracker::gravity` when it is used, even
//!   if `TnuaGravity` is not used)
//!
//! The integration backend's systems must run with the same timing as the physics backend. If the
//! physics backend supports running in a different schedule, the integration plugin should also
//! support it by adding a `::new()` method that accepts a schedule and registers all the systems
//! there. It should also implement `Default` to make it run in the default schedule (usually
//! `Update`)
//!
//! Note that a physics backend may run its systems under `PostUpdate` instead of `Update` (both
//! Rapier and Avian do this by default). In this case, it's still okay for the integration backend
//! to run under `Update`, because they use the same timing.
//!
//! If the integration crate needs the character entity to have more components from the physics
//! engine crate, that one would not naturally add to it, it should define a bundle named
//! `Tnua<physics-engine-name>IOBundle` that adds these components. One would naturally add a rigid
//! body and a collider, so they should not go in that bundle, but if the crate needs things users
//! rarely think about - for example, bevy_rapier's `ReadMassProperties` - then these components
//! should go in that bundle.
use bevy::prelude::*;

pub mod data_for_backends;
pub mod math;
pub mod obstacle_radar;
pub mod spatial_ext;
pub mod subservient_sensors;

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
    /// Forces are applied in the physics backend.
    Motors,
}
