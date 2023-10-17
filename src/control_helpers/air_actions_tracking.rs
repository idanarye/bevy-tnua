use bevy::prelude::*;

use crate::controller::TnuaActionFlowStatus;
use crate::prelude::*;

#[derive(Default)]
pub struct TnuaAirActionsTracker {
    considered_in_air: bool,
}

impl TnuaAirActionsTracker {
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

#[derive(Debug, Clone, Copy)]
pub enum TnuaAirActionsUpdate {
    NoChange,
    FreeFallStarted,
    AirActionStarted(&'static str),
    ActionFinishedInAir,
    JustLanded,
}

#[derive(Component, Default)]
pub struct TnuaSimpleAirActionsCounter {
    tracker: TnuaAirActionsTracker,
    current_action: Option<&'static str>,
    actions_including_current: usize,
}

impl TnuaSimpleAirActionsCounter {
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

    pub fn air_count_for(&self, action_name: &str) -> usize {
        if self.current_action == Some(action_name) {
            self.actions_including_current - 1
        } else {
            self.actions_including_current
        }
    }
}
