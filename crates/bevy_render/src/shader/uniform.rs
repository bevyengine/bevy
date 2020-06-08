use crate::{pipeline::BindType, texture::Texture, Renderable};
use bevy_asset::{Assets, Handle};
use legion::prelude::*;

pub use bevy_derive::{Uniform, Uniforms};

pub trait Uniforms: Send + Sync + 'static {
    fn get_field_infos() -> &'static [FieldInfo];
    fn write_uniform_bytes(&self, name: &str, buffer: &mut [u8]);
    fn uniform_byte_len(&self, name: &str) -> usize;
    fn get_uniform_texture(&self, name: &str) -> Option<Handle<Texture>>;
    fn get_shader_defs(&self) -> Option<Vec<String>>;
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType>;
}

pub fn shader_def_system<T>(uniforms: Com<T>, mut renderable: ComMut<Renderable>)
where
    T: Uniforms + Send + Sync + 'static,
{
    if let Some(shader_defs) = uniforms.get_shader_defs() {
        renderable
            .render_resource_assignments
            .pipeline_specialization
            .shader_specialization
            .shader_defs
            .extend(shader_defs)
    }
}

pub fn asset_shader_def_system<T>(
    assets: Res<Assets<T>>,
    asset_handle: Com<Handle<T>>,
    mut renderable: ComMut<Renderable>,
) where
    T: Uniforms + Send + Sync + 'static,
{
    if !renderable.is_visible || renderable.is_instanced {
        return;
    }

    let uniforms = assets.get(&asset_handle).unwrap();
    if let Some(shader_defs) = uniforms.get_shader_defs() {
        renderable
            .render_resource_assignments
            .pipeline_specialization
            .shader_specialization
            .shader_defs
            .extend(shader_defs)
    }
}

pub trait ShaderDefSuffixProvider {
    fn get_shader_def(&self) -> Option<&'static str>;
}

impl ShaderDefSuffixProvider for bool {
    fn get_shader_def(&self) -> Option<&'static str> {
        match *self {
            true => Some(""),
            false => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldBindType {
    Uniform { size: usize },
    Buffer { size: usize },
    Texture,
}

pub struct FieldInfo {
    pub name: &'static str,
    pub uniform_name: &'static str,
    pub texture_name: &'static str,
    pub sampler_name: &'static str,
}

pub struct UniformInfo<'a> {
    pub name: &'a str,
    pub bind_type: BindType,
}
