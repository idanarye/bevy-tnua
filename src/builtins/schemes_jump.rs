use crate::math::*;
use crate::schemes_traits::{Tnua2Action, Tnua2Basis};
use bevy::prelude::*;

#[derive(Default)]
pub struct Tnua2BuiltinJump {
    pub vertical_displacement: Option<Vector3>,

    /// Allow this action to start even if the character is not touching ground nor in coyote time.
    pub allow_in_air: bool,

    /// Force the character to face in a particular direction.
    ///
    /// Note that there are no acceleration limits because unlike
    /// [crate::prelude::TnuaBuiltinWalk::desired_forward] this field will attempt to force the
    /// direction during a single frame. It is useful for when the jump animation needs to be
    /// aligned with the [`vertical_displacement`](Self::vertical_displacement).
    pub force_forward: Option<Dir3>,
}

#[derive(Clone)]
pub struct Tnua2BuiltinJumpConfig {
    /// The height the character will jump to.
    ///
    /// If [`shorten_extra_gravity`](Self::shorten_extra_gravity) is higher than `0.0`, the
    /// character may stop the jump in the middle if the jump action is no longer fed (usually when
    /// the player releases the jump button) and the character may not reach its full jump height.
    ///
    /// The jump height is calculated from the center of the character at float_height to the
    /// center of the character at the top of the jump. It _does not_ mean the height from the
    /// ground. The float height is calculated by the inspecting the character's current position
    /// and the basis' [`displacement`](crate::TnuaBasis::displacement).
    pub height: Float,

    /// Extra gravity for breaking too fast jump from running up a slope.
    ///
    /// When running up a slope, the character gets more jump strength to avoid slamming into the
    /// slope. This may cause the jump to be too high, so this value is used to brake it.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub upslope_extra_gravity: Float,

    /// Extra gravity for fast takeoff.
    ///
    /// Without this, jumps feel painfully slow. Adding this will apply extra gravity until the
    /// vertical velocity reaches below [`takeoff_above_velocity`](Self::takeoff_above_velocity),
    /// and increase the initial jump boost in order to compensate. This will make the jump feel
    /// more snappy.
    pub takeoff_extra_gravity: Float,

    /// The range of upward velocity during [`takeoff_extra_gravity`](Self::takeoff_extra_gravity)
    /// is applied.
    ///
    /// To disable, set this to [`Float::INFINITY`] rather than zero.
    pub takeoff_above_velocity: Float,

    /// Extra gravity for falling down after reaching the top of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub fall_extra_gravity: Float,

    /// Extra gravity for shortening a jump when the player releases the jump button.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub shorten_extra_gravity: Float,

    /// Used to decrease the time the character spends "floating" at the peak of the jump.
    ///
    /// When the character's upward velocity is above this value,
    /// [`peak_prevention_extra_gravity`](Self::peak_prevention_extra_gravity) will be added to the
    /// gravity in order to shorten the float time.
    ///
    /// This extra gravity is taken into account when calculating the initial jump speed, so the
    /// character is still supposed to reach its full jump [`height`](Self::height).
    pub peak_prevention_at_upward_velocity: Float,

    /// Extra gravity for decreasing the time the character spends at the peak of the jump.
    ///
    /// **NOTE**: This force will be added to the normal gravity.
    pub peak_prevention_extra_gravity: Float,

    /// A duration, in seconds, after which the character would jump if the jump button was already
    /// pressed when the jump became available.
    ///
    /// The duration is measured from the moment the jump became available - not from the moment
    /// the jump button was pressed.
    ///
    /// When set to `None`, the character will not jump no matter how long the player holds the
    /// jump button.
    ///
    /// If the jump button is held but the jump input is still buffered (see
    /// [`input_buffer_time`](Self::input_buffer_time)), this setting will have no effect because
    /// the character will simply jump immediately.
    pub reschedule_cooldown: Option<Float>,

    /// A duration, in seconds, where a player can press a jump button before a jump becomes
    /// possible (typically when a character is still in the air and about the land) and the jump
    /// action would still get registered and be executed once the jump is possible.
    pub input_buffer_time: Float,

    pub disable_force_forward_after_peak: bool,
}

impl Default for Tnua2BuiltinJumpConfig {
    fn default() -> Self {
        Self {
            height: 0.0,
            upslope_extra_gravity: 30.0,
            takeoff_extra_gravity: 30.0,
            takeoff_above_velocity: 2.0,
            fall_extra_gravity: 20.0,
            shorten_extra_gravity: 60.0,
            peak_prevention_at_upward_velocity: 1.0,
            peak_prevention_extra_gravity: 20.0,
            reschedule_cooldown: None,
            input_buffer_time: 0.2,
            disable_force_forward_after_peak: true,
        }
    }
}

#[derive(Default)]
pub struct Tnua2BuiltinJumpMemory {}

impl<B: Tnua2Basis> Tnua2Action<B> for Tnua2BuiltinJump {
    type Config = Tnua2BuiltinJumpConfig;
    type Memory = Tnua2BuiltinJumpMemory;

    fn apply(
        &self,
        config: &Self::Config,
        memory: &mut Self::Memory,
        ctx: crate::schemes_traits::Tnua2ActionContext<B>,
        lifecycle_status: crate::TnuaActionLifecycleStatus,
        motor: &mut bevy_tnua_physics_integration_layer::data_for_backends::TnuaMotor,
    ) -> crate::TnuaActionLifecycleDirective {
        info!("Applying jump");
        lifecycle_status.directive_simple()
    }
}
