use bevy::prelude::*;

pub struct PushEffectPlugin;

impl Plugin for PushEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_push_effect);
    }
}

fn apply_push_effect() {}
