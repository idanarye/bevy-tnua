use bevy::prelude::*;

use bevy_tnua_physics_integration_layer::data_for_backends::TnuaVelChange;
use bevy_tnua_physics_integration_layer::math::{AdjustPrecision, Float, Quaternion, Vector3};

use crate::TnuaActionContext;

/// Helper trait for implementing basis and actions.
///
/// Methods of this trait typically return a [`TnuaVelChange`] that can be added to
/// [`motor.lin`](crate::TnuaMotor::lin) or [`motor.ang`](crate::TnuaMotor::ang).
pub trait MotionHelper {
    fn frame_duration(&self) -> Float;
    fn up_direction(&self) -> Dir3;
    fn gravity(&self) -> Vector3;
    fn position(&self) -> Vector3;
    fn velocity(&self) -> Vector3;
    fn rotation(&self) -> Quaternion;
    fn angvel(&self) -> Vector3;

    /// A force for cancelling out the gravity.
    fn negate_gravity(&self) -> TnuaVelChange {
        TnuaVelChange::acceleration(-self.gravity())
    }

    /// A force for setting the velocity to the desired velocity, with acceleration constraints.
    ///
    /// The `dimlim` parameter can be used to only set the velocity on certain axis/plane.
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

    /// A force for setting the vertical velocity to the desired velocity, with acceleration constraints.
    fn adjust_vertical_velocity(&self, target: Float, acceleration: Float) -> TnuaVelChange {
        let up_vector = self.up_direction().adjust_precision();
        self.adjust_velocity(up_vector * target, acceleration, |v| {
            v.project_onto_normalized(up_vector)
        })
    }

    /// A force for setting the horizontal velocity to the desired velocity, with acceleration constraints.
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

    /// A force for stopping the character right before reaching a certain point.
    ///
    /// Please note that unlike most [`MotionHelper`] methods that "stack" on impulses and forces
    /// already applied, this one needs to be able to "cancel" the previous changes (on the
    /// relevant axis, at least) which is why it has a `current_vel_change` argument. Due to that,
    /// this must be the **last** velchange applied to the motor (at least, to it's
    /// [`lin`](crate::TnuaMotor::lin) field, on the axis of `direction`)
    ///
    /// `direction` is the direction _from_ the character _toward_ that point. So, for example, in
    /// order to stop a character climbing a ladder when reaching the top of the ladder, `stop_at`
    /// needs to be the coordinates of the top of the ladder while `direction` needs to be
    /// `Dir3::Y` (the UP direction)
    ///
    /// Velocities perpendicular to the `direction` will not be affected.
    fn hard_stop(
        &self,
        direction: Dir3,
        stop_at: Vector3,
        current_vel_change: &TnuaVelChange,
    ) -> TnuaVelChange {
        let expected_velocity =
            self.velocity() + current_vel_change.calc_mean_boost(self.frame_duration());
        let expected_velocity_in_direction = expected_velocity.dot(direction.adjust_precision());
        if expected_velocity_in_direction <= 0.0 {
            return TnuaVelChange::default();
        }
        let distance_in_direction = (stop_at - self.position()).dot(direction.adjust_precision());
        let max_allowed_velocity = distance_in_direction / self.frame_duration();
        let velocity_to_cut = expected_velocity_in_direction - max_allowed_velocity;
        if 0.0 < velocity_to_cut {
            TnuaVelChange::boost(velocity_to_cut * -direction.adjust_precision())
        } else {
            TnuaVelChange::default()
        }
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

    fn position(&self) -> Vector3 {
        self.tracker.translation
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
