use bevy::prelude::*;

use crate::controller::TnuaActionFlowStatus;
use crate::prelude::*;

/// An helper for tracking air actions.
///
/// It's [`update`](Self::update) must be called every frame - even when the result is not used.
///
/// For simpler usage, see [`TnuaSimpleAirActionsCounter`].
#[derive(Default)]
pub struct TnuaAirActionsTracker {
    considered_in_air: bool,
}

impl TnuaAirActionsTracker {
    /// Call this every frame to track the air actions.
    pub fn update(&mut self, controller: &TnuaController) -> TnuaAirActionsUpdate {
        match controller.action_flow_status() {
            TnuaActionFlowStatus::NoAction => self.update_regardless_of_action(controller),
            TnuaActionFlowStatus::ActionOngoing(action_name) => {
                if controller
                    .dynamic_action()
                    .map_or(false, |action| action.violates_coyote_time())
                {
                    if self.considered_in_air {
                        TnuaAirActionsUpdate::NoChange
                    } else {
                        self.considered_in_air = true;
                        TnuaAirActionsUpdate::AirActionStarted(action_name)
                    }
                } else {
                    self.update_regardless_of_action(controller)
                }
            }
            TnuaActionFlowStatus::ActionStarted(action_name)
            | TnuaActionFlowStatus::Cancelled {
                old: _,
                new: action_name,
            } => {
                if controller
                    .dynamic_action()
                    .map_or(false, |action| action.violates_coyote_time())
                {
                    self.considered_in_air = true;
                    TnuaAirActionsUpdate::AirActionStarted(action_name)
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

    fn update_regardless_of_action(&mut self, controller: &TnuaController) -> TnuaAirActionsUpdate {
        if let Some(basis) = controller.dynamic_basis() {
            if basis.is_airborne() {
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
pub enum TnuaAirActionsUpdate {
    /// Nothing of interest happened this frame.
    NoChange,

    /// The character has just started a free fall this frame.
    FreeFallStarted,

    /// The character has just started an air action this frame.
    AirActionStarted(&'static str),

    /// The character has just finished an air action this frame, and is still in the air.
    ActionFinishedInAir,

    /// The character has just landed this frame.
    JustLanded,
}

/// A simple counter that counts together all the air actions a character is able to perform.
///
/// It's [`update`](Self::update) must be called every frame.
#[derive(Component, Default)]
pub struct TnuaSimpleAirActionsCounter {
    tracker: TnuaAirActionsTracker,
    current_action: Option<&'static str>,
    actions_including_current: usize,
}

impl TnuaSimpleAirActionsCounter {
    /// Call this every frame to track the air actions.
    pub fn update(&mut self, controller: &TnuaController) {
        let update = self.tracker.update(controller);
        match update {
            TnuaAirActionsUpdate::NoChange => {}
            TnuaAirActionsUpdate::FreeFallStarted => {
                // The free fall is considered the first action
                self.current_action = None;
                self.actions_including_current += 1;
            }
            TnuaAirActionsUpdate::AirActionStarted(action_name) => {
                self.current_action = Some(action_name);
                self.actions_including_current += 1;
            }
            TnuaAirActionsUpdate::ActionFinishedInAir => {
                self.current_action = None;
            }
            TnuaAirActionsUpdate::JustLanded => {
                self.current_action = None;
                self.actions_including_current = 0;
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
    /// use bevy_tnua::control_helpers::TnuaSimpleAirActionsCounter;
    /// let mut air_actions_counter = TnuaSimpleAirActionsCounter::default();
    ///
    /// // Reset the air actions count to 3 (excluding the current action). should also be updated as stated in TnuaAirActionsTracker
    /// air_actions_counter.reset_count_to(3);
    /// ```
    pub fn reset_count_to(&mut self, count: usize) {
        self.actions_including_current = count;
    }

    /// Resets the air actions counter to zero, excluding the current action.
    ///
    /// This method is a convenience function equivalent to calling `reset_count_to(0)`. It sets
    /// the count of air actions (excluding the current action) to zero.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_tnua::control_helpers::TnuaSimpleAirActionsCounter;
    /// let mut air_actions_counter = TnuaSimpleAirActionsCounter::default();
    ///
    /// // Reset the air actions count to zero (excluding the current action). should also be updated as stated in TnuaAirActionsTracker
    /// air_actions_counter.reset_count();
    /// ```
    pub fn reset_count(&mut self) {
        self.reset_count_to(0);
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
    /// Note that the action name is important, because Tnua relies on constant feed of some
    /// actions. As long as you pass the correct name, the number will not change while the action
    /// continues to be fed. The correct name is [`TnuaAction::NAME`] when using
    /// [`TnuaController::action`] or the first argument when using
    /// [`TnuaController::named_action`].
    pub fn air_count_for(&self, action_name: &str) -> usize {
        if self.current_action == Some(action_name) {
            self.actions_including_current.saturating_sub(1)
        } else {
            self.actions_including_current
        }
    }
}
