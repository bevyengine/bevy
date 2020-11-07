// TODO: Later merge with bevy_animation
use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::MapEntities;
use bevy_math::Mat4;
use bevy_property::Properties;
use bevy_render::mesh::Mesh;
use bevy_transform::prelude::*;
use bevy_type_registry::{RegisterType, TypeUuid};
use smallvec::SmallVec;
// use serde::{Deserialize, Serialize};

/// Skin asset used by the mesh skinning process
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "129d54f5-4ee7-456f-9340-32e71469cdaf"]
pub struct MeshSkin {
    pub inverse_bind_matrices: Vec<Mat4>,
    pub bones_names: Vec<String>,
    /// Each bone as an entry that specifies their parent bone, maybe used to reconstruct
    /// the bone hierarchy when missing
    pub bones_parents: Vec<Option<usize>>,
}

impl MeshSkin {
    #[inline(always)]
    pub fn bone_count(&self) -> usize {
        self.bones_names.len()
    }
}

/// Component that skins some mesh.
/// Requires a `Handle<MeshSkin>` attached to same entity as the component
#[derive(Properties)]
pub struct MeshSkinner {
    /// Keeps track of what `MeshSkin` this component is configured to,
    /// extra work is required to keep bones in order.
    ///
    /// It's expected to the skin not to change very often or at all
    #[property(ignore)]
    skin: Option<Handle<MeshSkin>>,
    /// Skeleton root entity
    pub skeleton: Entity,
    /// Maps each bone to an entity, order matters and must match the
    /// `Handle<MeshSkin>` bone order, this will simplify the lookup of
    /// the bind matrix for each bone
    pub bones: SmallVec<[Option<Entity>; 16]>, // ! FIXME: Property can't handle Vec<Entity>
    /// List of sub-meshes (gltf primitives) that uses this mesh skinner
    pub meshes: SmallVec<[Entity; 8]>, // ! FIXME: Property can't handle Vec<Entity>
}

impl MeshSkinner {
    pub fn with_skeleton(skeleton: Entity) -> Self {
        Self {
            skin: None,
            skeleton,
            bones: Default::default(),
            meshes: Default::default(),
        }
    }

    // TODO: Provide a safe interface to `set_bone_by_name` in the MeshSkinner
}

// TODO: Same problem of Parent component
impl FromResources for MeshSkinner {
    fn from_resources(_resources: &bevy_ecs::Resources) -> Self {
        MeshSkinner::with_skeleton(Entity::new(u32::MAX))
    }
}

impl MapEntities for MeshSkinner {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::EntityMap,
    ) -> Result<(), bevy_ecs::MapEntitiesError> {
        for bone in &mut self.bones {
            if let Some(bone_entity) = bone {
                *bone_entity = entity_map.get(*bone_entity)?;
            }
        }
        self.skeleton = entity_map.get(self.skeleton)?;
        Ok(())
    }
}

// NOTE: This system is provided for a user convenience, once the root bone is assigned this system
// will find the rest of the skeleton hierarchy.
fn mesh_skinner_startup(
    mesh_skin_assets: Res<Assets<MeshSkin>>,
    mut skinners_query: Query<(&Handle<MeshSkin>, &mut MeshSkinner, Option<&Children>)>,
    meshes_query: Query<(&Handle<Mesh>,)>,
    bones_query: Query<(Entity, &Name, Option<&Children>)>,
) {
    for (mesh_skin, mut mesh_skinner, children) in skinners_query.iter_mut() {
        // Already assigned
        if Some(mesh_skin) == mesh_skinner.skin.as_ref() {
            continue;
        }

        // Lookup for all non assigned sub-meshes
        if let Some(children) = children {
            for mesh in children
                .iter()
                .filter_map(|child| meshes_query.get(*child).map_or(None, |_| Some(child)))
                .copied()
            {
                if mesh_skinner.meshes.contains(&mesh) {
                    continue;
                }

                mesh_skinner.meshes.push(mesh);
            }
        }

        if let Some(skin) = mesh_skin_assets.get(mesh_skin) {
            // Ensure bone capacity
            mesh_skinner.bones.resize(skin.bone_count(), None);

            let mut root = true;
            let mut stack = vec![mesh_skinner.skeleton];
            while let Some(entity) = stack.pop() {
                // Lookup bones in the hierarchy
                if let Ok((bone_entity, name, children)) = bones_query.get(entity) {
                    if root {
                        children.map(|c| stack.extend(c.iter()));
                        root = false;
                    }

                    if let Some((bone_index, _)) = skin
                        .bones_names
                        .iter()
                        .enumerate()
                        .find(|(_, n)| name.eq(*n))
                    {
                        mesh_skinner.bones[bone_index] = Some(bone_entity);
                        children.map(|c| stack.extend(c.iter()));
                    }
                }
            }

            mesh_skinner.skin = Some(mesh_skin.clone());
        }
    }
}

// TODO: MeshSkinner system
// fn mesh_skinner_update() {
//     // TODO: have to send the matrices into each entity
// }

#[derive(Default)]
pub struct MeshSkinPlugin;

impl Plugin for MeshSkinPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<MeshSkin>()
            .register_component_with::<MeshSkinner>(|reg| reg.map_entities())
            .add_system(mesh_skinner_startup.system());
        // .register_component::<BoneName>();
    }
}
