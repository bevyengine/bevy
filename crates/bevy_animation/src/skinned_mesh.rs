// TODO: Later merge with bevy_animation
use crate::hierarchy::Hierarchy;
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::MapEntities;
use bevy_math::Mat4;
use bevy_pbr::prelude::*;
use bevy_reflect::{Reflect, ReflectComponent, ReflectMapEntities, TypeUuid};
use bevy_render::mesh::{shape, Indices, Mesh, VertexAttributeValues};
use bevy_render::pipeline::{PipelineDescriptor, PrimitiveTopology, RenderPipelines};
use bevy_render::render_graph::{base, AssetRenderResourcesNode, RenderGraph};
use bevy_render::renderer::RenderResources;
use bevy_render::shader::{Shader, ShaderStage};
use bevy_transform::prelude::*;
use smallvec::SmallVec;

// TODO: We could use a computer shader to skin the mesh with morph targets (performance improvement)
// TODO: Morph targets

// NOTE: generated using python `import secrets; secrets.token_hex(8)`
pub const FORWARD_SKINNED_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0xedf5a66b71d07478u64);

/// Skin asset used by the skinning process
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "129d54f5-4ee7-456f-9340-32e71469cdaf"]
pub struct SkinAsset {
    /// Inverse bind matrices in root node space
    pub inverse_bind_matrices: Vec<Mat4>,
    /// Skin joint hierarchy, plus one extra the root node as the first entry.
    pub hierarchy: Hierarchy,
}

#[derive(Default, Debug, Clone, TypeUuid, RenderResources)]
#[uuid = "e25de07b-f813-4c6b-ad06-b160c3e924b2"]
pub struct SkinInstance {
    // TODO: Use 4x3 matrices instead, the last row will be always vec4(0, 0, 0, 1)
    // NOTE: Mat4 doesn't impl `Byteable` so we need to use an array here
    #[render_resources(buffer)]
    joint_matrices: Vec<[f32; 16]>,
    // TODO: Define number of bones per vertex
}

/// Component that skins some mesh.
/// Requires a `Handle<Skin>` attached to same entity as the component
#[derive(Reflect)]
#[reflect(Component, MapEntities)]
pub struct SkinComponent {
    /// Keeps track of what `MeshSkin` this component is configured to,
    /// extra work is required to keep bones in order.
    ///
    /// It's expected to the skin not to change very often or at all
    #[reflect(ignore)]
    previous_skin: Option<Handle<SkinAsset>>,

    /// Maps each bone to an entity, order matters and must match the
    /// `Handle<Skin>` bone order, this will simplify the lookup of
    /// the bind matrix for each bone
    #[reflect(ignore)]
    joint_entities: Vec<Option<Entity>>,

    #[reflect(ignore)]
    instance: Option<Handle<SkinInstance>>,

    /// Skeleton root entity
    pub root: Entity,

    /// List of entities with `RenderPipelines` attached to it
    /// that will share this component render resources
    ///
    /// *NOTE* Children of this component doesn't require to be added
    pub renderers: SmallVec<[Entity; 8]>, // ! FIXME: Property can't handle Vec<Entity>
}

impl SkinComponent {
    /// Creates a new SkinBinder component
    pub fn with_root(root: Entity) -> Self {
        Self {
            previous_skin: None,
            joint_entities: Default::default(),
            instance: None,
            root,
            renderers: Default::default(),
        }
    }
}

impl FromResources for SkinComponent {
    fn from_resources(_resources: &bevy_ecs::Resources) -> Self {
        SkinComponent::with_root(Entity::new(u32::MAX))
    }
}

impl MapEntities for SkinComponent {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::EntityMap,
    ) -> Result<(), bevy_ecs::MapEntitiesError> {
        for renderer in &mut self.renderers {
            *renderer = entity_map.get(*renderer)?;
        }
        self.root = entity_map.get(self.root)?;
        Ok(())
    }
}

pub(crate) fn skinning_setup(
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    let __span = tracing::info_span!("skinning_setup");
    let __guard = __span.enter();

    let mut forward_skinned_pipeline = pipelines
        .get(bevy_pbr::render_graph::FORWARD_PIPELINE_HANDLE)
        .expect("missing forward pipeline")
        .clone();

    forward_skinned_pipeline.shader_stages.vertex = shaders.add(Shader::from_glsl(
        ShaderStage::Vertex,
        include_str!("forward_skinned.vert"),
    ));

    pipelines.set_untracked(FORWARD_SKINNED_PIPELINE_HANDLE, forward_skinned_pipeline);

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind SkinInstance resources to our shader
    render_graph.add_system_node(
        "skin_instance",
        AssetRenderResourcesNode::<SkinInstance>::new(false),
    );

    // Add a Render Graph edge connecting our new "skin_instance" node to the main pass node.
    // This ensures "skin_instance" runs before the main pass
    render_graph
        .add_node_edge("skin_instance", base::node::MAIN_PASS)
        .unwrap();

    std::mem::drop(__guard);
}

