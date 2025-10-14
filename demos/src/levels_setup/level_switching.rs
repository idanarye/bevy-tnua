use std::time::Duration;

use bevy::{
    ecs::{query::QueryData, system::SystemId},
    prelude::*,
};

#[derive(Component)]
pub struct LevelObject;

#[derive(Component)]
pub struct IsPlayer;

#[derive(Component)]
pub struct PositionPlayer {
    position: Vec3,
    ttl: Timer,
}

impl From<Vec3> for PositionPlayer {
    fn from(position: Vec3) -> Self {
        Self {
            position,
            ttl: Timer::new(Duration::from_millis(500), TimerMode::Once),
        }
    }
}

pub struct LevelSwitchingPlugin {
    #[allow(clippy::type_complexity)]
    levels: Vec<(String, Box<dyn Send + Sync + Fn(&mut World) -> SystemId>)>,
    default_level: Option<String>,
}

impl LevelSwitchingPlugin {
    pub fn new(default_level: Option<impl ToString>) -> Self {
        Self {
            levels: Default::default(),
            default_level: default_level.map(|name| name.to_string()),
        }
    }

    pub fn add<M>(
        &mut self,
        name: impl ToString,
        system: impl 'static + Send + Sync + Clone + IntoSystem<(), (), M>,
    ) {
        self.levels.push((
            name.to_string(),
            Box::new(move |world| world.register_system(system.clone())),
        ));
    }

    pub fn with<M>(
        mut self,
        name: impl ToString,
        system: impl 'static + Send + Sync + Clone + IntoSystem<(), (), M>,
    ) -> Self {
        self.add(name, system);
        self
    }

    pub fn without(mut self, name: &str) -> Self {
        self.levels.retain(|(level_name, _)| level_name != name);
        self
    }

    pub fn with_levels(mut self, levels_adder: impl FnOnce(&mut Self)) -> Self {
        levels_adder(&mut self);
        self
    }
}

impl Plugin for LevelSwitchingPlugin {
    fn build(&self, app: &mut App) {
        let levels = self
            .levels
            .iter()
            .map(|(name, system_registrar)| SwitchableLevel {
                name: name.clone(),
                level: system_registrar(app.world_mut()),
            })
            .collect::<Vec<_>>();
        let level_index = if let Some(default_level) = self.default_level.as_ref() {
            levels
                .iter()
                .position(|level| level.name() == default_level)
                .unwrap_or_else(|| panic!("Level {default_level:?} not found"))
        } else {
            0
        };
        app.insert_resource(SwitchableLevels { current: 0, levels });
        app.add_message::<SwitchToLevel>();
        app.add_systems(Update, (handle_level_switching, handle_player_positioning));
        app.add_systems(Startup, move |mut writer: MessageWriter<SwitchToLevel>| {
            writer.write(SwitchToLevel(level_index));
        });
    }
}

#[derive(Clone)]
pub struct SwitchableLevel {
    name: String,
    level: SystemId,
}

impl SwitchableLevel {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Message)]
pub struct SwitchToLevel(pub usize);

#[derive(Resource)]
pub struct SwitchableLevels {
    pub current: usize,
    pub levels: Vec<SwitchableLevel>,
}

impl SwitchableLevels {
    pub fn current(&self) -> &SwitchableLevel {
        &self.levels[self.current]
    }
    pub fn iter(&self) -> impl Iterator<Item = &SwitchableLevel> {
        self.levels.iter()
    }
}

#[allow(clippy::type_complexity)]
fn handle_level_switching(
    mut reader: MessageReader<SwitchToLevel>,
    mut switchable_levels: ResMut<SwitchableLevels>,
    query: Query<Entity, Or<(With<LevelObject>, With<PositionPlayer>)>>,
    mut commands: Commands,
) {
    let Some(SwitchToLevel(new_level_index)) = reader.read().last() else {
        return;
    };
    switchable_levels.current = *new_level_index;
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    commands.run_system(switchable_levels.current().level);
}

#[derive(QueryData)]
#[query_data(mutable)]
struct PlayerQueryForPositioning {
    transform: &'static mut Transform,

    #[cfg(feature = "rapier2d")]
    rapier2d_velocity: Option<&'static mut bevy_rapier2d::prelude::Velocity>,

    #[cfg(feature = "rapier3d")]
    rapier3d_velocity: Option<&'static mut bevy_rapier3d::prelude::Velocity>,

    #[cfg(feature = "avian2d")]
    avian2d_linear_velocity: Option<&'static mut avian2d::prelude::LinearVelocity>,
    #[cfg(feature = "avian2d")]
    avian2d_angular_velocity: Option<&'static mut avian2d::prelude::AngularVelocity>,

    #[cfg(feature = "avian3d")]
    avian3d_linear_velocity: Option<&'static mut avian3d::prelude::LinearVelocity>,
    #[cfg(feature = "avian3d")]
    avian3d_angular_velocity: Option<&'static mut avian3d::prelude::AngularVelocity>,
}

fn handle_player_positioning(
    time: Res<Time>,
    mut players_query: Query<PlayerQueryForPositioning, With<IsPlayer>>,
    mut positioning_query: Query<(Entity, &mut PositionPlayer)>,
    mut commands: Commands,
) {
    let Some((positioner_entity, mut position_player)) = positioning_query.iter_mut().next() else {
        return;
    };
    for mut player in players_query.iter_mut() {
        player.transform.translation = position_player.position;

        #[cfg(feature = "rapier2d")]
        if let Some(velocity) = player.rapier2d_velocity.as_mut() {
            velocity.linvel = Default::default();
            velocity.angvel = Default::default();
        }

        #[cfg(feature = "rapier3d")]
        if let Some(velocity) = player.rapier3d_velocity.as_mut() {
            velocity.linvel = Default::default();
            velocity.angvel = Default::default();
        }

        #[cfg(feature = "avian2d")]
        if let Some(velocity) = player.avian2d_linear_velocity.as_mut() {
            velocity.0 = Default::default();
        }
        #[cfg(feature = "avian2d")]
        if let Some(velocity) = player.avian2d_angular_velocity.as_mut() {
            velocity.0 = Default::default();
        }

        #[cfg(feature = "avian3d")]
        if let Some(velocity) = player.avian3d_linear_velocity.as_mut() {
            velocity.0 = Default::default();
        }
        #[cfg(feature = "avian3d")]
        if let Some(velocity) = player.avian3d_angular_velocity.as_mut() {
            velocity.0 = Default::default();
        }
    }
    if position_player.ttl.tick(time.delta()).is_finished() {
        commands.entity(positioner_entity).despawn();
    }
}
