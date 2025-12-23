use crate::math::*;
use crate::schemes_traits::{Tnua2Basis, Tnua2BasisAccess};

pub trait TnuaBasisWithEffectiveVelocity: Tnua2Basis {
    /// The velocity of the character, relative the what the basis considers its frame of
    /// reference.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn effective_velocity(access: &Tnua2BasisAccess<Self>) -> Vector3;

    /// The vertical velocity the character requires to stay the same height if it wants to move in
    /// [`effective_velocity`](Self::effective_velocity).
    fn vertical_velocity(access: &Tnua2BasisAccess<Self>) -> Float;
}

pub trait TnuaBasisWithDisplacement: Tnua2Basis {
    /// The displacement of the character from where the basis wants it to be.
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn displacement(access: &Tnua2BasisAccess<Self>) -> Option<Vector3>;
}

pub trait TnuaBasisWithGround: Tnua2Basis {
    /// Can be queried by an action to determine if the character should be considered "in the air".
    ///
    /// This is a query method, used by the action to determine what the basis thinks.
    fn is_airborne(access: &Tnua2BasisAccess<Self>) -> bool;

    /// If the basis is at coyote time - finish the coyote time.
    ///
    /// This will be called automatically by Tnua, if the controller runs an action that  [violated
    /// coyote time](TnuaAction::VIOLATES_COYOTE_TIME), so that a long coyote time will not allow,
    /// for example, unaccounted air jumps.
    ///
    /// If the character is fully grounded, this method must not change that.
    fn violate_coyote_time(memory: &mut Self::Memory);
}
