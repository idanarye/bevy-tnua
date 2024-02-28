#[cfg(feature = "f64")]
pub type TargetFloat = f64;
#[cfg(not(feature = "f64"))]
pub type TargetFloat = f32;

#[cfg(feature = "f64")]
use bevy::math::DVec3;
use bevy::math::Vec3;

#[cfg(feature = "f64")]
pub type TargetVec3 = DVec3;
#[cfg(not(feature = "f64"))]
pub type TargetVec3 = Vec3;

#[cfg(feature = "f64")]
use bevy::math::DVec2;
use bevy::math::Vec2;

#[cfg(feature = "f64")]
pub type TargetVec2 = DVec2;
#[cfg(not(feature = "f64"))]
pub type TargetVec2 = Vec2;

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
