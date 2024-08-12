use bevy::{ecs::system::EntityCommands, prelude::*};

use crate::levels_setup::LevelObject;

#[derive(Component)]
pub struct Cannon {
    pub timer: Timer,
    pub cmd: Box<dyn Send + Sync + Fn(&mut EntityCommands)>,
}

pub struct CannonPlugin;

impl Plugin for CannonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, shoot);
    }
}

fn shoot(
    time: Res<Time>,
    mut query: Query<(&mut Cannon, &GlobalTransform, Option<&Name>)>,
    mut commands: Commands,
) {
    for (mut cannon, cannon_transform, cannon_name) in query.iter_mut() {
        if cannon.timer.tick(time.delta()).just_finished() {
            let mut cmd = commands.spawn(LevelObject);
            if let Some(cannon_name) = cannon_name.as_ref() {
                cmd.insert(Name::new(format!("{cannon_name} projectile")));
            }
            (cannon.cmd)(&mut cmd);
            cmd.insert(TransformBundle::from_transform(
                Transform::from_translation(cannon_transform.translation()),
            ));
        }
    }
}
