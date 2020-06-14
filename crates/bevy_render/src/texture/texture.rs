use super::{SamplerDescriptor, TextureDescriptor};
use crate::{
    render_resource::{RenderResource, RenderResourceId, RenderResourceType},
    renderer::RenderResourceContext,
};
use bevy_app::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use glam::Vec2;
use legion::prelude::*;
use std::collections::HashSet;

pub const TEXTURE_ASSET_INDEX: usize = 0;
pub const SAMPLER_ASSET_INDEX: usize = 1;

#[derive(Default, Clone)]
pub struct Texture {
    pub data: Vec<u8>,
    pub size: Vec2,
}

const FORMAT_SIZE: usize = 4; // TODO: get this from an actual format type

impl Texture {
    pub fn new(data: Vec<u8>, size: Vec2) -> Self {
        Self { data, size }
    }

    pub fn new_fill(size: Vec2, pixel: &[u8]) -> Self {
        let mut value = Self::default();
        value.resize(size);
        for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
            current_pixel.copy_from_slice(&pixel);
        }
        value
    }

    pub fn aspect(&self) -> f32 {
        self.size.y() / self.size.x()
    }

    pub fn resize(&mut self, size: Vec2) {
        self.size = size;
        let width = size.x() as usize;
        let height = size.y() as usize;
        self.data.resize(width * height * FORMAT_SIZE, 0);
    }

    pub fn texture_resource_system(
        mut state: ResMut<TextureResourceSystemState>,
        render_resource_context: Res<Box<dyn RenderResourceContext>>,
        textures: Res<Assets<Texture>>,
        texture_events: Res<Events<AssetEvent<Texture>>>,
    ) {
        let render_resource_context = &**render_resource_context;
        let mut changed_textures = HashSet::new();
        for event in state.event_reader.iter(&texture_events) {
            match event {
                AssetEvent::Created { handle } => {
                    changed_textures.insert(*handle);
                }
                AssetEvent::Modified { handle } => {
                    changed_textures.insert(*handle);
                    Self::remove_current_texture_resources(render_resource_context, *handle);
                }
                AssetEvent::Removed { handle } => {
                    Self::remove_current_texture_resources(render_resource_context, *handle);
                    // if texture was modified and removed in the same update, ignore the modification
                    // events are ordered so future modification events are ok
                    changed_textures.remove(handle);
                }
            }
        }

        for texture_handle in changed_textures.iter() {
            if let Some(texture) = textures.get(texture_handle) {
                let texture_descriptor: TextureDescriptor = texture.into();
                let texture_resource = render_resource_context.create_texture(texture_descriptor);

                let sampler_descriptor: SamplerDescriptor = texture.into();
                let sampler_resource = render_resource_context.create_sampler(&sampler_descriptor);

                render_resource_context.set_asset_resource(
                    *texture_handle,
                    RenderResourceId::Texture(texture_resource),
                    TEXTURE_ASSET_INDEX,
                );
                render_resource_context.set_asset_resource(
                    *texture_handle,
                    RenderResourceId::Sampler(sampler_resource),
                    SAMPLER_ASSET_INDEX,
                );
            }
        }
    }

    fn remove_current_texture_resources(
        render_resource_context: &dyn RenderResourceContext,
        handle: Handle<Texture>,
    ) {
        if let Some(RenderResourceId::Texture(resource)) =
            render_resource_context.get_asset_resource(handle, TEXTURE_ASSET_INDEX)
        {
            render_resource_context.remove_texture(resource);
            render_resource_context.remove_asset_resource(handle, TEXTURE_ASSET_INDEX);
        }
        if let Some(RenderResourceId::Sampler(resource)) =
            render_resource_context.get_asset_resource(handle, SAMPLER_ASSET_INDEX)
        {
            render_resource_context.remove_sampler(resource);
            render_resource_context.remove_asset_resource(handle, SAMPLER_ASSET_INDEX);
        }
    }
}

#[derive(Default)]
pub struct TextureResourceSystemState {
    event_reader: EventReader<AssetEvent<Texture>>,
}

impl RenderResource for Option<Handle<Texture>> {
    fn resource_type(&self) -> Option<RenderResourceType> {
        self.map(|_texture| RenderResourceType::Texture)
    }
    fn write_buffer_bytes(&self, _buffer: &mut [u8]) {}
    fn buffer_byte_len(&self) -> Option<usize> {
        None
    }
    fn texture(&self) -> Option<Handle<Texture>> {
        self.clone()
    }
}

impl RenderResource for Handle<Texture> {
    fn resource_type(&self) -> Option<RenderResourceType> {
        Some(RenderResourceType::Texture)
    }
    fn write_buffer_bytes(&self, _buffer: &mut [u8]) {}
    fn buffer_byte_len(&self) -> Option<usize> {
        None
    }
    fn texture(&self) -> Option<Handle<Texture>> {
        Some(self.clone())
    }
}
