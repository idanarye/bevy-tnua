use bevy::prelude::*;
use bevy_tnua::math::AdjustPrecision;
use bevy_tnua::TnuaGravity;
use ordered_float::OrderedFloat;

use crate::levels_setup::IsPlayer;

pub struct CenterOfGravityPlugin;

impl Plugin for CenterOfGravityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (add_or_remove_components, set_gravity));
    }
}

#[derive(Component)]
pub struct IsCenterOfGraity;

fn add_or_remove_components(
    players_with: Query<Entity, With<TnuaGravity>>,
    players_without: Query<Entity, (With<IsPlayer>, Without<TnuaGravity>)>,
    centers_of_graity: Query<(), With<IsCenterOfGraity>>,
    mut commands: Commands,
) {
    if centers_of_graity.is_empty() {
        for entity in players_with.iter() {
            commands.entity(entity).remove::<TnuaGravity>();
        }
    } else {
        for entity in players_without.iter() {
            let mut cmd = commands.entity(entity);
            cmd.insert(TnuaGravity(Default::default()));
            #[cfg(feature = "avian3d")]
            cmd.insert(avian3d::prelude::LockedAxes::new());
            #[cfg(feature = "rapier3d")]
            cmd.insert(bevy_rapier3d::prelude::LockedAxes::empty());
        }
    }
}

fn set_gravity(
    mut characters: Query<(&mut TnuaGravity, &GlobalTransform)>,
    centers: Query<&GlobalTransform, With<IsCenterOfGraity>>,
) {
    for (mut character_gravity, character_transform) in characters.iter_mut() {
        let character_position = character_transform.translation();
        let Some(center_of_gravity) = centers
            .iter()
            .map(|center_transform| center_transform.translation())
            .min_by_key(|center_position| {
                OrderedFloat(center_position.distance_squared(character_position))
            })
        else {
            continue;
        };
        character_gravity.0 = (center_of_gravity - character_position)
            .adjust_precision()
            .normalize_or_zero()
            * 9.81;
    }
}
