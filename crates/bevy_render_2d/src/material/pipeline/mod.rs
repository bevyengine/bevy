pub mod commands;
pub mod instances;
pub mod prepared_asset;
pub mod properties;
pub mod specialization;

use core::marker::PhantomData;

use bevy_asset::Handle;
use bevy_ecs::resource::Resource;
use bevy_render::{
    render_phase::SetItemPipeline,
    render_resource::{BindGroupLayout, Shader},
};

use crate::mesh_pipeline::{
    commands::{DrawMesh2d, SetMesh2dBindGroup, SetMesh2dViewBindGroup},
    pipeline::Mesh2dPipeline,
};

use super::Material2d;

use commands::SetMaterial2dBindGroup;

pub type DrawMaterial2d<M> = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    SetMaterial2dBindGroup<M, 2>,
    DrawMesh2d,
);

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
