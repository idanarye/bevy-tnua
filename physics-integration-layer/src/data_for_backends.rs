use std::ops::{Add, AddAssign};

use crate::math::{AdjustPrecision, Float, Quaternion, Vector3};
use bevy::prelude::*;

pub use crate::obstacle_radar::TnuaObstacleRadar;

/// Allows disabling Tnua for a specific entity.
///
/// This can be used to let some other system  temporarily take control over a character.
///
/// This component is not mandatory - if omitted, Tnua will just assume it is enabled for that
/// entity.
#[derive(Component, Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum TnuaToggle {
    /// Do not update the sensors, and do not apply forces from the motor.
    ///
    /// The controller system will also not run and won't update the motor components not the state
    /// stored in the `TnuaController` component. They will retain their last value from before
    /// `TnuaToggle::Disabled` was set.
    Disabled,
    /// Update the sensors, but do not apply forces from the motor.
    ///
    /// The platformer controller system will still run and still update the motor components and
    /// state stored in the `TnuaController` component. only the system that applies the motor
    /// forces will be disabled.
    SenseOnly,
    #[default]
    /// The backend behaves normally - it updates the sensors and applies forces from the motor.
    Enabled,
}

/// Newtonian state of the rigid body.
///
/// Tnua takes the position and rotation of the rigid body from its `GlobalTransform`, but things
/// like velocity are dependent on the physics engine. The physics backend is responsible for
/// updating this component from the physics engine during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors).
#[derive(Component, Debug)]
pub struct TnuaRigidBodyTracker {
    pub translation: Vector3,
    pub rotation: Quaternion,
    pub velocity: Vector3,
    /// Angular velocity as the rotation axis multiplied by the rotation speed in radians per
    /// second. Can be extracted from a quaternion using [`Quaternion::xyz`].
    pub angvel: Vector3,
    pub gravity: Vector3,
}

impl Default for TnuaRigidBodyTracker {
    fn default() -> Self {
        Self {
            translation: Vector3::ZERO,
            rotation: Quaternion::IDENTITY,
            velocity: Vector3::ZERO,
            angvel: Vector3::ZERO,
            gravity: Vector3::ZERO,
        }
    }
}

/// Distance from another collider in a certain direction, and information on that collider.
///
/// The physics backend is responsible for updating this component from the physics engine during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors), usually by casting a ray
/// or a shape in the `cast_direction`.
#[derive(Component, Debug)]
pub struct TnuaProximitySensor {
    /// The cast origin in the entity's coord system.
    pub cast_origin: Vector3,
    /// The direction in world coord system (unmodified by the entity's transform)
    pub cast_direction: Dir3,
    /// Tnua will update this field according to its need. The backend only needs to read it.
    pub cast_range: Float,
    pub output: Option<TnuaProximitySensorOutput>,
}

impl Default for TnuaProximitySensor {
    fn default() -> Self {
        Self {
            cast_origin: Vector3::ZERO,
            cast_direction: Dir3::NEG_Y,
            cast_range: 0.0,
            output: None,
        }
    }
}

/// Information from [`TnuaProximitySensor`] that have detected another collider.
#[derive(Debug, Clone)]
pub struct TnuaProximitySensorOutput {
    /// The entity of the collider detected by the ray.
    pub entity: Entity,
    /// The distance to the collider from [`cast_origin`](TnuaProximitySensor::cast_origin) along the
    /// [`cast_direction`](TnuaProximitySensor::cast_direction).
    pub proximity: Float,
    /// The normal from the detected collider's surface where the ray hits.
    pub normal: Dir3,
    /// The velocity of the detected entity,
    pub entity_linvel: Vector3,
    /// The angular velocity of the detected entity, given as the rotation axis multiplied by the
    /// rotation speed in radians per second. Can be extracted from a quaternion using
    /// [`Quaternion::xyz`].
    pub entity_angvel: Vector3,
}

/// Represents a change to velocity (linear or angular)
#[derive(Debug, Clone)]
pub struct TnuaVelChange {
    // The part of the velocity change that gets multiplied by the frame duration.
    //
    // In Rapier, this is applied using `ExternalForce` so that the simulation will apply in
    // smoothly over time and won't be sensitive to frame rate.
    pub acceleration: Vector3,
    // The part of the velocity change that gets added to the velocity as-is.
    //
    // In Rapier, this is added directly to the `Velocity` component.
    pub boost: Vector3,
}

impl TnuaVelChange {
    pub const ZERO: Self = Self {
        acceleration: Vector3::ZERO,
        boost: Vector3::ZERO,
    };

    pub fn acceleration(acceleration: Vector3) -> Self {
        Self {
            acceleration,
            boost: Vector3::ZERO,
        }
    }

    pub fn boost(boost: Vector3) -> Self {
        Self {
            acceleration: Vector3::ZERO,
            boost,
        }
    }

    pub fn clear(&mut self) {
        self.acceleration = Vector3::ZERO;
        self.boost = Vector3::ZERO;
    }

