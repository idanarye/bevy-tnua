use bevy::prelude::*;

use bevy_tnua_physics_integration_layer::data_for_backends::TnuaVelChange;
use bevy_tnua_physics_integration_layer::math::{AdjustPrecision, Float, Vector3};

use crate::TnuaActionContext;

pub trait MotionHelper {
    fn frame_duration(&self) -> Float;
    fn up_direction(&self) -> Dir3;
    fn gravity(&self) -> Vector3;
    fn velocity(&self) -> Vector3;

    fn negate_gravity(&self) -> TnuaVelChange {
        TnuaVelChange::acceleration(-self.gravity())
    }

    fn adjust_velocity(
        &self,
        target: Vector3,
        acceleration: Float,
        dimlim: impl Fn(Vector3) -> Vector3,
    ) -> TnuaVelChange {
        let delta = dimlim(target - self.velocity());
        let allowed_this_frame = acceleration * self.frame_duration();
        if delta.length_squared() <= allowed_this_frame.powi(2) {
            TnuaVelChange::boost(delta)
        } else {
            TnuaVelChange::acceleration(delta.normalize_or_zero() * acceleration)
        }
    }

    fn adjust_vertical_velocity(&self, target: Float, acceleration: Float) -> TnuaVelChange {
        let up_vector = self.up_direction().adjust_precision();
        self.adjust_velocity(up_vector * target, acceleration, |v| {
            v.project_onto_normalized(up_vector)
        })
    }
}

impl MotionHelper for TnuaActionContext<'_> {
    fn frame_duration(&self) -> Float {
        self.frame_duration
    }

    fn up_direction(&self) -> Dir3 {
        self.up_direction
    }

    fn gravity(&self) -> Vector3 {
        self.tracker.gravity
    }

    fn velocity(&self) -> Vector3 {
        self.tracker.velocity
    }
}
