#[cfg(feature = "bevy_animation")]
use bevy_animation::AnimationPlayer;
use bevy_asset::{Handle, LoadContext};
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    world::World,
};
use bevy_hierarchy::BuildChildren;
use bevy_render::{mesh::skinning::SkinnedMesh, view::Visibility};
use bevy_scene::Scene;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
#[cfg(feature = "bevy_animation")]
use bevy_utils::HashSet;

use crate::{GltfAssetLabel, GltfError, GltfLoaderSettings, GltfSceneExtras};

use super::{ExtrasExt, NodeExt};

/// [`Scene`](gltf::Scene) extension
pub trait SceneExt {
    #[allow(clippy::result_large_err)]
    fn load_scene(
        &self,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        gltf: &gltf::Gltf,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
        active_camera_found: &mut bool,
    ) -> Result<Handle<Scene>, GltfError>;

    /// Create a [`GltfAssetLabel`] for the [`Scene`](gltf::Scene).
    fn to_label(&self) -> GltfAssetLabel;
}

impl SceneExt for gltf::Scene<'_> {
    fn load_scene(
        &self,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        gltf: &gltf::Gltf,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
        active_camera_found: &mut bool,
    ) -> Result<Handle<Scene>, GltfError> {
        let mut world = World::default();
        let mut node_index_to_entity_map = HashMap::new();
        let mut entity_to_skin_index_map = EntityHashMap::default();
        let mut scene_load_context = load_context.begin_labeled_asset();

        let world_root_id = world
            .spawn((Transform::default(), Visibility::default()))
            .id();

        let nodes = self
            .nodes()
            .map(|node| {
                node.load_scene_node(
                    &mut world,
                    load_context,
                    &mut scene_load_context,
                    settings,
                    &mut node_index_to_entity_map,
                    &mut entity_to_skin_index_map,
                    active_camera_found,
                    &Transform::default(),
                    #[cfg(feature = "bevy_animation")]
                    animation_roots,
                    #[cfg(feature = "bevy_animation")]
                    None,
                    &gltf.document,
                )
            })
            .collect::<Result<Vec<Entity>, GltfError>>()?;

        world.entity_mut(world_root_id).add_children(&nodes);

        if let Some(extras) = self.extras().get() {
            world
                .entity_mut(world_root_id)
                .insert(GltfSceneExtras::from(extras));
        }

        #[cfg(feature = "bevy_animation")]
        {
            // for each node root in a scene, check if it's the root of an animation
            // if it is, add the AnimationPlayer component
            for node in self.nodes() {
                if animation_roots.contains(&node.index()) {
                    world
                        .entity_mut(*node_index_to_entity_map.get(&node.index()).unwrap())
                        .insert(AnimationPlayer::default());
                }
            }
        }

        for (&entity, &skin_index) in &entity_to_skin_index_map {
            let mut entity = world.entity_mut(entity);
            let skin = gltf.skins().nth(skin_index).unwrap();
            let joint_entities: Vec<_> = skin
                .joints()
                .map(|node| node_index_to_entity_map[&node.index()])
                .collect();

            entity.insert(SkinnedMesh {
                inverse_bindposes: scene_load_context
                    .get_label_handle(GltfAssetLabel::InverseBindMatrices(skin_index).to_string()),
                joints: joint_entities,
            });
        }
        let loaded_scene = scene_load_context.finish(Scene::new(world), None);
        let scene_handle =
            load_context.add_loaded_labeled_asset(self.to_label().to_string(), loaded_scene);

        Ok(scene_handle)
    }

    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Scene(self.index())
    }
}
