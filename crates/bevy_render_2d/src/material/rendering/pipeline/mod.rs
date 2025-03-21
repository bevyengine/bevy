pub mod cache;

use core::{hash::Hash, marker::PhantomData};

use bevy_asset::{AssetServer, Handle};
use bevy_ecs::{
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_render::{
    mesh::MeshVertexBufferLayoutRef,
    render_phase::SetItemPipeline,
    render_resource::{
        BindGroupLayout, RenderPipelineDescriptor, Shader, ShaderRef, SpecializedMeshPipeline,
        SpecializedMeshPipelineError,
    },
    renderer::RenderDevice,
};

use crate::{
    mesh::{
        DrawMesh2d, Mesh2dPipeline, Mesh2dPipelineKey, SetMesh2dBindGroup, SetMesh2dViewBindGroup,
    },
    prelude::Material2d,
};

use super::command::SetMaterial2dBindGroup;

/// Render pipeline data for a given [`Material2d`]
#[derive(Resource)]
pub struct Material2dPipeline<M: Material2d> {
    pub mesh2d_pipeline: Mesh2dPipeline,
    pub material2d_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

pub struct Material2dKey<M: Material2d> {
    pub mesh_key: Mesh2dPipelineKey,
    pub bind_group_data: M::Data,
}

pub type DrawMaterial2d<M> = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    SetMaterial2dBindGroup<M, 2>,
    DrawMesh2d,
);

impl<M: Material2d> Clone for Material2dPipeline<M> {
    fn clone(&self) -> Self {
        Self {
            mesh2d_pipeline: self.mesh2d_pipeline.clone(),
            material2d_layout: self.material2d_layout.clone(),
            vertex_shader: self.vertex_shader.clone(),
            fragment_shader: self.fragment_shader.clone(),
            marker: PhantomData,
        }
    }
}

impl<M: Material2d> SpecializedMeshPipeline for Material2dPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = Material2dKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }
        descriptor.layout = vec![
            self.mesh2d_pipeline.view_layout.clone(),
            self.mesh2d_pipeline.mesh_layout.clone(),
            self.material2d_layout.clone(),
        ];

        M::specialize(&mut descriptor, layout, key)?;
        Ok(descriptor)
    }
}

impl<M: Material2d> FromWorld for Material2dPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let material2d_layout = M::bind_group_layout(render_device);

        Material2dPipeline {
            mesh2d_pipeline: world.resource::<Mesh2dPipeline>().clone(),
            material2d_layout,
            vertex_shader: match M::vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: match M::fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            marker: PhantomData,
        }
    }
}

impl<M: Material2d> Eq for Material2dKey<M> where M::Data: PartialEq {}

impl<M: Material2d> PartialEq for Material2dKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.mesh_key == other.mesh_key && self.bind_group_data == other.bind_group_data
    }
}

impl<M: Material2d> Clone for Material2dKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            mesh_key: self.mesh_key,
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: Material2d> Hash for Material2dKey<M>
where
    M::Data: Hash,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.mesh_key.hash(state);
        self.bind_group_data.hash(state);
    }
}
