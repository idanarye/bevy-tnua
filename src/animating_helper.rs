use std::mem::discriminant;

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

pub enum TnuaAnimatingStateDirective<'a, State> {
    Maintain {
        state: &'a State,
    },
    Alter {
        old_state: Option<State>,
        state: &'a State,
    },
}

impl<State> TnuaAnimatingState<State> {
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

    pub fn by_value(&mut self, new_state: State) -> TnuaAnimatingStateDirective<State>
    where
        State: PartialEq,
    {
        self.update_by(new_state, |a, b| a == b)
    }

    pub fn by_discriminant(&mut self, new_state: State) -> TnuaAnimatingStateDirective<State> {
        self.update_by(new_state, |a, b| discriminant(a) == discriminant(b))
    }
}
