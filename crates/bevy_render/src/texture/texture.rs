use super::{SamplerDescriptor, TextureDescriptor};
use crate::{
    renderer::{RenderResourceContext, RenderResources},
    shader::ShaderDefSuffixProvider,
};
use bevy_app::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_derive::FromResources;
use legion::prelude::*;
use std::{collections::HashSet, fs::File};
use glam::Vec2;

pub const TEXTURE_ASSET_INDEX: usize = 0;
pub const SAMPLER_ASSET_INDEX: usize = 1;
pub enum TextureType {
    Data(Vec<u8>, usize, usize),
    Png(String), // TODO: please rethink this
}

pub struct Texture {
    pub data: Vec<u8>,
    pub size: Vec2,
}

impl Texture {
    pub fn aspect(&self) -> f32 {
        self.size.y() / self.size.x()
    }

    pub fn load(descriptor: TextureType) -> Self {
        let (data, width, height) = match descriptor {
            TextureType::Data(data, width, height) => (data.clone(), width, height),
            TextureType::Png(path) => {
                let decoder = png::Decoder::new(File::open(&path).unwrap());
                let (info, mut reader) = decoder.read_info().unwrap();
                let mut buf = vec![0; info.buffer_size()];
                reader.next_frame(&mut buf).unwrap();
                (buf, info.width as usize, info.height as usize)
            }
        };

        Texture {
            data,
            size: Vec2::new(width as f32, height as f32)
        }
    }

    pub fn texture_resource_system(
        mut state: ResMut<TextureResourceSystemState>,
        render_resources: Res<RenderResources>,
        textures: Res<Assets<Texture>>,
        texture_events: Res<Events<AssetEvent<Texture>>>,
    ) {
        let render_resources = &*render_resources.context;
        let mut changed_textures = HashSet::new();
        for event in state.event_reader.iter(&texture_events) {
            match event {
                AssetEvent::Created { handle } => {
                    changed_textures.insert(*handle);
                }
                AssetEvent::Modified { handle } => {
                    changed_textures.insert(*handle);
                    Self::remove_current_texture_resources(render_resources, *handle);
                }
                AssetEvent::Removed { handle } => {
                    Self::remove_current_texture_resources(render_resources, *handle);
                    // if texture was modified and removed in the same update, ignore the modification
                    // events are ordered so future modification events are ok
                    changed_textures.remove(handle);
                }
            }
        }

        for texture_handle in changed_textures.iter() {
            if let Some(texture) = textures.get(texture_handle) {
                let texture_descriptor: TextureDescriptor = texture.into();
                let texture_resource = render_resources.create_texture(texture_descriptor);

                let sampler_descriptor: SamplerDescriptor = texture.into();
                let sampler_resource = render_resources.create_sampler(&sampler_descriptor);

                render_resources.set_asset_resource(
                    *texture_handle,
                    texture_resource,
                    TEXTURE_ASSET_INDEX,
                );
                render_resources.set_asset_resource(
                    *texture_handle,
                    sampler_resource,
                    SAMPLER_ASSET_INDEX,
                );
            }
        }
    }

    fn remove_current_texture_resources(
        render_resources: &dyn RenderResourceContext,
        handle: Handle<Texture>,
    ) {
        if let Some(resource) = render_resources.get_asset_resource(handle, TEXTURE_ASSET_INDEX) {
            render_resources.remove_texture(resource);
            render_resources.remove_asset_resource(handle, TEXTURE_ASSET_INDEX);
        }
        if let Some(resource) = render_resources.get_asset_resource(handle, SAMPLER_ASSET_INDEX) {
            render_resources.remove_sampler(resource);
            render_resources.remove_asset_resource(handle, SAMPLER_ASSET_INDEX);
        }
    }
}

#[derive(FromResources)]
pub struct TextureResourceSystemState {
    event_reader: EventReader<AssetEvent<Texture>>,
}

impl ShaderDefSuffixProvider for Option<Handle<Texture>> {
    fn get_shader_def(&self) -> Option<&'static str> {
        match *self {
            Some(_) => Some(""),
            None => None,
        }
    }
}
