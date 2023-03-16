use bevy::prelude::*;

/// Newtonian state of the rigid body.
///
/// Tnua takes the position and rotation of the rigid body from its `GlobalTransform`, but things
/// like velocity are dependent on the physics engine. The physics backend is responsible for
/// updating this component from the physics engine during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors).
#[derive(Component, Debug)]
pub struct TnuaRigidBodyTracker {
    pub velocity: Vec3,
    /// Angular velocity as the rotation axis multiplied by the rotation speed in radians per
    /// second. Can be extracted from a quaternion using [`Quat::xyz`].
    pub angvel: Vec3,
    pub gravity: Vec3,
}

impl Default for TnuaRigidBodyTracker {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            angvel: Vec3::ZERO,
            gravity: Vec3::ZERO,
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
    pub cast_origin: Vec3,
    /// The direction in world coord system (unmodified by the entity's transform)
    pub cast_direction: Vec3,
    /// Tnua will update this field according to its need. The backend only needs to read it.
    pub cast_range: f32,
    pub output: Option<TnuaProximitySensorOutput>,
}

impl Default for TnuaProximitySensor {
    fn default() -> Self {
        Self {
            cast_origin: Vec3::ZERO,
            cast_direction: -Vec3::Y,
            cast_range: 0.0,
            output: None,
        }
    }
}

/// Information from [`TnuaProximitySensor`] that have detected another collider.
#[derive(Debug)]
pub struct TnuaProximitySensorOutput {
    /// The entity of the collider detected by the ray.
    pub entity: Entity,
    /// The distance to the collider from [`cast_origin`](TnuaProximitySensor::cast_origin) along the
    /// [`cast_direction`](TnuaProximitySensor::cast_direction).
    pub proximity: f32,
    /// The normal from the detected collider's surface where the ray hits.
    pub normal: Vec3,
    /// The velocity of the detected entity,
    pub entity_linvel: Vec3,
    /// The angular velocity of the detected entity, given as the rotation axis multiplied by the
    /// rotation speed in radians per second. Can be extracted from a quaternion using
    /// [`Quat::xyz`].
    pub entity_angvel: Vec3,
}

/// Instructions on how to move forces to the rigid body.
///
/// The physics backend is responsible for reading this component during
/// [`TnuaPipelineStages::Sensors`](crate::TnuaPipelineStages::Sensors) and apply the forces to the
/// rigid body.
///
/// This documentation uses the term "forces", but in fact these numbers ignore mass and are
/// applied directly to the velocity.
#[derive(Component, Default)]
pub struct TnuaMotor {
    /// How much velocity to add to the rigid body in the current frame. Does not get multiplied by
    /// the frame's duration - Tnua already does that multiplication.
    pub desired_acceleration: Vec3,
    /// How much angular velocity to add to the rigid body in the current frame, given as the
    /// rotation axis multiplied by the rotation speed in radians per second. Can be extracted from
    /// a quaternion using [`Quat::xyz`]. Does not get multiplied by the frame's duration - Tnua
    /// already does that multiplication.
    pub desired_angacl: Vec3,
}
