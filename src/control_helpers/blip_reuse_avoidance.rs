use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::obstacle_radar::TnuaObstacleRadar;

use crate::controller::TnuaActionFlowStatus;
use crate::prelude::TnuaController;

#[derive(Default, Component)]
pub struct TnuaBlipReuseAvoidance {
    current_entity: Option<Entity>,
    entities_to_avoid: HashMap<Entity, &'static str>,
}

impl TnuaBlipReuseAvoidance {
    pub fn update(&mut self, controller: &TnuaController, radar: &TnuaObstacleRadar) {
        let current_entity = controller
            .dynamic_action()
            .and_then(|action| action.target_entity());

        if current_entity != self.current_entity {
            if let Some(old_entity) = self.current_entity.as_ref() {
                if let TnuaActionFlowStatus::ActionEnded(action_name)
                | TnuaActionFlowStatus::Cancelled {
                    old: action_name,
                    new: _,
                } = controller.action_flow_status()
                {
                    self.entities_to_avoid.insert(*old_entity, action_name);
                }
            }
        }

        self.entities_to_avoid
            .retain(|entity, _| radar.has_blip(*entity));

        self.current_entity = current_entity;
    }

    pub fn should_avoid(&self, entity: Entity) -> bool {
        self.entities_to_avoid.contains_key(&entity)
    }
}
