use core::marker::PhantomData;

use bevy_asset::Handle;
use bevy_ecs::resource::Resource;
use bevy_render::render_resource::{BindGroupLayout, Shader};

use crate::mesh_pipeline::pipeline::Mesh2dPipeline;

use super::Material2d;

/// Render pipeline data for a given [`Material2d`]
#[derive(Resource)]
pub struct Material2dPipeline<M: Material2d> {
    pub mesh2d_pipeline: Mesh2dPipeline,
    pub material2d_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: Material2d> Material2dPipeline<M> {
    pub fn new(
        mesh2d_pipeline: Mesh2dPipeline,
        material2d_layout: BindGroupLayout,
        vertex_shader: Option<Handle<Shader>>,
        fragment_shader: Option<Handle<Shader>>,
    ) -> Self {
        Self {
            mesh2d_pipeline,
            material2d_layout,
            vertex_shader,
            fragment_shader,
            marker: PhantomData,
        }
    }
}

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
