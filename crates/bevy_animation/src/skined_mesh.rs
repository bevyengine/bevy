// TODO: Later merge with bevy_animation
use bevy_asset::{Assets, Handle};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::MapEntities;
use bevy_math::Mat4;
use bevy_pbr::prelude::*;
use bevy_property::Properties;
use bevy_render::mesh::{shape, Indices, Mesh, VertexAttributeValues};
use bevy_render::pipeline::PrimitiveTopology;
use bevy_transform::prelude::*;
use bevy_type_registry::TypeUuid;
use smallvec::SmallVec;

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
pub struct MeshSkinBinder {
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

impl MeshSkinBinder {
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
impl FromResources for MeshSkinBinder {
    fn from_resources(_resources: &bevy_ecs::Resources) -> Self {
        MeshSkinBinder::with_skeleton(Entity::new(u32::MAX))
    }
}

impl MapEntities for MeshSkinBinder {
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
pub(crate) fn mesh_skinner_startup(
    mesh_skin_assets: Res<Assets<MeshSkin>>,
    mut skinners_query: Query<(&Handle<MeshSkin>, &mut MeshSkinBinder, Option<&Children>)>,
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
                        continue;
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

// TODO: MeshSkinBinder system
// fn mesh_skin_binder_update() {
//     // TODO: have to send the matrices into each entity
// }

///////////////////////////////////////////////////////////////////////////////

// TODO: Should I implement a SkeletalAnimator for better performance for mesh animations
// // Similar to Animator but only works for transforms
// pub struct SkeletalAnimator {}

// pub(crate) fn skeletal_animator_update(
//     time: Res<Time>,
//     // keyboard: Res<Input<KeyCode>>,
//     clips: Res<Assets<Clip>>,
//     mut animators_query: Query<(Mut<Animator>,)>,
//     mut transforms_query: Query<(Mut<Transform>,)>,
// ) {
//     // let delta_time = if keyboard.just_pressed(KeyCode::Right) {
//     //     1.0 / 60.0
//     // } else {
//     //     0.0
//     // };

//     let delta_time = time.delta_seconds;

//     for (mut debugger,) in animators_query.iter_mut() {
//         let debugger = debugger.deref_mut();

//         if !debugger.builded {
//             return;
//         }

//         // Time scales by component
//         let delta_time = delta_time * debugger.time_scale;

//         // Ensure capacity for cached keyframe index vec
//         let nodes_count = debugger.hierarchy.len();
//         if debugger.current.keyframe.len() != nodes_count {
//             debugger.current.keyframe.clear();
//             debugger
//                 .current
//                 .keyframe
//                 .resize_with(nodes_count, || Default::default());
//         }

//         if let Some(clip) = clips.get(&debugger.clip) {
//             // Update time
//             let mut time = debugger.current.time + delta_time;

//             // Warp mode
//             if clip.warp {
//                 // Warp Around
//                 if time > clip.length {
//                     time = (time / clip.length).fract() * clip.length;
//                     // Reset all keyframes cached indexes
//                     debugger
//                         .current
//                         .keyframe
//                         .iter_mut()
//                         .for_each(|x| *x = Default::default())
//                 }
//             } else {
//                 // Hold
//                 time = time.min(clip.length);
//             }

//             // Advance state time
//             debugger.current.time = time;

//             for (i, entity) in debugger.hierarchy.iter().enumerate() {
//                 if let Ok((mut transform,)) = transforms_query.get_mut(*entity) {
//                     // Get cached keyframe info
//                     let keyframe = &mut debugger.current.keyframe[i];

//                     // Sample animation
//                     let (k, v) = clip.bones[i].position.sample(keyframe.position, time);
//                     keyframe.position = k;
//                     transform.translation = v;

//                     let (k, v) = clip.bones[i].rotation.sample(keyframe.rotation, time);
//                     keyframe.rotation = k;
//                     transform.rotation = v;

//                     let (k, v) = clip.bones[i].scale.sample(keyframe.scale, time);
//                     keyframe.scale = k;
//                     transform.scale = v;
//                 }
//             }
//         }
//     }
// }

///////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, Properties)]
pub struct MeshSkinnerDebuger {
    //pub enabled: bool,
    #[property(ignore)]
    started: bool,
    #[property(ignore)]
    mesh: Option<Handle<Mesh>>,
    #[property(ignore)]
    entity: Option<Entity>,
}

pub(crate) fn mesh_skinner_debugger_update(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    skins: Res<Assets<MeshSkin>>,
    mut debugger_query: Query<(&Handle<MeshSkin>, &MeshSkinBinder, &mut MeshSkinnerDebuger)>,
    bones_query: Query<(&GlobalTransform,)>,
) {
    for (skin_handle, skinner, mut debugger) in debugger_query.iter_mut() {
        if skinner.skin.as_ref() != Some(skin_handle) {
            continue;
        }

        let debugger = &mut *debugger;

        // if !debugger.enabled {
        //     continue;
        // }

        if let Some(skin) = skins.get(skin_handle) {
            if debugger.mesh.is_none() {
                let mesh = Mesh::new(PrimitiveTopology::LineList);
                debugger.mesh = Some(meshes.add(mesh));
            }

            if !debugger.started {
                let bone_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.02 }));
                for bone in skinner.bones.iter() {
                    if let Some(entity) = bone {
                        commands
                            .spawn(PbrBundle {
                                mesh: bone_mesh.clone(),
                                ..Default::default()
                            })
                            .with(Parent(*entity));
                    }
                }

                debugger.started = true;
            }

            let mesh_handle = debugger.mesh.as_ref().unwrap();
            let mesh = meshes.get_mut(mesh_handle).unwrap();

            if debugger.entity.is_none() {
                debugger.entity = commands
                    .spawn(PbrBundle {
                        mesh: mesh_handle.clone(),
                        ..Default::default()
                    })
                    .current_entity()
            }

            let positions = skinner
                .bones
                .iter()
                .map(|bone| {
                    if let Some(entity) = *bone {
                        if let Ok((global_transform,)) = bones_query.get(entity) {
                            global_transform.translation
                        } else {
                            Default::default()
                        }
                    } else {
                        Default::default()
                    }
                })
                .collect::<Vec<_>>();

            // TODO: Improve mesh generation with a 3 sided pyramid
            // TODO: How reuse mesh buffers?

            let mut indices = vec![];
            let mut vertices = vec![];

            for (i, parent) in skin.bones_parents.iter().enumerate() {
                if let Some(parent) = *parent {
                    indices.push(vertices.len() as u32);
                    vertices.push(positions[i].into());
                    indices.push(vertices.len() as u32);
                    vertices.push(positions[parent].into());
                }
            }

            // TODO: Change shader to not require normals and uv attributes

            let normals = Some([0f32; 3])
                .iter()
                .copied()
                .cycle()
                .take(vertices.len())
                .collect::<Vec<_>>();

            let uvs = Some([0f32; 2])
                .iter()
                .copied()
                .cycle()
                .take(vertices.len())
                .collect::<Vec<_>>();

            mesh.set_attribute(
                Mesh::ATTRIBUTE_POSITION,
                VertexAttributeValues::Float3(vertices),
            );
            mesh.set_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                VertexAttributeValues::Float3(normals),
            );
            mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float2(uvs));
            mesh.set_indices(Some(Indices::U32(indices)));
        }
    }
}
