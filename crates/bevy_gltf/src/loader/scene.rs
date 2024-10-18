#[cfg(feature = "bevy_animation")]
use bevy_animation::prelude::*;
use bevy_asset::{Handle, LoadContext};
use bevy_ecs::{entity::EntityHashMap, world::World};
use bevy_hierarchy::BuildChildren;
use bevy_render::{
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    view::Visibility,
};
use bevy_scene::Scene;
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
#[cfg(feature = "bevy_animation")]
use bevy_utils::HashSet;

use crate::{GltfAssetLabel, GltfSceneExtras};

use super::{GltfError, GltfLoaderSettings};

#[allow(clippy::result_large_err)]
pub fn load_scenes(
    load_context: &mut LoadContext,
    settings: &GltfLoaderSettings,
    gltf: &gltf::Gltf,
    #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    skinned_mesh_inverse_bindposes: &[Handle<SkinnedMeshInverseBindposes>],
) -> Result<(Vec<Handle<Scene>>, HashMap<Box<str>, Handle<Scene>>), GltfError> {
    let mut scenes = vec![];
    let mut named_scenes = HashMap::default();
    let mut active_camera_found = false;
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        let mut node_index_to_entity_map = HashMap::new();
        let mut entity_to_skin_index_map = EntityHashMap::default();
        let mut scene_load_context = load_context.begin_labeled_asset();

        let world_root_id = world
            .spawn((Transform::default(), Visibility::default()))
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result = super::node::load_node(
                        &node,
                        parent,
                        load_context,
                        &mut scene_load_context,
                        settings,
                        &mut node_index_to_entity_map,
                        &mut entity_to_skin_index_map,
                        &mut active_camera_found,
                        &Transform::default(),
                        #[cfg(feature = "bevy_animation")]
                        animation_roots,
                        #[cfg(feature = "bevy_animation")]
                        None,
                        &gltf.document,
                    );
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            })
            .id();

        if let Some(extras) = scene.extras().as_ref() {
            world.entity_mut(world_root_id).insert(GltfSceneExtras {
                value: extras.get().to_string(),
            });
        }

        if let Some(Err(err)) = err {
            return Err(err);
        }

        #[cfg(feature = "bevy_animation")]
        {
            // for each node root in a scene, check if it's the root of an animation
            // if it is, add the AnimationPlayer component
            for node in scene.nodes() {
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
                inverse_bindposes: skinned_mesh_inverse_bindposes[skin_index].clone(),
                joints: joint_entities,
            });
        }
        let loaded_scene = scene_load_context.finish(Scene::new(world), None);
        let scene_handle = load_context.add_loaded_labeled_asset(scene_label(&scene), loaded_scene);

        if let Some(name) = scene.name() {
            named_scenes.insert(name.into(), scene_handle.clone());
        }
        scenes.push(scene_handle);
    }
    Ok((scenes, named_scenes))
}

/// Returns the label for the `scene`.
fn scene_label(scene: &gltf::Scene) -> String {
    GltfAssetLabel::Scene(scene.index()).to_string()
}
