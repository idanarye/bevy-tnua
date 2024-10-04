use bevy::prelude::*;
use bevy_tnua::{builtins::TnuaBuiltinKnockback, math::Vector3, prelude::TnuaController};

use crate::character_control_systems::platformer_control_systems::CharacterMotionConfigForPlatformerDemo;

pub struct PushEffectPlugin;

impl Plugin for PushEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_push_effect);
    }
}

#[derive(Component)]
pub enum PushEffect {
    Impulse(Vector3),
}

fn apply_push_effect(
    mut query: Query<(
        Entity,
        &PushEffect,
        &mut TnuaController,
        &CharacterMotionConfigForPlatformerDemo,
    )>,
    mut commands: Commands,
) {
    for (entity, push_effect, mut controller, config) in query.iter_mut() {
        match push_effect {
            PushEffect::Impulse(impulse) => {
                controller.action(TnuaBuiltinKnockback {
                    shove: *impulse,
                    force_forward: None,
                    ..config.knockback
                });
                commands.entity(entity).remove::<PushEffect>();
            }
        }
    }
}
