use bevy::prelude::*;

use crate::TnuaBasis;

pub struct Movement {
    pub desired_velocity: Vec3,
}

impl TnuaBasis for Movement {
    type State = ();

    fn apply(&self, _state: &mut Self::State) {
        info!("MOVING! {:?}", self.desired_velocity);
    }
}
