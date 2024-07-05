use std::mem::discriminant;

use bevy::prelude::*;

/// Utility for deciding which animation to play.
///
/// Add `TnuaAnimatingState<State>` as a component, where `State` is a data type - usually an
/// `enum` - that determines which animation to play. Each frame, decide (with the help of
/// [`TnuaController`](crate::prelude::TnuaController)) which animation should
/// be played and the animation's parameters (like speed) and feed it to the `TnuaAnimatingState`.
/// Use the emitted [`TnuaAnimatingStateDirective`] to determine if this is a new animation or an
/// existing one (possibly with different parameters), and use that information to work the actual
/// animation player.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_tnua::prelude::*;
/// # use bevy_tnua::{TnuaAnimatingState, TnuaAnimatingStateDirective};
/// # use bevy_tnua::math::Float;
/// # #[derive(Resource)]
/// # struct AnimationNodes {
/// #     standing: AnimationNodeIndex,
/// #     running: AnimationNodeIndex,
/// # }
/// enum AnimationState {
///     Standing,
///     Running(Float),
/// }
///
/// fn animating_system(
///     mut query: &mut Query<(
///         &mut TnuaAnimatingState<AnimationState>,
///         &TnuaController,
///         &mut AnimationPlayer,
///     )>,
///     animation_nodes: Res<AnimationNodes>,
/// ) {
///     for (mut animating_state, controller, mut animation_player) in query.iter_mut() {
///         match animating_state.update_by_discriminant({
///             let Some((_, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>()
///             else {
///                 continue;
///             };
///             let speed = basis_state.running_velocity.length();
///             if 0.01 < speed {
///                 AnimationState::Running(speed)
///             } else {
///                 AnimationState::Standing
///             }
///         }) {
///             TnuaAnimatingStateDirective::Maintain { state } => {
///                 if let AnimationState::Running(speed) = state {
///                     if let Some(active_animation) = animation_player.animation_mut(animation_nodes.running) {
///                         active_animation.set_speed(*speed);
///                     }
///                 }
///             }
///             TnuaAnimatingStateDirective::Alter {
///                 // We don't need the old state here, but it's available for transition
///                 // animations.
///                 old_state: _,
///                 state,
///             } => match state {
///                 AnimationState::Standing => {
///                     animation_player
///                         .start(animation_nodes.standing)
///                         .set_speed(1.0)
///                         .repeat();
///                 }
///                 AnimationState::Running(speed) => {
///                     animation_player
///                         .start(animation_nodes.running)
///                         .set_speed(*speed)
///                         .repeat();
///                 }
///             }
///         }
///     }
/// }
/// ```
#[derive(Component)]
pub struct TnuaAnimatingState<State> {
    state: Option<State>,
}

impl<State> Default for TnuaAnimatingState<State> {
    fn default() -> Self {
        Self { state: None }
    }
}

pub enum TnuaAnimatingStateDirective<'a, State> {
    /// The animation to play remains the same - possibly with different parameters.
    Maintain { state: &'a State },
    /// A different animation needs to be played.
    ///
    /// Also returned (with `old_state: None`) if this is the first animation to be played.
    Alter {
        old_state: Option<State>,
        state: &'a State,
    },
}

impl<State> TnuaAnimatingState<State> {
    /// Consider a new animation to play.
    ///
    /// The comparison function decides if its the same animation (possibly with different
    /// parameters) or a different animation.
    pub fn update_by(
        &mut self,
        new_state: State,
        comparison: impl FnOnce(&State, &State) -> bool,
    ) -> TnuaAnimatingStateDirective<State> {
        let is_same = self
            .state
            .as_ref()
            .map_or(false, |old_state| comparison(old_state, &new_state));
        let old_state = self.state.replace(new_state);
        if is_same {
            TnuaAnimatingStateDirective::Maintain {
                state: self.state.as_ref().expect("state was just placed there"),
            }
        } else {
            TnuaAnimatingStateDirective::Alter {
                old_state,
                state: self.state.as_ref().expect("state was just placed there"),
            }
        }
    }

    /// Consider a new animation to play.
    ///
    /// The new animation is considered the same if and only if it is equal to the old animation.
    pub fn update_by_value(&mut self, new_state: State) -> TnuaAnimatingStateDirective<State>
    where
        State: PartialEq,
    {
        self.update_by(new_state, |a, b| a == b)
    }

    /// Consider a new animation to play.
    ///
    /// The new animation is considered the same if it is the same variant of the enum as the old
    /// animation.
    ///
    /// If the `State` is not an `enum`, using this method will not result in undefined behavior,
    /// but the behavior is unspecified.
    pub fn update_by_discriminant(
        &mut self,
        new_state: State,
    ) -> TnuaAnimatingStateDirective<State> {
        self.update_by(new_state, |a, b| discriminant(a) == discriminant(b))
    }
}
