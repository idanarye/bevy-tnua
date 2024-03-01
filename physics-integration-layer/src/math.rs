#[cfg(feature = "f64")]
pub type TargetFloat = f64;
#[cfg(not(feature = "f64"))]
pub type TargetFloat = f32;

#[cfg(feature = "f64")]
use bevy::math::{DQuat, DVec2, DVec3};
use bevy::math::{Quat, Vec2, Vec3};

#[cfg(feature = "f64")]
pub type TargetVec3 = DVec3;
#[cfg(not(feature = "f64"))]
pub type TargetVec3 = Vec3;

#[cfg(feature = "f64")]
pub type TargetVec2 = DVec2;
#[cfg(not(feature = "f64"))]
pub type TargetVec2 = Vec2;

#[cfg(feature = "f64")]
pub type TargetQuat = DQuat;
#[cfg(not(feature = "f64"))]
pub type TargetQuat = Quat;

// Taken from `bevy_xpbd` https://github.com/Jondolf/bevy_xpbd/blob/main/src/math/double.rs#L39
/// Adjust the precision of the math construct to the precision chosen for compilation.
pub trait AdjustPrecision {
    /// A math construct type with the desired precision.
    type Adjusted;
    /// Adjusts the precision of [`self`] to [`Self::Adjusted`](#associatedtype.Adjusted).
    fn adjust_precision(&self) -> Self::Adjusted;
}

impl AdjustPrecision for f32 {
    type Adjusted = TargetFloat;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return (*self).into();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for f64 {
    type Adjusted = TargetFloat;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return (*self).into();
    }
}

impl AdjustPrecision for Vec3 {
    type Adjusted = TargetVec3;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return self.as_dvec3();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for DVec3 {
    type Adjusted = TargetVec3;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return self.as_vec3();
    }
}

impl AdjustPrecision for Quat {
    type Adjusted = TargetQuat;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return self.as_dquat();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for DQuat {
    type Adjusted = TargetQuat;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return *self;
        #[cfg(not(feature = "f64"))]
        return self.as_quat();
    }
}

impl AdjustPrecision for Vec2 {
    type Adjusted = TargetVec2;
    fn adjust_precision(&self) -> Self::Adjusted {
        #[cfg(feature = "f64")]
        return self.as_dvec2();
        #[cfg(not(feature = "f64"))]
        return *self;
    }
}

#[cfg(feature = "f64")]
impl AdjustPrecision for DVec2 {
    type Adjusted = TargetVec2;
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

#[cfg(feature = "f64")]
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

#[cfg(feature = "f64")]
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
