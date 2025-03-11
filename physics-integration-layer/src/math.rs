#[cfg(feature = "f64")]
pub type Float = f64;
#[cfg(not(feature = "f64"))]
pub type Float = f32;

#[cfg(not(feature = "f64"))]
pub use std::f32::consts as float_consts;
#[cfg(feature = "f64")]
pub use std::f64::consts as float_consts;

use bevy::math::{DQuat, DVec2, DVec3};
use bevy::math::{Quat, Vec2, Vec3};

#[cfg(feature = "f64")]
pub type Vector3 = DVec3;
#[cfg(not(feature = "f64"))]
pub type Vector3 = Vec3;

#[cfg(feature = "f64")]
pub type Vector2 = DVec2;
#[cfg(not(feature = "f64"))]
pub type Vector2 = Vec2;

#[cfg(feature = "f64")]
pub type Quaternion = DQuat;
#[cfg(not(feature = "f64"))]
pub type Quaternion = Quat;

// Taken from `avian` https://github.com/Jondolf/avian/blob/main/src/math/double.rs#L39
/// Adjust the precision of the math construct to the precision chosen for compilation.
pub trait AdjustPrecision {
    /// A math construct type with the desired precision.
    type Adjusted;
    /// Adjusts the precision of [`self`] to [`Self::Adjusted`](#associatedtype.Adjusted).
    fn adjust_precision(&self) -> Self::Adjusted;
}

impl AdjustPrecision for f32 {
    type Adjusted = Float;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return (*self).into();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for f64 {
    type Adjusted = Float;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return (*self).into();
    }
}

impl AdjustPrecision for Vec3 {
    type Adjusted = Vector3;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return self.as_dvec3();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for DVec3 {
    type Adjusted = Vector3;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return self.as_vec3();
    }
}

impl AdjustPrecision for Quat {
    type Adjusted = Quaternion;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return self.as_dquat();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for DQuat {
    type Adjusted = Quaternion;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return self.as_quat();
    }
}

impl AdjustPrecision for Vec2 {
    type Adjusted = Vector2;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return self.as_dvec2();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for DVec2 {
    type Adjusted = Vector2;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return self.as_vec2();
    }
}

/// Adjust the precision down to `f32` regardless of compilation.
pub trait AsF32 {
    /// The `f32` version of a math construct.
    type F32;
    /// Returns the `f32` version of this type.
    fn f32(&self) -> Self::F32;
}

impl AsF32 for f32 {
    type F32 = f32;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for f64 {
    type F32 = f32;
    fn f32(&self) -> Self::F32 {
        *self as f32
    }
}

impl AsF32 for DVec3 {
    type F32 = Vec3;
    fn f32(&self) -> Self::F32 {
        self.as_vec3()
    }
}

impl AsF32 for Vec3 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for DVec2 {
    type F32 = Vec2;
    fn f32(&self) -> Self::F32 {
        self.as_vec2()
    }
}

impl AsF32 for Vec2 {
    type F32 = Self;
    fn f32(&self) -> Self::F32 {
        *self
    }
}

impl AsF32 for DQuat {
    type F32 = Quat;
    fn f32(&self) -> Self::F32 {
        self.as_quat()
    }
}

impl AsF32 for Quat {
    type F32 = Quat;
    fn f32(&self) -> Self::F32 {
        *self
    }
}
