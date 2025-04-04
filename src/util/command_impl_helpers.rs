use bevy::prelude::*;

use bevy_tnua_physics_integration_layer::data_for_backends::TnuaVelChange;
use bevy_tnua_physics_integration_layer::math::{AdjustPrecision, Float, Quaternion, Vector3};

use crate::TnuaActionContext;

pub trait MotionHelper {
    fn frame_duration(&self) -> Float;
    fn up_direction(&self) -> Dir3;
    fn gravity(&self) -> Vector3;
    fn velocity(&self) -> Vector3;
    fn rotation(&self) -> Quaternion;
    fn angvel(&self) -> Vector3;

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

    fn adjust_horizontal_velocity(&self, target: Vector3, acceleration: Float) -> TnuaVelChange {
        self.adjust_velocity(target, acceleration, |v| {
            v.reject_from_normalized(self.up_direction().adjust_precision())
        })
    }

    /// Calculate the rotation around `up_direction` required to rotate the character from the
    /// current forward to `desired_forward`.
    fn turn_to_direction(&self, desired_forward: Dir3, up_direction: Dir3) -> TnuaVelChange {
        let up_vector = up_direction.adjust_precision();
        let current_forward = self.rotation().mul_vec3(Vector3::NEG_Z);
        let rotation_along_up_axis = crate::util::rotation_arc_around_axis(
            up_direction,
            current_forward,
            desired_forward.adjust_precision(),
        )
            .unwrap_or(0.0);
        let desired_angvel = rotation_along_up_axis / self.frame_duration();
        let existing_angvel = self.angvel().dot(up_vector);
        let torque_to_turn = desired_angvel - existing_angvel;
        TnuaVelChange::boost(torque_to_turn * up_vector)
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

    fn rotation(&self) -> Quaternion {
        self.tracker.rotation
    }

    fn angvel(&self) -> Vector3 {
        self.tracker.angvel
    }
}
