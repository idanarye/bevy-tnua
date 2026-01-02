use bevy::prelude::*;

use crate::basis_capabilities::TnuaBasisWithGround;
use crate::controller::TnuaActionFlowStatus;
use crate::{TnuaActionDiscriminant, prelude::*};

/// An helper for tracking air actions.
///
/// It's [`update`](Self::update) must be called every frame - even when the result is not used.
///
/// For simpler usage, see [`TnuaSimpleAirActionsCounter`].
#[derive(Default)]
pub struct TnuaAirActionsTracker {
    considered_in_air: bool,
}

pub trait TnuaAirActionDefinition: TnuaScheme {
    fn is_air_action(action: Self::ActionDiscriminant) -> bool;
}

impl TnuaAirActionsTracker {
    /// Call this every frame to track the air actions.
    pub fn update<S>(
        &mut self,
        controller: &TnuaController<S>,
    ) -> TnuaAirActionsUpdate<S::ActionDiscriminant>
    where
        S: TnuaScheme + TnuaAirActionDefinition,
        S::Basis: TnuaBasisWithGround,
    {
        match controller.action_flow_status() {
            TnuaActionFlowStatus::NoAction => self.update_regardless_of_action(controller),
            TnuaActionFlowStatus::ActionOngoing(action_discriminant) => {
                if controller
                    .action_discriminant()
                    .is_some_and(S::is_air_action)
                {
                    if self.considered_in_air {
                        TnuaAirActionsUpdate::NoChange
                    } else {
                        self.considered_in_air = true;
                        TnuaAirActionsUpdate::AirActionStarted(*action_discriminant)
                    }
                } else {
                    self.update_regardless_of_action(controller)
                }
            }
            TnuaActionFlowStatus::ActionStarted(action_discriminant)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: action_discriminant,
            } => {
                if controller
                    .action_discriminant()
                    .is_some_and(S::is_air_action)
                {
                    self.considered_in_air = true;
                    TnuaAirActionsUpdate::AirActionStarted(*action_discriminant)
                } else {
                    self.update_regardless_of_action(controller)
                }
            }
            TnuaActionFlowStatus::ActionEnded(_) => {
                let result = self.update_regardless_of_action(controller);
                if self.considered_in_air {
                    TnuaAirActionsUpdate::ActionFinishedInAir
                } else {
                    result
                }
            }
        }
    }

    fn update_regardless_of_action<S>(
        &mut self,
        controller: &TnuaController<S>,
    ) -> TnuaAirActionsUpdate<S::ActionDiscriminant>
    where
        S: TnuaScheme,
        S::Basis: TnuaBasisWithGround,
    {
        if let Ok(basis_access) = controller.basis_access() {
            if S::Basis::is_airborne(&basis_access) {
                if self.considered_in_air {
                    TnuaAirActionsUpdate::NoChange
                } else {
                    self.considered_in_air = true;
                    TnuaAirActionsUpdate::FreeFallStarted
                }
            } else if self.considered_in_air {
                self.considered_in_air = false;
                TnuaAirActionsUpdate::JustLanded
            } else {
                TnuaAirActionsUpdate::NoChange
            }
        } else {
            TnuaAirActionsUpdate::NoChange
        }
    }
}

/// The result of [`TnuaAirActionsTracker::update()`].
#[derive(Debug, Clone, Copy)]
pub enum TnuaAirActionsUpdate<D: TnuaActionDiscriminant> {
    /// Nothing of interest happened this frame.
    NoChange,

    /// The character has just started a free fall this frame.
    FreeFallStarted,

    /// The character has just started an air action this frame.
    AirActionStarted(D),

    /// The character has just finished an air action this frame, and is still in the air.
    ActionFinishedInAir,

    /// The character has just landed this frame.
    JustLanded,
}

/// A simple counter that counts together all the air actions a character is able to perform.
///
/// It's [`update`](Self::update) must be called every frame.
#[derive(Component)]
pub struct TnuaSimpleAirActionsCounter<S: TnuaScheme> {
    tracker: TnuaAirActionsTracker,
    current_action: Option<(S::ActionDiscriminant, usize)>,
    air_actions_count: usize,
}

impl<S: TnuaScheme> Default for TnuaSimpleAirActionsCounter<S> {
    fn default() -> Self {
        Self {
            tracker: Default::default(),
            current_action: None,
            air_actions_count: 0,
        }
    }
}

