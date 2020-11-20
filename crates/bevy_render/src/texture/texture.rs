use super::{SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat};
use crate::renderer::{
    RenderResource, RenderResourceContext, RenderResourceId, RenderResourceType,
};
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{Res, ResMut};
use bevy_math::{Vec2, Vec3};
use bevy_type_registry::TypeUuid;
use bevy_utils::HashSet;

pub const TEXTURE_ASSET_INDEX: u64 = 0;
pub const SAMPLER_ASSET_INDEX: u64 = 1;

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "6ea26da6-6cf8-4ea2-9986-1d7bf6c17d6f"]
pub struct Texture {
    pub data: Vec<u8>,
    pub size: Vec3,
    pub format: TextureFormat,
    pub dimension: TextureDimension,
    pub sampler: SamplerDescriptor,
}

impl Default for Texture {
    fn default() -> Self {
        Texture {
            data: Default::default(),
            size: Default::default(),
            format: TextureFormat::Rgba8UnormSrgb,
            dimension: TextureDimension::D2,
            sampler: Default::default(),
        }
    }
}

impl Texture {
    pub fn new(
        size: Vec3,
        data: Vec<u8>,
        dimension: TextureDimension,
        format: TextureFormat,
    ) -> Self {
        debug_assert_eq!(
            size.x as usize * size.y as usize * size.z as usize * format.pixel_size(),
            data.len(),
            "Pixel data, size and format have to match",
        );
        Self {
            data,
            size,
            dimension,
            format,
            ..Default::default()
        }
    }

    pub fn new_2d(size: Vec2, data: Vec<u8>, format: TextureFormat) -> Self {
        Self::new(size.extend(1.0), data, TextureDimension::D2, format)
    }

    pub fn new_fill(
        size: Vec3,
        dimension: TextureDimension,
        pixel: &[u8],
        format: TextureFormat,
    ) -> Self {
        let mut value = Texture {
            format,
            dimension,
            ..Default::default()
        };
        value.resize(size);

        debug_assert_eq!(
            pixel.len() % format.pixel_size(),
            0,
            "Must not have incomplete pixel data"
        );
        debug_assert!(
            pixel.len() <= value.data.len(),
            "Fill data must fit within pixel buffer"
        );

        for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
            current_pixel.copy_from_slice(&pixel);
        }
        value
    }

    pub fn new_fill_2d(size: Vec2, pixel: &[u8], format: TextureFormat) -> Self {
        Self::new_fill(size.extend(1.0), TextureDimension::D2, pixel, format)
    }

    pub fn aspect_2d(&self) -> f32 {
        self.size.y / self.size.x
    }

    pub fn size_2d(&self) -> Vec2 {
        self.size.truncate()
    }

    pub fn resize(&mut self, size: Vec3) {
        self.size = size;
        let width = size.x as usize;
        let height = size.y as usize;
        let depth = size.z as usize;
        self.data
            .resize(width * height * depth * self.format.pixel_size(), 0);
    }

    pub fn resize_2d(&mut self, size: Vec2) {
        self.resize(size.extend(1.0))
    }

    /// Changes the `size`, asserting that the total number of data elements (pixels) remains the same.
    pub fn reinterpret_size(&mut self, new_size: Vec3) {
        let old_width = self.size.x as usize;
        let old_height = self.size.y as usize;
        let old_depth = self.size.z as usize;
        let new_width = new_size.x as usize;
        let new_height = new_size.y as usize;
        let new_depth = new_size.z as usize;

        assert!(
            new_width * new_height * new_depth == old_width * old_height * old_depth,
            "Incompatible sizes: old = {} new = {}",
            self.size,
            new_size
        );

        self.size = new_size;
    }

    /// Takes a 2D texture containing vertically stacked images of the same size, and reinterprets it as a 2D array texture,
    /// where each of the stacked images becomes one layer of the array. This is primarily for use with the `texture2DArray`
    /// shader uniform type.
    pub fn reinterpret_stacked_2d_as_array(&mut self, layers: usize) {
        assert!(self.dimension == TextureDimension::D2);
        let mut new_size = self.size;
        new_size.y = (self.size.y as usize / layers) as f32;
        new_size.z = layers as f32;
        self.reinterpret_size(new_size);
    }

    pub fn texture_resource_system(
        mut state: ResMut<TextureResourceSystemState>,
        render_resource_context: Res<Box<dyn RenderResourceContext>>,
        textures: Res<Assets<Texture>>,
        texture_events: Res<Events<AssetEvent<Texture>>>,
    ) {
        let render_resource_context = &**render_resource_context;
        let mut changed_textures = HashSet::default();
        for event in state.event_reader.iter(&texture_events) {
            match event {
                AssetEvent::Created { handle } => {
                    changed_textures.insert(handle);
                }
                AssetEvent::Modified { handle } => {
                    changed_textures.insert(handle);
                    Self::remove_current_texture_resources(render_resource_context, handle);
                }
                AssetEvent::Removed { handle } => {
                    Self::remove_current_texture_resources(render_resource_context, handle);
                    // if texture was modified and removed in the same update, ignore the modification
                    // events are ordered so future modification events are ok
                    changed_textures.remove(handle);
                }
            }
        }

        for texture_handle in changed_textures.iter() {
            if let Some(texture) = textures.get(*texture_handle) {
                let texture_descriptor: TextureDescriptor = texture.into();
                let texture_resource = render_resource_context.create_texture(texture_descriptor);

                let sampler_resource = render_resource_context.create_sampler(&texture.sampler);

                render_resource_context.set_asset_resource(
                    texture_handle,
                    RenderResourceId::Texture(texture_resource),
                    TEXTURE_ASSET_INDEX,
                );
                render_resource_context.set_asset_resource(
                    texture_handle,
                    RenderResourceId::Sampler(sampler_resource),
                    SAMPLER_ASSET_INDEX,
                );
            }
        }
    }

    fn remove_current_texture_resources(
        render_resource_context: &dyn RenderResourceContext,
        handle: &Handle<Texture>,
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
        self.as_ref().map(|_texture| RenderResourceType::Texture)
    }

    fn write_buffer_bytes(&self, _buffer: &mut [u8]) {}

    fn buffer_byte_len(&self) -> Option<usize> {
        None
    }

    fn texture(&self) -> Option<&Handle<Texture>> {
        self.as_ref()
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

    fn texture(&self) -> Option<&Handle<Texture>> {
        Some(self)
    }
}
