use bevy::prelude::*;

use crate::TnuaBasis;

pub struct Movement {
    pub desired_velocity: Vec3,
    pub float_height: f32,
    pub cling_distance: f32,
}

impl TnuaBasis for Movement {
    type State = ();

    fn apply(
        &self,
        _state: &mut Self::State,
        _ctx: crate::basis_trait::TnuaBasisContext,
        _motor: &mut crate::TnuaMotor,
    ) {
        info!("MOVING! {:?}", self.desired_velocity);
    }

    fn proximity_sensor_cast_range(&self) -> f32 {
        // TODO - also need to consider float_height_offset? Or maybe it should be united,
        // or converted into an action?
        self.float_height + self.cling_distance
    }
}
