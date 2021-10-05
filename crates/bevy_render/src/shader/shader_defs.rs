use bevy_asset::{Asset, Assets, Handle};

use crate::{draw::OutsideFrustum, pipeline::RenderPipelines, Texture};
pub use bevy_derive::ShaderDefs;
use bevy_ecs::{
    prelude::Component,
    query::Without,
    system::{Query, Res},
};

/// Something that can either be "defined" or "not defined". This is used to determine if a "shader
/// def" should be considered "defined"
pub trait ShaderDef {
    fn is_defined(&self) -> bool;
}

/// A collection of "shader defs", which define compile time definitions for shaders.
pub trait ShaderDefs {
    fn shader_defs_len(&self) -> usize;
    fn get_shader_def(&self, index: usize) -> Option<&str>;
    fn iter_shader_defs(&self) -> ShaderDefIterator;
}

/// Iterates over all [ShaderDef] items in [ShaderDefs]
pub struct ShaderDefIterator<'a> {
    shader_defs: &'a dyn ShaderDefs,
    index: usize,
}

impl<'a> ShaderDefIterator<'a> {
    pub fn new(shader_defs: &'a dyn ShaderDefs) -> Self {
        Self {
            shader_defs,
            index: 0,
        }
    }
}
impl<'a> Iterator for ShaderDefIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index == self.shader_defs.shader_defs_len() {
                return None;
            }
            let shader_def = self.shader_defs.get_shader_def(self.index);
            self.index += 1;
            if shader_def.is_some() {
                return shader_def;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.shader_defs.shader_defs_len()))
    }
}

impl ShaderDef for bool {
    fn is_defined(&self) -> bool {
        *self
    }
}

impl ShaderDef for Option<Handle<Texture>> {
    fn is_defined(&self) -> bool {
        self.is_some()
    }
}

/// Updates [RenderPipelines] with the latest [ShaderDefs]
pub fn shader_defs_system<T>(mut query: Query<(&T, &mut RenderPipelines), Without<OutsideFrustum>>)
where
    T: ShaderDefs + Component,
{
    for (shader_defs, mut render_pipelines) in query.iter_mut() {
        for shader_def in shader_defs.iter_shader_defs() {
            for render_pipeline in render_pipelines.pipelines.iter_mut() {
                render_pipeline
                    .specialization
                    .shader_specialization
                    .shader_defs
                    .insert(shader_def.to_string());
            }
        }
    }
}

/// Clears each [RenderPipelines]' shader defs collection
pub fn clear_shader_defs_system(mut query: Query<&mut RenderPipelines>) {
    for mut render_pipelines in query.iter_mut() {
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            render_pipeline
                .specialization
                .shader_specialization
                .shader_defs
                .clear();
        }
    }
}

/// Updates [RenderPipelines] with the latest [ShaderDefs] from a given asset type
pub fn asset_shader_defs_system<T: Asset>(
    assets: Res<Assets<T>>,
    mut query: Query<(&Handle<T>, &mut RenderPipelines), Without<OutsideFrustum>>,
) where
    T: ShaderDefs + Send + Sync + 'static,
{
    for (asset_handle, mut render_pipelines) in query.iter_mut() {
        if let Some(asset_handle) = assets.get(asset_handle) {
            let shader_defs = asset_handle;
            for shader_def in shader_defs.iter_shader_defs() {
                for render_pipeline in render_pipelines.pipelines.iter_mut() {
                    render_pipeline
                        .specialization
                        .shader_specialization
                        .shader_defs
                        .insert(shader_def.to_string());
                }
            }
        }
    }
}
