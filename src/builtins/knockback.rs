#![allow(unused_imports)]
use crate::math::{AdjustPrecision, Float, Vector3};
use bevy::prelude::*;

#[derive(Clone)]
pub struct TnuaBuiltinKnockback {
    /// Timeout (in seconds) for abandoning a Pushover boundary that no longer gets pushed.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub no_push_timeout: f32,

    /// An exponent for controlling the shape of the Pushover barrier diminishing.
    ///
    /// For best results, set it to values larger than 1.0.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub barrier_strength_diminishing: Float,

    /// Acceleration cap when pushing against the Pushover barrier.
    ///
    /// In practice this will be averaged with [`acceleration`](Self::acceleration) (weighted by a
    /// function of the pushover boundary penetration percentage and
    /// [`barrier_strength_diminishing`](Self::barrier_strength_diminishing)) so
    /// the actual acceleration limit will higher than that.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub acceleration_limit: Float,

    /// Acceleration cap when pushing against the Pushover barrier while in the air.
    ///
    /// In practice this will be averaged with [`air_acceleration`](Self::air_acceleration)
    /// (weighted by a function of the pushover boundary penetration percentage and
    /// [`barrier_strength_diminishing`](Self::barrier_strength_diminishing)) so
    /// the actual acceleration limit will higher than that.
    ///
    /// Refer to [`VelocityBoundaryTracker`] for more information about the Pushover feature.
    pub air_acceleration_limit: Float,
}

pub struct TnuaBuiltinKnockbackState {}
