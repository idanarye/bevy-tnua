//! Generic properties of [basis](TnuaBasis) that [actions](crate::TnuaAction) and control helpers
//! may rely on.
//!
//! All the basis capabilities provided by Tnua itself should be in this module, but third party
//! crates can define their own. Custom capabilities for user code (an actual game that uses Tnua)
//! are usually redundant, since actions and control helpers defined there can usually just use the
//! concrete basis.
//!
//! Capabilities typically use [`TnuaBasisAccess`] to access the basis, since it provides the
//! configuration and memory of the basis rather than just the input.

use std::ops::Range;

use bevy_tnua_physics_integration_layer::data_for_backends::{TnuaProximitySensor, TnuaVelChange};

use crate::TnuaBasis;
use crate::basis_action_traits::TnuaBasisAccess;
use crate::{TnuaBasisContext, math::*};

/// The character controlled by the basis may stand on the surface of an moving object, and needs
/// to move together with said object.
pub trait TnuaBasisWithFrameOfReferenceSurface: TnuaBasis {
    /// The velocity of the character, relative the what the basis considers its frame of
    /// reference.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn effective_velocity(access: &TnuaBasisAccess<Self>) -> Vector3;

    /// The vertical velocity the character requires to stay the same height if it wants to move in
    /// [`effective_velocity`](Self::effective_velocity).
    fn vertical_velocity(access: &TnuaBasisAccess<Self>) -> Float;
}

/// The basis has a specific point the character should be at, which may not be the actual position
/// in Bevy or in the physics engine.
///
/// This typically means the basis is applying forces to get the characeter to that position.
pub trait TnuaBasisWithDisplacement: TnuaBasis {
    /// The displacement of the character from where the basis wants it to be.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn displacement(access: &TnuaBasisAccess<Self>) -> Option<Vector3>;
}

/// The basis keeps track on the entity the chracter stands on - and whether or not it stands on
/// something.
pub trait TnuaBasisWithGround: TnuaBasis {
    /// Can be queried by an action to determine if the character should be considered "in the air".
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn is_airborne(access: &TnuaBasisAccess<Self>) -> bool;

    /// If the basis is at coyote time - finish the coyote time.
    ///
    /// This is typically called by air actions so that a long coyote time will not allow, for
    /// example, unaccounted air jumps. These actions should invoke this method inside their
    /// [`influence_basis`](crate::TnuaAction::influence_basis) method.
    ///
    /// If the character is fully grounded, this method must not change that.
    fn violate_coyote_time(memory: &mut Self::Memory);

    /// The sensor used to detect the ground.
    fn ground_sensor<'a>(sensors: &Self::Sensors<'a>) -> &'a TnuaProximitySensor;
}

/// The basis can keeps track of the space above the character.
///
/// Note that it's possible to opt out of this in the configuration.
pub trait TnuaBasisWithHeadroom: TnuaBasis {
    /// The headroom sensor has detected a ceiling above the character's head.
    ///
    /// This returns `None` when either no ceiling is deteceted in the sensor's range - or when the
    /// headroom sensor is not configured.
    ///
    /// The start of the returned range is the distance from the center of the character's collider
    /// to top of the collider. The end of the range is the distance from the center of the
    /// character's colldier to the detected ceiling.
    fn headroom_intrusion<'a>(
        access: &TnuaBasisAccess<Self>,
        sensors: &Self::Sensors<'a>,
    ) -> Option<Range<Float>>;

    /// Increase the range of the headroom sensor.
    fn set_extra_headroom(memory: &mut Self::Memory, extra_headroom: Float);
}

/// The basis is a floating character controller.
pub trait TnuaBasisWithFloating: TnuaBasis {
    /// The height the basis is configured to float at, measured from the ground to the center of
    /// the character collider.
    fn float_height(access: &TnuaBasisAccess<Self>) -> Float;
}

/// The basis applies a spring force.
pub trait TnuaBasisWithSpring: TnuaBasis {
    /// Calculate the vertical spring force that this basis would need to apply assuming its
    /// vertical distance from the vertical distance it needs to be at equals the `spring_offset`
    /// argument.
    fn spring_force(
        access: &TnuaBasisAccess<Self>,
        ctx: &TnuaBasisContext,
        spring_offset: Float,
    ) -> TnuaVelChange;
}
