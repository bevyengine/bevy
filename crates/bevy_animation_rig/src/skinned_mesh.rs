use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_ecs::{
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    reflect::{ReflectComponent, ReflectMapEntities},
    system::{Query, Res, ResMut},
};
use bevy_math::Mat4;
use bevy_pbr::render_graph;
use bevy_reflect::{
    serde, DynamicStruct, FieldIter, Reflect, ReflectMut, ReflectRef, Struct, TypeUuid,
};
use bevy_render::{
    pipeline::PipelineDescriptor,
    render_graph::{RenderGraph, RenderResourcesNode},
    renderer::{
        RenderResource, RenderResourceHints, RenderResourceIterator, RenderResourceType,
        RenderResources,
    },
    shader::{Shader, ShaderStage},
    texture::Texture,
};
use bevy_transform::components::GlobalTransform;

/// The name of skinned mesh node
pub mod node {
    pub const SKINNED_MESH: &str = "skinned_mesh";
}

/// The name of skinned mesh buffer
pub mod buffer {
    pub const JOINT_TRANSFORMS: &str = "JointTransforms";
}

/// Specify RenderPipelines with this handle to render the skinned mesh.
pub const SKINNED_MESH_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0x14db1922328e7fcc);

/// Used to update and bind joint transforms to the skinned mesh render pipeline specified with [`SKINNED_MESH_PIPELINE_HANDLE`].
///
/// The length of entities vector passed to [`SkinnedMesh::new()`] should equal to the number of matrices inside [`SkinnedMeshInverseBindposes`].
///
/// The content of `joint_transforms` can be modified manually if [`skinned_mesh_update`] system is disabled.
///
/// # Example
/// ```
/// use bevy_animation_rig::{SkinnedMesh, SKINNED_MESH_PIPELINE_HANDLE};
/// use bevy_ecs::{entity::Entity, system::Commands};
/// use bevy_pbr::prelude::PbrBundle;
/// use bevy_render::pipeline::{RenderPipeline, RenderPipelines};
///
/// fn example_system(mut commands: Commands) {
///     commands.spawn_bundle(PbrBundle {
///         render_pipelines: RenderPipelines::from_pipelines(
///             vec![RenderPipeline::new(SKINNED_MESH_PIPELINE_HANDLE.typed())]
///         ),
///         ..Default::default()
///     }).insert(SkinnedMesh::new(
///         // Refer to [`SkinnedMeshInverseBindposes`] example on how to create inverse bindposes data.
///         Default::default(),
///         // Specify joint entities here.
///         vec![Entity::new(0)]
///     ));
/// }
/// ```
#[derive(Debug, Default, Clone, Reflect)]
#[reflect(Component, MapEntities)]
pub struct SkinnedMesh {
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    pub joints: Vec<SkinnedMeshJoint>,
}

impl SkinnedMesh {
    pub fn new(
        inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
        joint_entities: impl IntoIterator<Item = Entity>,
    ) -> Self {
        Self {
            inverse_bindposes,
            joints: joint_entities
                .into_iter()
                .map(|entity| SkinnedMeshJoint {
                    entity,
                    transform: Mat4::IDENTITY,
                })
                .collect(),
        }
    }

    pub fn update_joint_transforms(
        &mut self,
        inverse_bindposes_assets: &Res<Assets<SkinnedMeshInverseBindposes>>,
        global_transform_query: &Query<&GlobalTransform>,
    ) {
        let inverse_bindposes = inverse_bindposes_assets
            .get(self.inverse_bindposes.clone())
            .unwrap();

        for (joint, &inverse_bindpose) in self.joints.iter_mut().zip(inverse_bindposes.0.iter()) {
            let global_transform = global_transform_query.get(joint.entity).unwrap();
            joint.transform = global_transform.compute_matrix() * inverse_bindpose;
        }
    }
}

impl MapEntities for SkinnedMesh {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for joint in &mut self.joints {
            joint.entity = entity_map.get(joint.entity)?;
        }

        Ok(())
    }
}

impl RenderResource for SkinnedMesh {
    fn resource_type(&self) -> Option<RenderResourceType> {
        Some(RenderResourceType::Buffer)
    }

    fn write_buffer_bytes(&self, buffer: &mut [u8]) {
        let transform_size = std::mem::size_of::<[f32; 16]>();

        for (index, joint) in self.joints.iter().enumerate() {
            joint.transform.write_buffer_bytes(
                &mut buffer[index * transform_size..(index + 1) * transform_size],
            );
        }
    }

    fn buffer_byte_len(&self) -> Option<usize> {
        Some(self.joints.len() * std::mem::size_of::<[f32; 16]>())
    }

    fn texture(&self) -> Option<&Handle<Texture>> {
        None
    }
}

impl RenderResources for SkinnedMesh {
    fn render_resources_len(&self) -> usize {
        1
    }

    fn get_render_resource(&self, index: usize) -> Option<&dyn RenderResource> {
        (index == 0).then(|| self as &dyn RenderResource)
    }

    fn get_render_resource_name(&self, index: usize) -> Option<&str> {
        (index == 0).then(|| buffer::JOINT_TRANSFORMS)
    }

    // Used to tell GLSL to use storage buffer instead of uniform buffer
    fn get_render_resource_hints(&self, index: usize) -> Option<RenderResourceHints> {
        (index == 0).then(|| RenderResourceHints::BUFFER)
    }

    fn iter(&self) -> RenderResourceIterator {
        RenderResourceIterator::new(self)
    }
}

/// Store data for each joint belongs to the [`SkinnedMesh`]
#[derive(Debug, Clone)]
pub struct SkinnedMeshJoint {
    pub entity: Entity,
    pub transform: Mat4,
}

