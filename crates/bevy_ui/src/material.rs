use std::{hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Handle};
use bevy_ecs::system::Resource;
use bevy_reflect::{TypePath, TypeUuid};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    render_resource::{
        AsBindGroup, BindGroupLayout, RenderPipelineDescriptor, Shader,
        ShaderRef, SpecializedRenderPipeline,
    },
    RenderApp,
};

use crate::{UiPipeline, UiPipelineKey};

pub struct UiMaterialKey<M: UiMaterial> {
    pub is_hdr: bool,
    pub bind_group_data: M::Data,
}

impl<M: UiMaterial> Eq for UiMaterialKey<M> where M::Data: PartialEq {}

impl<M: UiMaterial> PartialEq for UiMaterialKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.is_hdr == other.is_hdr && self.bind_group_data == other.bind_group_data
    }
}

impl<M: UiMaterial> Clone for UiMaterialKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            is_hdr: self.is_hdr,
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: UiMaterial> Hash for UiMaterialKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.is_hdr.hash(state);
        self.bind_group_data.hash(state);
    }
}

pub trait UiMaterial: AsBindGroup + Send + Sync + Clone + TypeUuid + TypePath + Sized {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    #[allow(unused_variables)]
    #[inline]
    fn specialize(descriptor: &mut RenderPipelineDescriptor, key: UiMaterialKey<Self>) {}
}

pub struct UiMaterialPlugin<M: UiMaterial> {
    pub _marker: PhantomData<M>,
}

impl<M: UiMaterial> Default for UiMaterialPlugin<M> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

impl<M: UiMaterial> Plugin for UiMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugins(ExtractComponentPlugin::<Handle<M>>::extract_visible());
    
        
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            todo!("Implement this further")
        }
    }
}

#[derive(Resource)]
pub struct UiMaterialPipeline<M: UiMaterial> {
    pub ui_pipeline: UiPipeline,
    pub ui_material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    markus: PhantomData<M>,
}

impl<M: UiMaterial> SpecializedRenderPipeline for UiMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = UiMaterialKey<M>;
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut descriptor = self
            .ui_pipeline
            .specialize(UiPipelineKey { hdr: key.is_hdr });
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout = vec![
            self.ui_pipeline.view_layout.clone(),
            self.ui_pipeline.image_layout.clone(),
            self.ui_material_layout.clone(),
        ];

        M::specialize(&mut descriptor, key);
        descriptor
    }
}
