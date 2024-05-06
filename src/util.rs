use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::math::{
    AdjustPrecision, Float, Quaternion, Vector2, Vector3,
};

/// Calculate the kinetic energy required to jump to a certain height when different gravity is
/// applied in different segments of the jump.
///
/// **MOTIVATION**: Ballistically accurate jumps where the gravity is constant don't feel good in
/// games. To improve the player experience, Tnua applies higher gravity in different segments of
/// the jump (e.g. to get faster takeoff or to reduce the airtime at the tip of the jump). Being
/// able to control the height of the jump is still vital though, and needs to be done by setting
/// the initial upward velocity of the jump. `SegmentedJumpInitialVelocityCalculator` is a tool for
/// calculating the latter from the former.
///
/// ```
/// # use bevy_tnua::util::SegmentedJumpInitialVelocityCalculator;
/// # use bevy_tnua::math::Float;
/// # let jump_height = 2.0;
/// # const GRAVITY: Float = 9.81;
/// let takeoff_upward_velocity = SegmentedJumpInitialVelocityCalculator::new(jump_height)
///     // When upward velocity is below 1.0, use an extra gravity of 20.0
///     .add_segment(GRAVITY + 20.0, 1.0)
///     // When upward velocity is between 1.0 and 2.0, use regular gravity
///     .add_segment(GRAVITY, 2.0)
///     // When upward velocity is higher than 2.0, use an extra gravity of 30.0
///     .add_final_segment(GRAVITY + 30.0)
///     // After adding all the segments, get the velocity required to make such a jump
///     .required_initial_velocity()
///     .expect("`add_final_segment` should have covered remaining height");
/// ```
///
/// Note that:
///
/// * Only the part of the jump where the character goes up is relevant here. The part after the
///   peak where the character goes down may have its own varying gravity, but since that gravity
///   can not affect the height of the jump `SegmentedJumpInitialVelocityCalculator` does not need
///   to care about it.
/// * Segments are calculated from top to bottom. The very top - the peak of the jump - has, by
///   definition, zero upward velocity, so the `velocity_threshold` passed to it is the one at the
///   bottom. The last segment should have `INFINITY` as its velocity.
/// * The internal representation and calculation is with kinetic energy for a rigid body with a
///   mass of 1.0 rather than with velocities.
pub struct SegmentedJumpInitialVelocityCalculator {
    height: Float,
    kinetic_energy: Float,
}

/// Thrown when attempting to retrieve the result of [`SegmentedJumpInitialVelocityCalculator`]
/// without converting all the height to kinetic energy.
#[derive(thiserror::Error, Debug)]
#[error("Engergy or velocity retrived while not all height was coverted")]
pub struct LeftoverHeight;

impl SegmentedJumpInitialVelocityCalculator {
    /// Create a `SegmentedJumpInitialVelocityCalculator` ready to calculate the velocity required
    /// for a jump of the specified height.
    pub fn new(total_height: Float) -> Self {
        Self {
            height: total_height,
            kinetic_energy: 0.0,
        }
    }

    /// Convert height to kinetic energy for segment under the given gravity.
    ///
    /// The segment is specified by velocity. The bottom determined by the `velocity_threshold`
    /// argument and the top is the bottom of the previous call to `add_segment` - or the peak of
    /// the jump, if this is the first call.
    ///
    /// If there is no height left to convert, nothing will be changed.
    pub fn add_segment(&mut self, gravity: Float, velocity_threshold: Float) -> &mut Self {
        if self.height <= 0.0 {
            // No more height to jump
            return self;
        }

        let kinetic_energy_at_velocity_threshold = 0.5 * velocity_threshold.powi(2);

        let transferred_energy = kinetic_energy_at_velocity_threshold - self.kinetic_energy;
        if transferred_energy <= 0.0 {
            // Already faster than that velocity
            return self;
        }

        let segment_height = transferred_energy / gravity;
        if self.height < segment_height {
            // This segment will be the last
            self.add_final_segment(gravity);
        } else {
            self.kinetic_energy += transferred_energy;
            self.height -= segment_height;
        }

        self
    }

    /// Convert the remaining height to kinetic energy under the given gravity.
    pub fn add_final_segment(&mut self, gravity: Float) -> &mut Self {
        self.kinetic_energy += self.height * gravity;
        self.height = 0.0;
        self
    }

    /// The kinetic energy required to make the jump.
    ///
    /// This should only be called after _all_ the height was converted - otherwise it'll return a
    /// [`LeftoverHeight`] error.
    pub fn kinetic_energy(&self) -> Result<Float, LeftoverHeight> {
        if 0.0 < self.height {
            Err(LeftoverHeight)
        } else {
            Ok(self.kinetic_energy)
        }
    }

    /// Convert kinetic energy to velocity for a rigid body with a mass of 1.0.
    pub fn kinetic_energy_to_velocity(kinetic_energy: Float) -> Float {
        (2.0 * kinetic_energy).sqrt()
    }

    /// The initial upward velocity required to make the jump.
    ///
    /// This should only be called after _all_ the height was converted - otherwise it'll return a
    /// [`LeftoverHeight`] error.
    pub fn required_initial_velocity(&self) -> Result<Float, LeftoverHeight> {
        Ok(Self::kinetic_energy_to_velocity(self.kinetic_energy()?))
    }
}

pub struct ProjectionPlaneForRotation {
    pub forward: Vector3,
    pub sideways: Vector3,
}

impl ProjectionPlaneForRotation {
    pub fn from_up_and_fowrard(up: Direction3d, forward: Vector3) -> Self {
        Self {
            forward,
            sideways: up.adjust_precision().cross(forward),
        }
    }

    pub fn from_up_using_default_forward(up: Direction3d) -> Self {
        Self::from_up_and_fowrard(up, Vector3::NEG_Z)
    }

    pub fn project_and_normalize(&self, vector: Vector3) -> Vector2 {
        Vector2::new(vector.dot(self.forward), vector.dot(self.sideways)).normalize_or_zero()
    }

    pub fn rotation_to_set_forward(
        &self,
        current_forward: Vector3,
        desired_forward: Vector3,
    ) -> Float {
        let rotation_to_set_forward = Quaternion::from_rotation_arc_2d(
            self.project_and_normalize(current_forward),
            self.project_and_normalize(desired_forward),
        );
        rotation_to_set_forward.xyz().z
    }
}