// ? NOTE: You can check the follow link, for more details
// ? https://github.com/KhronosGroup/glTF-Tutorials/blob/master/gltfTutorial/gltfTutorial_020_Skins.md
pub(crate) fn skinning_update(
    commands: &mut Commands,
    skin_assets: Res<Assets<SkinAsset>>,
    mut skin_instances: ResMut<Assets<SkinInstance>>,
    mut binds_query: Query<(&Handle<SkinAsset>, &mut SkinComponent, &Children)>,
    mut children_query: Query<&Children>,
    mut name_query: Query<(&Parent, &Name)>,
    transforms_query: Query<&GlobalTransform>,
    mut renderers_query: Query<&mut RenderPipelines>,
) {
    let __span = tracing::info_span!("skinning_update");
    let __guard = __span.enter();
    for (skin_asset_handle, mut skin_bind, skin_children) in binds_query.iter_mut() {
        // Already assigned
        if Some(skin_asset_handle) != skin_bind.previous_skin.as_ref() {
            // Clear
            skin_bind.joint_entities.clear();
            skin_bind.previous_skin = Some(skin_asset_handle.clone());
            skin_bind.instance = None;
        }

        if let Some(skin_asset) = skin_assets.get(skin_asset_handle) {
            // Ensure bone capacity and assign the root entity
            skin_bind
                .joint_entities
                .resize(skin_asset.hierarchy.len(), None);
            skin_bind.joint_entities[0] = Some(skin_bind.root);

            // TODO: Don't look for these every time!
            // Look for skeleton entities
            for entity_index in 1..skin_asset.hierarchy.len() {
                skin_asset.hierarchy.find_entity(
                    entity_index as u16,
                    &mut skin_bind.joint_entities,
                    &mut children_query,
                    &mut name_query,
                );
            }

            if skin_bind.instance.is_none() {
                // Create skin instance
                let mut joint_matrices = vec![];
                joint_matrices.resize(
                    skin_asset.inverse_bind_matrices.len(),
                    Mat4::identity().to_cols_array(),
                );
                skin_bind.instance = Some(skin_instances.add(SkinInstance { joint_matrices }));
            }

            // Bind skins in the renderers

            let skin_instance_handle = skin_bind
                .instance
                .as_ref()
                .expect("missing skin instance handle");

            for renderer_entity in skin_bind.renderers.iter().chain(skin_children.iter()) {
                // Insert right skinning info
                commands.insert_one(*renderer_entity, skin_instance_handle.clone());

                // Change render pipeline
                if let Ok(mut renderer) = renderers_query.get_mut(*renderer_entity) {
                    renderer.pipelines[0].pipeline = FORWARD_SKINNED_PIPELINE_HANDLE.typed();
                }
            }

            let skin_instance = skin_instances
                .get_mut(skin_instance_handle)
                .expect("missing skin instance");

            skin_instance
                .joint_matrices
                .iter_mut()
                .zip(
                    skin_bind
                        .joint_entities
                        .iter()
                        .skip(1) // Skip the root entity
                        .zip(skin_asset.inverse_bind_matrices.iter()),
                )
                .for_each(|(joint_matrix, (joint_entity, joint_inverse_matrix))| {
                    if let Some(entity) = joint_entity {
                        if let Ok(global_transform) = transforms_query.get(*entity) {
                            *joint_matrix = (global_transform.compute_matrix()
                                * (*joint_inverse_matrix))
                                .to_cols_array();
                        }
                    }
                });
        }
    }

    std::mem::drop(__guard);
}

///////////////////////////////////////////////////////////////////////////////

// TODO: Fix this component arrangement
// TODO: Custom shaders for gizmos

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct SkinDebugger {
    #[reflect(ignore)]
    started: bool,
    #[reflect(ignore)]
    mesh: Option<Handle<Mesh>>,
    #[reflect(ignore)]
    entity: Option<Entity>,
}

pub(crate) fn skinning_debugger_update(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    skins: Res<Assets<SkinAsset>>,
    mut debugger_query: Query<(&Handle<SkinAsset>, &SkinComponent, &mut SkinDebugger)>,
    bones_query: Query<(&GlobalTransform,)>,
) {
    for (skin_handle, skinner, mut debugger) in debugger_query.iter_mut() {
        if skinner.previous_skin.as_ref() != Some(skin_handle) {
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
                let bone_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.05 }));
                // TODO: Keep track of all these entities, their position will only be updated if the parent GlobalTransform changes ...
                for bone in skinner.joint_entities.iter() {
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
                        // TODO: Ignore depth test and draw skeleton on top of everything
                        ..Default::default()
                    })
                    .current_entity()
            }

            let positions = skinner
                .joint_entities
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

            for (i, ((parent_index, _), _)) in skin.hierarchy.iter().enumerate() {
                if let Some(parent) = positions.get(*parent_index as usize) {
                    indices.push(vertices.len() as u32);
                    vertices.push(positions[i].into());
                    indices.push(vertices.len() as u32);
                    vertices.push((*parent).into());
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
