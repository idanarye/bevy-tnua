use crate::{
    math::{AdjustPrecision, AsF32, Float, Vector3},
    util::calc_angular_velchange_to_force_forward,
    TnuaAction, TnuaActionInitiationDirective, TnuaActionLifecycleDirective,
    TnuaActionLifecycleStatus, TnuaMotor, TnuaVelChange,
};
use bevy::prelude::*;

/// An [action](TnuaAction) for sliding on walls.
#[derive(Clone)]
pub struct TnuaBuiltinWallSlide {
    /// The entity of the wall to slide on.
    pub wall_entity: Option<Entity>,

    /// The on the wall where the character touches it.
    ///
    /// Note that this does not actually have to be on an actual wall. It can be a point in the
    /// middle of the air, and the action will cause the character to pretend there is a wall there
    /// and slide on it.
    pub contact_point_with_wall: Vector3,

    /// The wall's normal
    pub normal: Dir3,

    /// Force the character to face in a particular direction.
    pub force_forward: Option<Dir3>,

    /// When the character slides faster than that speed, slow it down.
    pub max_fall_speed: Float,

    /// A distance to maintain from the wall.
    ///
    /// Specifically - the distance from
    /// [`contact_point_with_wall`](Self::contact_point_with_wall) in the direction of the
    /// [`normal`](Self::normal).
    pub maintain_distance: Option<Float>,

    /// The maximum speed the character is allowed to move sideways on the wall while sliding
    /// down on it.
    pub max_sideways_speed: Float,

    /// The maximum acceleration the character is allowed to move sideways on the wall while
    /// sliding down on it.
    ///
    /// Note that this also apply to the acceleration used to brake the character's horitonztal
    /// movement when it enters the wall slide faster than
    /// [`max_sideways_speed`](Self::max_sideways_speed).
    pub max_sideways_acceleration: Float,
}

impl Default for TnuaBuiltinWallSlide {
    fn default() -> Self {
        Self {
            wall_entity: None,
            contact_point_with_wall: Vector3::ZERO,
            normal: Dir3::Y,
            force_forward: None, // obvisouly invalid value
            max_fall_speed: 2.0,
            maintain_distance: None,
            max_sideways_speed: 1.0,
            max_sideways_acceleration: 60.0,
        }
    }
}

impl TnuaAction for TnuaBuiltinWallSlide {
    const NAME: &'static str = "TnuaBuiltinWallSlide";

    type Memory = TnuaBuiltinWallSlideMemory;

    const VIOLATES_COYOTE_TIME: bool = true;

    fn apply(
        &self,
        _memory: &mut Self::Memory,
        ctx: crate::TnuaActionContext,
        lifecycle_status: TnuaActionLifecycleStatus,
        motor: &mut TnuaMotor,
    ) -> TnuaActionLifecycleDirective {
        if !lifecycle_status.is_active() {
            return TnuaActionLifecycleDirective::Finished;
        }

        let downward_speed = -ctx
            .tracker
            .velocity
            .dot(ctx.up_direction.adjust_precision());
        let desired_upward_boost = downward_speed - self.max_fall_speed;
        let actual_upwrad_boost = motor
            .lin
            .calc_boost(ctx.frame_duration)
            .dot(ctx.up_direction.adjust_precision());
        let upward_boost_for_compensation = desired_upward_boost - actual_upwrad_boost;
        motor.lin += TnuaVelChange::acceleration(
            upward_boost_for_compensation * ctx.up_direction.adjust_precision()
                / ctx.frame_duration,
        );

        if let Some(maintain_distance) = self.maintain_distance {
            let planar_vector = (ctx.tracker.translation - self.contact_point_with_wall)
                .reject_from(ctx.up_direction.adjust_precision());
            if let Ok((cling_direction, current_cling_distance)) =
                Dir3::new_and_length(planar_vector.f32())
            {
                let current_cling_speed =
                    ctx.tracker.velocity.dot(cling_direction.adjust_precision());
                let desired_cling_speed = (maintain_distance
                    - current_cling_distance.adjust_precision())
                    / ctx.frame_duration;
                let cling_boost = desired_cling_speed - current_cling_speed;
                motor.lin.cancel_on_axis(cling_direction.adjust_precision());
                motor.lin += TnuaVelChange::boost(cling_boost * cling_direction.adjust_precision());
            }
        }

        let sideways_direction = self.normal.cross(*ctx.up_direction).adjust_precision();
        let projected_sideways_velocity =
            sideways_direction.dot(ctx.tracker.velocity + motor.lin.calc_boost(ctx.frame_duration));
        if self.max_sideways_speed < projected_sideways_velocity.abs() {
            let desired_sideways_velocity =
                self.max_sideways_speed * projected_sideways_velocity.signum();
            let desired_sideways_boost = desired_sideways_velocity - projected_sideways_velocity;
            let desired_sideways_acceleration = desired_sideways_boost / ctx.frame_duration;
            motor.lin +=
                TnuaVelChange::acceleration(sideways_direction * desired_sideways_acceleration);
        }

        let sideways_acceleration =
            sideways_direction.dot(motor.lin.calc_acceleration(ctx.frame_duration));
        if self.max_sideways_acceleration < sideways_acceleration.abs() {
            let desired_sideways_acceleration =
                self.max_sideways_acceleration * sideways_acceleration.signum();
            motor.lin += TnuaVelChange::acceleration(
                sideways_direction * (desired_sideways_acceleration - sideways_acceleration),
            );
        }

        if let Some(force_forward) = self.force_forward {
            motor
                .ang
                .cancel_on_axis(ctx.up_direction.adjust_precision());
            motor.ang += calc_angular_velchange_to_force_forward(
                force_forward,
                ctx.tracker.rotation,
                ctx.tracker.angvel,
                ctx.up_direction,
                ctx.frame_duration,
            );
        }

        TnuaActionLifecycleDirective::StillActive
    }

    fn initiation_decision(
        &self,
        _ctx: crate::TnuaActionContext,
        _being_fed_for: &bevy::time::Stopwatch,
    ) -> TnuaActionInitiationDirective {
        TnuaActionInitiationDirective::Allow
    }
}

#[derive(Default, Debug)]
pub struct TnuaBuiltinWallSlideMemory {}
