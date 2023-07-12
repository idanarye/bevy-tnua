// use bevy::prelude::*;

use std::any::Any;

pub trait TnuaBasis: 'static + Send + Sync {
    type State: Default + Send + Sync;

    fn apply(&self, state: &mut Self::State);
}

pub(crate) trait DynamicBasis: Send + Sync + Any + 'static {
    fn apply(&mut self);

    fn as_mut_any(&mut self) -> &mut dyn Any;
}

pub(crate) struct BoxableBasis<B: TnuaBasis> {
    pub(crate) input: B,
    pub(crate) state: B::State,
}

impl<B: TnuaBasis> BoxableBasis<B> {
    pub(crate) fn new(basis: B) -> Self {
        Self {
            input: basis,
            state: Default::default(),
        }
    }
}

impl<B: TnuaBasis> DynamicBasis for BoxableBasis<B> {
    fn apply(&mut self) {
        self.input.apply(&mut self.state);
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}
