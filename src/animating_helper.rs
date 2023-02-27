use bevy::prelude::*;

#[derive(Component)]
pub struct TnuaAnimatingState<State> {
    state: Option<State>,
}

impl<State> Default for TnuaAnimatingState<State> {
    fn default() -> Self {
        Self { state: None }
    }
}

pub enum TnuaAnimatingStateDirective<State, Control> {
    Maintain {
        state: State,
        control: Control,
    },
    Alter {
        old_state: Option<State>,
        state: State,
        control: Control,
    },
}

impl<State: Clone + PartialEq> TnuaAnimatingState<State> {
    pub fn update<Control>(
        &mut self,
        dlg: impl FnOnce() -> (State, Control),
    ) -> TnuaAnimatingStateDirective<State, Control> {
        let (new_state, control) = dlg();
        if let Some(old_state) = self.state.as_ref() {
            if *old_state == new_state {
                return TnuaAnimatingStateDirective::Maintain {
                    state: new_state,
                    control,
                };
            }
        }
        return TnuaAnimatingStateDirective::Alter {
            old_state: self.state.replace(new_state.clone()),
            state: new_state,
            control,
        };
    }
}
