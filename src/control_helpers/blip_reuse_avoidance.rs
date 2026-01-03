use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_tnua_physics_integration_layer::obstacle_radar::TnuaObstacleRadar;

use crate::TnuaScheme;
use crate::controller::TnuaActionFlowStatus;
use crate::prelude::TnuaController;

/// Helper for keeping track on entities the character was just interacting with, so that it won't
/// immediately interact with again after the action is finished.
///
/// For example - this can be use to avoid climbing again on a ladder immediately after dropping
/// from it.
#[derive(Component)]
pub struct TnuaBlipReuseAvoidance<S: TnuaScheme> {
    current_entity: Option<Entity>,
    entities_to_avoid: HashMap<Entity, S::ActionDiscriminant>,
}

impl<S: TnuaScheme> Default for TnuaBlipReuseAvoidance<S> {
    fn default() -> Self {
        Self {
            current_entity: None,
            entities_to_avoid: Default::default(),
        }
    }
}

/// Must be implemented by control schemes that want to use [`TnuaBlipReuseAvoidance`] or
pub trait TnuaHasTargetEntity: TnuaScheme {
    /// The entity used by the given action.
    ///
    /// Note that entities are not part of the actions themselves - they are part of the payloads.
    /// It's up to user code to define them in the control scheme for the relevant actions and to
    /// pass then when feeding these actions.
    fn target_entity(action_state: &Self::ActionState) -> Option<Entity>;
}

impl<S> TnuaBlipReuseAvoidance<S>
where
    S: TnuaScheme + TnuaHasTargetEntity,
{
    /// Call this every frame.
    pub fn update(&mut self, controller: &TnuaController<S>, radar: &TnuaObstacleRadar) {
        let current_entity = controller
            .current_action
            .as_ref()
            .and_then(S::target_entity);

        if current_entity != self.current_entity
            && let Some(old_entity) = self.current_entity.as_ref()
            && let TnuaActionFlowStatus::ActionEnded(action_discriminant)
            | TnuaActionFlowStatus::Cancelled {
                old: action_discriminant,
                new: _,
            } = controller.action_flow_status()
        {
            self.entities_to_avoid
                .insert(*old_entity, *action_discriminant);
        }

        self.entities_to_avoid
            .retain(|entity, _| radar.has_blip(*entity));

        self.current_entity = current_entity;
    }

    /// Returns true the entity was already interacted with and the character did not move away
    /// from it yet.
    pub fn should_avoid(&self, entity: Entity) -> bool {
        self.entities_to_avoid.contains_key(&entity)
    }
}