impl<S> TnuaSimpleAirActionsCounter<S>
where
    S: TnuaScheme + TnuaAirActionDefinition,
    S::Basis: TnuaBasisWithGround,
{
    /// Call this every frame to track the air actions.
    pub fn update(&mut self, controller: &TnuaController<S>)
    where
        S: TnuaScheme + TnuaAirActionDefinition,
        S::Basis: TnuaBasisWithGround,
    {
        let update = self.tracker.update(controller);
        match update {
            TnuaAirActionsUpdate::NoChange => {}
            TnuaAirActionsUpdate::FreeFallStarted => {
                // The free fall is considered the first action
                self.current_action = None;
                self.air_actions_count += 1;
            }
            TnuaAirActionsUpdate::AirActionStarted(action_discriminant) => {
                self.current_action = Some((action_discriminant, self.air_actions_count));
                self.air_actions_count += 1;
            }
            TnuaAirActionsUpdate::ActionFinishedInAir => {
                self.current_action = None;
            }
            TnuaAirActionsUpdate::JustLanded => {
                self.current_action = None;
                self.air_actions_count = 0;
            }
        }
    }

    /// Resets the air actions counter to a specific count, excluding the current action.
    ///
    /// This method allows you to manually set the count of air actions (excluding the current
    /// action) to a specified value. Use this when you need to synchronize or initialize the air
    /// actions count to a specific state.
    ///
    /// # Arguments
    ///
    /// * `count` - The new count to set for air actions, excluding the current action.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_tnua::control_helpers::{TnuaSimpleAirActionsCounter, TnuaAirActionDefinition};
    /// # #[derive(bevy_tnua::TnuaScheme)] #[scheme(basis = bevy_tnua::builtins::TnuaBuiltinWalk)] enum ControlScheme {}
    /// # impl TnuaAirActionDefinition for ControlScheme { fn is_air_action(_: Self::ActionDiscriminant) -> bool { false } }
    /// # let mut air_actions_counter = TnuaSimpleAirActionsCounter::<ControlScheme>::default();
    ///
    /// // Reset the air actions count to 3 (excluding the current action). should also be updated as stated in TnuaAirActionsTracker
    /// air_actions_counter.reset_count_to(3);
    /// ```
    pub fn reset_count_to(&mut self, count: usize) {
        self.air_actions_count = count;
    }

    /// Obtain a mutable reference to the air counter.
    ///
    /// This can be use to modify the air counter while the player is in the air - for example,
    /// restoring an air jump when they pick up a floating token.
    ///
    /// When it fits the usage, prefer [`reset_count`](Self::reset_count) which is simpler.
    /// `get_count_mut` should be used for more complex cases, e.g. when the player is allowed
    /// multiple air jumps, but only one jump gets restored per token.
    ///
    /// Note that:
    ///
    /// * When the character is grounded, this method returns `None`. This is only for mutating the
    ///   counter while the character is airborne.
    /// * When the character jumps from the ground, or starts a free fall, the counter is one - not
    ///   zero. Setting the counter to 0 will mean that the next air jump will actually be treated
    ///   as a ground jump - and they'll get another air jump in addition to it. This is usually
    ///   not the desired behavior.
    /// * Changing the action counter returned by this method will not affect the value
    ///   [`air_count_for`](Self::air_count_for) returns for an action that continues to be fed.
    pub fn get_count_mut(&mut self) -> Option<&mut usize> {
        if self.air_actions_count == 0 {
            None
        } else {
            Some(&mut self.air_actions_count)
        }
    }

    /// Resets the air actions counter.
    ///
    /// This is equivalent to setting the counter to 1 using:
    ///
    /// ```no_run
    /// # use bevy_tnua::control_helpers::{TnuaSimpleAirActionsCounter, TnuaAirActionDefinition};
    /// # #[derive(bevy_tnua::TnuaScheme)] #[scheme(basis = bevy_tnua::builtins::TnuaBuiltinWalk)] enum ControlScheme {}
    /// # impl TnuaAirActionDefinition for ControlScheme { fn is_air_action(_: Self::ActionDiscriminant) -> bool { false } }
    /// # let mut air_actions_counter = TnuaSimpleAirActionsCounter::<ControlScheme>::default();
    /// if let Some(count) = air_actions_counter.get_count_mut() {
    ///     *count = 1;
    /// }
    /// ```
    ///
    /// The reason it is set to 1 and not 0 is that when the character jumps from the ground or
    /// starts a free fall the counter is 1 - and this is what one would usually want to reset to.
    /// Having a counter of 0 means that the character is grounded - but in that case
    /// [`get_count_mut`](Self::get_count_mut) will return `None` and the counter will not change.
    pub fn reset_count(&mut self) {
        if let Some(count) = self.get_count_mut() {
            *count = 1;
        }
    }

    /// Calculate the "air number" of an action.
    ///
    /// The air number of a ground action is 0. The first air jump (double jump) as an air number
    /// of 1, the second (triple jump) has an air number of 2 and so on. Other air actions (like
    /// air dashes) are counted together with the jumps.
    ///
    /// Use this number to:
    /// 1. Determine if the action is allowed.
    /// 2. Optionally change the action's parameters as the air number progresses.
    ///
    /// Note that the action discriminant is important, because Tnua relies on constant feed of
    /// some actions. As long as you pass the correct discriminant, the number will not change
    /// while the action continues to be fed. The discriminant can be obtained with
    /// [`TnuaController::action_discriminant`].
    pub fn air_count_for(&self, action: S::ActionDiscriminant) -> usize {
        if let Some((current_action, actions)) = self.current_action
            && current_action == action
        {
            return actions;
        }
        self.air_actions_count
    }
}
