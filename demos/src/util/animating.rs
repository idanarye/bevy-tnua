use bevy::gltf::Gltf;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

#[derive(Component)]
pub struct AnimationsHandler {
    pub player_entity: Entity,
    pub animations: HashMap<String, AnimationNodeIndex>,
}

#[derive(Component)]
pub struct GltfSceneHandler {
    pub names_from: Handle<Gltf>,
}

pub fn animation_patcher_system(
    animation_players_query: Query<Entity, Added<AnimationPlayer>>,
    parents_query: Query<&ChildOf>,
    scene_handlers_query: Query<&GltfSceneHandler>,
    gltf_assets: Res<Assets<Gltf>>,
    mut animation_graphs_assets: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    for player_entity in animation_players_query.iter() {
        let mut entity = player_entity;
        loop {
            if let Ok(GltfSceneHandler { names_from }) = scene_handlers_query.get(entity) {
                let gltf = gltf_assets.get(names_from).unwrap();
                let mut graph = AnimationGraph::new();
                let root_node = graph.root;
                let mut animations = HashMap::<String, AnimationNodeIndex>::new();

                for (name, clip) in gltf.named_animations.iter() {
                    let node_index = graph.add_clip(clip.clone(), 1.0, root_node);
                    animations.insert(name.to_string(), node_index);
                }

                let mut cmd = commands.entity(entity);
                cmd.remove::<GltfSceneHandler>();
                cmd.insert(AnimationsHandler {
                    player_entity,
                    animations,
                });
                commands
                    .entity(player_entity)
                    .insert(AnimationGraphHandle(animation_graphs_assets.add(graph)));
                break;
            }
            entity = if let Ok(child_of) = parents_query.get(entity) {
                child_of.parent()
            } else {
                break;
            };
        }
    }
}