/// Manually implement [`bevy_reflect::Reflect`] for [`SkinnedMeshJoint`] to work around an issue,
/// where spawning a scene with a component containings a vector of structs would result in runtime panic.
unsafe impl Reflect for SkinnedMeshJoint {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    #[inline]
    fn any(&self) -> &dyn std::any::Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Workaround
    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Struct(struct_value) = value.reflect_ref() {
            for (i, value) in struct_value.iter_fields().enumerate() {
                let name = struct_value.name_at(i).unwrap();

                if let Some(v) = self.field_mut(name) {
                    v.apply(value)
                }
            }
        } else {
            panic!("Attempted to apply non-struct type to struct type.");
        }
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Struct(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Struct(self)
    }

    fn serializable(&self) -> Option<serde::Serializable> {
        None
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, _value: &dyn Reflect) -> Option<bool> {
        None
    }
}

/// Manually implement [`bevy_reflect::Struct`] for [`SkinnedMeshJoint`] because it is required by [`bevy_reflect::Reflect`] trait.
impl Struct for SkinnedMeshJoint {
    fn field(&self, name: &str) -> Option<&dyn Reflect> {
        match name {
            "entity" => Some(&self.entity),
            "transform" => Some(&self.transform),
            _ => None,
        }
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        match name {
            "entity" => Some(&mut self.entity),
            "transform" => Some(&mut self.transform),
            _ => None,
        }
    }

    fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        match index {
            0 => Some(&self.entity),
            1 => Some(&self.transform),
            _ => None,
        }
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        match index {
            0 => Some(&mut self.entity),
            1 => Some(&mut self.transform),
            _ => None,
        }
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        match index {
            0 => Some("entity"),
            1 => Some("transform"),
            _ => None,
        }
    }

    fn field_len(&self) -> usize {
        2
    }

    fn iter_fields(&self) -> FieldIter {
        FieldIter::new(self)
    }

    fn clone_dynamic(&self) -> DynamicStruct {
        let mut dynamic = DynamicStruct::default();
        dynamic.set_name(self.type_name().to_string());
        dynamic.insert_boxed("entity", self.entity.clone_value());
        dynamic.insert_boxed("transform", self.transform.clone_value());
        dynamic
    }
}

/// Store joint inverse bindpose matrices. It can be shared between SkinnedMesh instances using assets.
///
/// The matrices can be loaded automatically from glTF or can be defined manually.
///
/// # Example
/// ```
/// use bevy_asset::Assets;
/// use bevy_animation_rig::{SkinnedMesh, SkinnedMeshInverseBindposes, SKINNED_MESH_PIPELINE_HANDLE};
/// use bevy_ecs::{entity::Entity, system::{Commands, ResMut}};
/// use bevy_math::Mat4;
/// use bevy_pbr::prelude::PbrBundle;
/// use bevy_render::pipeline::{RenderPipeline, RenderPipelines};
///
/// fn example_system(mut commands: Commands, mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>) {
///     // A skeleton with only 2 joints
///     let skinned_mesh_inverse_bindposes = skinned_mesh_inverse_bindposes_assets.add(SkinnedMeshInverseBindposes(vec![
///         Mat4::IDENTITY,
///         Mat4::IDENTITY,
///     ]));
///
///     // The inverse bindposes then can be shared between multiple skinned mesh instances
///     for _ in 0..3 {
///         commands.spawn_bundle(PbrBundle {
///             render_pipelines: RenderPipelines::from_pipelines(
///                 vec![RenderPipeline::new(SKINNED_MESH_PIPELINE_HANDLE.typed())]
///             ),
///             ..Default::default()
///         }).insert(SkinnedMesh::new(
///             skinned_mesh_inverse_bindposes.clone(),
///             // Remember to assign joint entity here!
///             vec![Entity::new(0); 2],
///         ));
///     }
/// }
/// ```
#[derive(Debug, TypeUuid)]
#[uuid = "b9f155a9-54ec-4026-988f-e0a03e99a76f"]
pub struct SkinnedMeshInverseBindposes(pub Vec<Mat4>);

pub fn skinned_mesh_setup(
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    let mut skinned_mesh_pipeline = pipelines
        .get(render_graph::PBR_PIPELINE_HANDLE)
        .unwrap()
        .clone();
    skinned_mesh_pipeline.name = Some("Skinned Mesh Pipeline".into());
    skinned_mesh_pipeline.shader_stages.vertex = shaders.add(Shader::from_glsl(
        ShaderStage::Vertex,
        include_str!("skinned_mesh.vert"),
    ));
    pipelines.set_untracked(SKINNED_MESH_PIPELINE_HANDLE, skinned_mesh_pipeline);

    render_graph.add_system_node(
        node::SKINNED_MESH,
        RenderResourcesNode::<SkinnedMesh>::new(false),
    );
    render_graph
        .add_node_edge(
            node::SKINNED_MESH,
            bevy_render::render_graph::base::node::MAIN_PASS,
        )
        .unwrap();
}

pub fn skinned_mesh_update(
    skinned_mesh_inverse_bindposes_assets: Res<Assets<SkinnedMeshInverseBindposes>>,
    global_transform_query: Query<&GlobalTransform>,
    mut skinned_mesh_query: Query<&mut SkinnedMesh>,
) {
    skinned_mesh_query.for_each_mut(|mut skinned_mesh| {
        skinned_mesh.update_joint_transforms(
            &skinned_mesh_inverse_bindposes_assets,
            &global_transform_query,
        );
    });
}
