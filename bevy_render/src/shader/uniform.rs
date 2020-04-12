use crate::{
    color::ColorSource,
    pipeline::{BindType, VertexBufferDescriptor},
    render_resource::AssetBatchers,
    texture::Texture,
    Renderable,
};

use bevy_asset::{AssetStorage, Handle};
use bevy_core::bytes::GetBytes;
use legion::prelude::*;

pub trait AsUniforms {
    fn get_field_infos() -> &'static [FieldInfo];
    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>>;
    fn get_uniform_texture(&self, name: &str) -> Option<Handle<Texture>>;
    fn get_shader_defs(&self) -> Option<Vec<String>>;
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType>;
    fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]>;
    fn get_vertex_buffer_descriptor() -> Option<&'static VertexBufferDescriptor>;
}

pub fn shader_def_system<T>() -> Box<dyn Schedulable>
where
    T: AsUniforms + Send + Sync + 'static,
{
    SystemBuilder::new(format!("shader_def::{}", std::any::type_name::<T>()))
        .with_query(<(Read<T>, Write<Renderable>)>::query())
        .build(|_, world, _, query| {
            for (uniforms, mut renderable) in query.iter_mut(world) {
                if let Some(shader_defs) = uniforms.get_shader_defs() {
                    renderable
                        .render_resource_assignments
                        .shader_defs
                        .extend(shader_defs)
                }
            }
        })
}

pub fn asset_handle_shader_def_system<T>() -> Box<dyn Schedulable>
where
    T: AsUniforms + Send + Sync + 'static,
{
    SystemBuilder::new(format!(
        "asset_handle_shader_def::{}",
        std::any::type_name::<T>()
    ))
    .read_resource::<AssetStorage<T>>()
    .with_query(<(Read<Handle<T>>, Write<Renderable>)>::query())
    .build(|_, world, asset_storage, query| {
        for (uniform_handle, mut renderable) in query.iter_mut(world) {
            if !renderable.is_visible || renderable.is_instanced {
                continue;
            }

            let uniforms = asset_storage.get(&uniform_handle).unwrap();
            if let Some(shader_defs) = uniforms.get_shader_defs() {
                renderable
                    .render_resource_assignments
                    .shader_defs
                    .extend(shader_defs)
            }
        }
    })
}

pub fn asset_handle_batcher_system<T>() -> Box<dyn Schedulable>
where
    T: AsUniforms + Send + Sync + 'static,
{
    SystemBuilder::new(format!(
        "asset_handle_batcher::{}",
        std::any::type_name::<T>()
    ))
    .write_resource::<AssetBatchers>()
    .with_query(<(Read<Handle<T>>, Read<Renderable>)>::query())
    .build(|_, world, asset_batchers, query| {
        for (entity, (uniform_handle, renderable)) in query.iter_entities(world) {
            if !renderable.is_visible || renderable.is_instanced {
                continue;
            }

            asset_batchers.set_entity_handle(entity, *uniform_handle);
        }
    })
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

pub enum FieldBindType {
    Uniform { size: usize },
    Texture,
}

pub struct FieldInfo {
    pub name: &'static str,
    pub uniform_name: &'static str,
    pub texture_name: &'static str,
    pub sampler_name: &'static str,
    pub is_instanceable: bool,
}

pub trait AsFieldBindType {
    fn get_field_bind_type(&self) -> Option<FieldBindType>;
}

impl AsFieldBindType for ColorSource {
    fn get_field_bind_type(&self) -> Option<FieldBindType> {
        match *self {
            ColorSource::Texture(_) => Some(FieldBindType::Texture),
            ColorSource::Color(color) => color.get_field_bind_type(),
        }
    }
}

impl AsFieldBindType for Option<Handle<Texture>> {
    fn get_field_bind_type(&self) -> Option<FieldBindType> {
        match *self {
            Some(_) => Some(FieldBindType::Texture),
            None => None,
        }
    }
}

impl AsFieldBindType for Handle<Texture> {
    fn get_field_bind_type(&self) -> Option<FieldBindType> {
        Some(FieldBindType::Texture)
    }
}

impl<T> AsFieldBindType for T
where
    T: GetBytes,
{
    // TODO: this breaks if get_bytes_ref() isn't supported for a datatype
    default fn get_field_bind_type(&self) -> Option<FieldBindType> {
        Some(FieldBindType::Uniform {
            size: self.get_bytes_ref().unwrap().len(),
        })
    }
}

pub trait GetTexture {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        None
    }
}

impl<T> GetTexture for T
where
    T: GetBytes,
{
    default fn get_texture(&self) -> Option<Handle<Texture>> {
        None
    }
}

impl GetTexture for Handle<Texture> {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        Some(self.clone())
    }
}

impl GetTexture for Option<Handle<Texture>> {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        *self
    }
}

impl GetTexture for ColorSource {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        match self {
            ColorSource::Color(_) => None,
            ColorSource::Texture(texture) => Some(texture.clone()),
        }
    }
}

pub struct UniformInfo<'a> {
    pub name: &'a str,
    pub bind_type: BindType,
}
