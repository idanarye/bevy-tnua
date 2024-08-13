use bevy::{ecs::query::QueryData, prelude::*};
use bevy_tnua::math::Vector3;

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

#[derive(QueryData)]
#[query_data(mutable)]
struct VelocityQuery {
    #[cfg(feature = "rapier2d")]
    rapier2d_velocity: Option<&'static mut bevy_rapier2d::prelude::Velocity>,

    #[cfg(feature = "rapier3d")]
    rapier3d_velocity: Option<&'static mut bevy_rapier3d::prelude::Velocity>,

    #[cfg(feature = "avian2d")]
    avian2d_linear_velocity: Option<&'static mut avian2d::prelude::LinearVelocity>,

    #[cfg(feature = "avian3d")]
    avian3d_linear_velocity: Option<&'static mut avian3d::prelude::LinearVelocity>,
}

impl VelocityQueryItem<'_> {
    fn apply_impulse(&mut self, impulse: Vector3) {
        #[cfg(feature = "rapier2d")]
        if let Some(velocity) = self.rapier2d_velocity.as_mut() {
            velocity.linvel += impulse.truncate();
        }

        #[cfg(feature = "rapier3d")]
        if let Some(velocity) = self.rapier3d_velocity.as_mut() {
            velocity.linvel += impulse;
        }

        #[cfg(feature = "avian2d")]
        if let Some(velocity) = self.avian2d_linear_velocity.as_mut() {
            velocity.0 += impulse.truncate();
        }

        #[cfg(feature = "avian3d")]
        if let Some(velocity) = self.avian3d_linear_velocity.as_mut() {
            velocity.0 += impulse;
        }
    }
}

fn apply_push_effect(
    mut query: Query<(Entity, &PushEffect, VelocityQuery)>,
    mut commands: Commands,
) {
    for (entity, push_effect, mut velocity) in query.iter_mut() {
        match push_effect {
            PushEffect::Impulse(impulse) => {
                velocity.apply_impulse(*impulse);
                commands.entity(entity).remove::<PushEffect>();
            }
        }
    }
}
