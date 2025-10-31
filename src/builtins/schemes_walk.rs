use bevy::prelude::*;

#[allow(unused_imports)]
use crate::math::{float_consts, AdjustPrecision, AsF32, Float, Quaternion, Vector3};
use crate::schemes_traits::Tnua2Basis;

#[derive(Default)]
pub struct Tnua2BuiltinWalk {
    /// The direction (in the world space) and speed to accelerate to.
    ///
    /// Tnua assumes that this vector is orthogonal to the up dierction.
    pub desired_motion: Vector3,

    /// If non-zero, Tnua will rotate the character so that its negative Z will face in that
    /// direction.
    ///
    /// Tnua assumes that this vector is orthogonal to the up direction.
    pub desired_forward: Option<Dir3>,
}

#[derive(Clone)]
pub struct Tnua2BuiltinWalkConfig {
    // How fast the character will go.
    //
    // Note that this will be the speed when [`desired_motion`](Tnua2BuiltinWalk::desired_motion)
    // is a unit vector - meaning that its length is 1.0. If its not 1.0, the speed will be a
    // multiply of that length.
    //
    // Also note that this is the full speed - the character will gradually accelerate to this
    // speed based on the acceleration configuration.
    pub speed: f32,

    /// The height at which the character will float above ground at rest.
    ///
    /// Note that this is the height of the character's center of mass - not the distance from its
    /// collision mesh.
    ///
    /// To make a character crouch, instead of altering this field, prefer to use the
    /// [`TnuaBuiltinCrouch`](crate::builtins::TnuaBuiltinCrouch) action.
    pub float_height: Float,

    /// Extra distance above the `float_height` where the spring is still in effect.
    ///
    /// When the character is at at most this distance above the
    /// [`float_height`](Self::float_height), the spring force will kick in and move it to the
    /// float height - even if that means pushing it down. If the character is above that distance
    /// above the `float_height`, Tnua will consider it to be in the air.
    pub cling_distance: Float,

    /// The force that pushes the character to the float height.
    ///
    /// The actual force applied is in direct linear relationship to the displacement from the
    /// `float_height`.
    pub spring_strength: Float,

    /// A force that slows down the characters vertical spring motion.
    ///
    /// The actual dampening is in direct linear relationship to the vertical velocity it tries to
    /// dampen.
    ///
    /// Note that as this approaches 2.0, the character starts to shake violently and eventually
    /// get launched upward at great speed.
    pub spring_dampening: Float,

    /// The acceleration for horizontal movement.
    ///
    /// Note that this is the acceleration for starting the horizontal motion and for reaching the
    /// top speed. When braking or changing direction the acceleration is greater, up to 2 times
    /// `acceleration` when doing a 180 turn.
    pub acceleration: Float,

    /// The acceleration for horizontal movement while in the air.
    ///
    /// Set to 0.0 to completely disable air movement.
    pub air_acceleration: Float,

    /// The time, in seconds, the character can still jump after losing their footing.
    pub coyote_time: Float,

    /// Extra gravity for free fall (fall that's not initiated by a jump or some other action that
    /// provides its own fall gravity)
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    ///
    /// **NOTE**: If the parameter set to this option is too low, the character may be able to run
    /// up a slope and "jump" potentially even higher than a regular jump, even without pressing
    /// the jump button.
    pub free_fall_extra_gravity: Float,

    /// The maximum angular velocity used for keeping the character standing upright.
    ///
    /// NOTE: The character's rotation can also be locked to prevent it from being tilted, in which
    /// case this paramter is redundant and can be set to 0.0.
    pub tilt_offset_angvel: Float,

    /// The maximum angular acceleration used for reaching `tilt_offset_angvel`.
    ///
    /// NOTE: The character's rotation can also be locked to prevent it from being tilted, in which
    /// case this paramter is redundant and can be set to 0.0.
    pub tilt_offset_angacl: Float,

    /// The maximum angular velocity used for turning the character when the direction changes.
    pub turning_angvel: Float,

    /// The maximum slope, in radians, that the character can stand on without slipping.
    pub max_slope: Float,
}

impl Default for Tnua2BuiltinWalkConfig {
    fn default() -> Self {
        Self {
            speed: 10.0,
            float_height: 0.0,
            cling_distance: 1.0,
            spring_strength: 400.0,
            spring_dampening: 1.2,
            acceleration: 60.0,
            air_acceleration: 20.0,
            coyote_time: 0.15,
            free_fall_extra_gravity: 60.0,
            tilt_offset_angvel: 5.0,
            tilt_offset_angacl: 500.0,
            turning_angvel: 10.0,
            max_slope: float_consts::FRAC_PI_2,
        }
    }
}

pub struct Tnua2BuiltinWalkMemory {}

impl Tnua2Basis for Tnua2BuiltinWalk {
    type Config = Tnua2BuiltinWalkConfig;

    type Memory = Tnua2BuiltinWalkMemory;
}