    pub fn project_onto(&self, rhs: Vector3) -> Self {
        Self {
            acceleration: self.acceleration.project_onto(rhs),
            boost: self.boost.project_onto(rhs),
        }
    }

    pub fn project_onto_normalized(&self, rhs: Vector3) -> Self {
        Self {
            acceleration: self.acceleration.project_onto_normalized(rhs),
            boost: self.boost.project_onto_normalized(rhs),
        }
    }

    pub fn project_onto_dir(&self, rhs: Dir3) -> Self {
        self.project_onto_normalized(rhs.adjust_precision())
    }

    pub fn cancel_on_axis(&mut self, axis: Vector3) {
        self.acceleration = self.acceleration.reject_from(axis);
        self.boost = self.boost.reject_from(axis);
    }

    pub fn calc_boost(&self, frame_duration: Float) -> Vector3 {
        self.acceleration * frame_duration + self.boost
    }

    pub fn calc_mean_boost(&self, frame_duration: Float) -> Vector3 {
        0.5 * self.acceleration * frame_duration + self.boost
    }

    pub fn calc_acceleration(&self, frame_duration: Float) -> Vector3 {
        self.acceleration + self.boost / frame_duration
    }

    pub fn apply_boost_limit(
        &mut self,
        frame_duration: Float,
        component_direction: Dir3,
        component_limit: Float,
    ) {
        let regular_boost = self.calc_boost(frame_duration);
        let regular = regular_boost.dot(component_direction.adjust_precision());
        let to_cut = regular - component_limit;
        if to_cut <= 0.0 {
            return;
        }
        let boost_part = self.boost.dot(component_direction.adjust_precision());
        if to_cut <= boost_part {
            // Can do the entire cut by just reducing the boost
            self.boost -= to_cut * component_direction.adjust_precision();
            return;
        }
        // Even nullifying the boost is not enough, and we don't want to
        // reverse it, so we're going to cut the acceleration as well.
        self.boost = self
            .boost
            .reject_from(component_direction.adjust_precision());
        let to_cut_from_acceleration = to_cut - boost_part;
        let acceleration_to_cut = to_cut_from_acceleration / frame_duration;
        self.acceleration -= acceleration_to_cut * component_direction.adjust_precision();
    }
}

impl Default for TnuaVelChange {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add<TnuaVelChange> for TnuaVelChange {
    type Output = TnuaVelChange;

    fn add(self, rhs: TnuaVelChange) -> Self::Output {
        Self::Output {
            acceleration: self.acceleration + rhs.acceleration,
            boost: self.boost + rhs.boost,
        }
    }
}

impl AddAssign for TnuaVelChange {
    fn add_assign(&mut self, rhs: Self) {
        self.acceleration += rhs.acceleration;
        self.boost += rhs.boost;
    }
}

/// Instructions on how to move forces to the rigid body.
///
/// The physics backend is responsible for reading this component during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors) and apply the forces to the
/// rigid body.
///
/// This documentation uses the term "forces", but in fact these numbers ignore mass and are
/// applied directly to the velocity.
#[derive(Component, Default, Debug)]
pub struct TnuaMotor {
    /// How much velocity to add to the rigid body in the current frame.
    pub lin: TnuaVelChange,

    /// How much angular velocity to add to the rigid body in the current frame, given as the
    /// rotation axis multiplied by the rotation speed in radians per second. Can be extracted from
    /// a quaternion using [`Quaternion::xyz`].
    pub ang: TnuaVelChange,
}

/// An addon for [`TnuaProximitySensor`] that allows it to detect [`TnuaGhostPlatform`] colliders.
///
/// Tnua will register all the ghost platforms encountered by the proximity sensor inside this
/// component, so that other systems may pick one to override the [sensor
/// output](TnuaProximitySensor::output)
///
/// See <https://github.com/idanarye/bevy-tnua/wiki/Jump-fall-Through-Platforms>
///
/// See `TnuaSimpleFallThroughPlatformsHelper`.
#[derive(Component, Default, Debug)]
pub struct TnuaGhostSensor(pub Vec<TnuaProximitySensorOutput>);

impl TnuaGhostSensor {
    pub fn iter(&self) -> impl Iterator<Item = &TnuaProximitySensorOutput> {
        self.0.iter()
    }
}

/// A marker for jump/fall-through platforms.
///
/// Ghost platforms must also have their solver groups (**not** collision groups) set to exclude
/// the character's collider. In order to sense them the player character's sensor must also use
/// [`TnuaGhostSensor`].
///
/// See <https://github.com/idanarye/bevy-tnua/wiki/Jump-fall-Through-Platforms>
///
/// See `TnuaSimpleFallThroughPlatformsHelper`.
#[derive(Component, Default, Debug)]
pub struct TnuaGhostPlatform;

/// Change the gravity for a Tnua-controlled character.
#[derive(Component, Debug)]
pub struct TnuaGravity(pub Vector3);

/// Marker component for colliders which Tnua should not treat as platform.
///
/// This means that the ray/shape cast ignores these hits.
#[derive(Component, Default, Debug)]
pub struct TnuaNotPlatform;
