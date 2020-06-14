use crate::{texture::Texture, RenderPipelines};
use bevy_asset::{Assets, Handle};
use legion::prelude::{Com, ComMut, Res};

pub use bevy_derive::ShaderDefs;

pub trait ShaderDef {
    fn is_defined(&self) -> bool;
}

pub trait ShaderDefs {
    fn shader_defs_len(&self) -> usize;
    fn get_shader_def(&self, index: usize) -> Option<&str>;
    fn iter_shader_defs(&self) -> ShaderDefIterator;
}

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

pub fn shader_def_system<T>(shader_defs: Com<T>, mut render_pipelines: ComMut<RenderPipelines>)
where
    T: ShaderDefs + Send + Sync + 'static,
{
    for shader_def in shader_defs.iter_shader_defs() {
        render_pipelines
            .render_resource_bindings
            .pipeline_specialization
            .shader_specialization
            .shader_defs
            .insert(shader_def.to_string());
    }
}

pub fn asset_shader_def_system<T>(
    assets: Res<Assets<T>>,
    asset_handle: Com<Handle<T>>,
    mut render_pipelines: ComMut<RenderPipelines>,
) where
    T: ShaderDefs + Send + Sync + 'static,
{
    let shader_defs = assets.get(&asset_handle).unwrap();
    for shader_def in shader_defs.iter_shader_defs() {
        render_pipelines
            .render_resource_bindings
            .pipeline_specialization
            .shader_specialization
            .shader_defs
            .insert(shader_def.to_string());
    }
}
