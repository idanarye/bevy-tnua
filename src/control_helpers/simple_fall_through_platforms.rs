use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::math::Float;

use crate::{TnuaGhostSensor, TnuaProximitySensor};

/// Helper component for implementing fall-through platforms.
///
/// See <https://github.com/idanarye/bevy-tnua/wiki/Jump-fall-Through-Platforms>
///
/// Place this component on the characetr entity (the one that has the [`TnuaProximitySensor`] and
/// the [`TnuaGhostSensor`]) and inside a system that runs in
/// [`TnuaUserControlsSystemSet`](crate::TnuaUserControlsSystemSet) (typically the player controls
/// system) use [`with`](Self::with) and call one of the methods of [the returned handle
/// object](TnuaHandleForSimpleFallThroughPlatformsHelper) every frame. See the description of
/// these methods to determine which one to call.
#[derive(Component, Default)]
pub struct TnuaSimpleFallThroughPlatformsHelper {
    currently_falling_through: HashSet<Entity>,
}

impl TnuaSimpleFallThroughPlatformsHelper {
    /// Get an handle for operating the helper.
    ///
    /// The `min_proximity` argument is the minimal distance from the origin of the cast ray/shape
    /// (usually the center of the character) to the platform. If the distance to the platform is
    /// below that, the helper will assume that the character only jumped halfway through it, not
    /// high enough to stand on it.
    pub fn with<'a>(
        &'a mut self,
        proximity_sensor: &'a mut TnuaProximitySensor,
        ghost_sensor: &'a TnuaGhostSensor,
        min_proximity: Float,
    ) -> TnuaHandleForSimpleFallThroughPlatformsHelper<'a> {
        TnuaHandleForSimpleFallThroughPlatformsHelper {
            parent: self,
            proximity_sensor,
            ghost_sensor,
            min_proximity,
        }
    }
}

/// Handle for working with [`TnuaSimpleFallThroughPlatformsHelper`].
///
/// This object should be created each frame, and one of its methods should be called depending on
/// whether the character wants to keep standing on the platform or fall through it.
pub struct TnuaHandleForSimpleFallThroughPlatformsHelper<'a> {
    parent: &'a mut TnuaSimpleFallThroughPlatformsHelper,
    proximity_sensor: &'a mut TnuaProximitySensor,
    ghost_sensor: &'a TnuaGhostSensor,
    min_proximity: Float,
}

impl TnuaHandleForSimpleFallThroughPlatformsHelper<'_> {
    /// Call this method to make the character stand on the platform (if there is any)
    pub fn dont_fall(&mut self) {
        let mut already_falling_through_not_yet_seen =
            self.parent.currently_falling_through.clone();
        for ghost_platform in self.ghost_sensor.iter() {
            if self.min_proximity <= ghost_platform.proximity
                && !already_falling_through_not_yet_seen.remove(&ghost_platform.entity)
            {
                self.proximity_sensor.output = Some(ghost_platform.clone());
                break;
            }
        }
        self.parent
            .currently_falling_through
            .retain(|entity| !already_falling_through_not_yet_seen.contains(entity));
    }

    /// Call this method to make the character drop through the platform.
    ///
    /// The character will fall through the first layer of ghost platforms detected since the last
    /// time it was called with `just_pressed` being `true`. This means that:
    ///
    /// * To let the player fall through all the platforms by simply holding the button, call this
    ///   with `just_pressed = true` as long as the button is held.
    /// * To let the player fall through one layer of platforms at a time, forcing them to release
    ///   and press again for each layer, pass `just_pressed = true` only when the button really is
    ///   just pressed.
    ///
    /// Returns `true` if actually dropping through a platform, to help determining if the
    /// character should be crouching (since these buttons are usually the same)
    pub fn try_falling(&mut self, just_pressed: bool) -> bool {
        if !just_pressed && !self.parent.currently_falling_through.is_empty() {
            for ghost_platform in self.ghost_sensor.iter() {
                if self.min_proximity <= ghost_platform.proximity
                    && !self
                        .parent
                        .currently_falling_through
                        .contains(&ghost_platform.entity)
                {
                    self.proximity_sensor.output = Some(ghost_platform.clone());
                    return true;
                }
            }
            return true;
        }
        self.parent.currently_falling_through.clear();
        for ghost_platform in self.ghost_sensor.iter() {
            if self.min_proximity <= ghost_platform.proximity {
                self.parent
                    .currently_falling_through
                    .insert(ghost_platform.entity);
            }
        }
        !self.parent.currently_falling_through.is_empty()
    }
}
