use crate::{TextureAtlas, Rect};
use bevy_asset::{Assets, Handle};
use bevy_render::texture::Texture;
use glam::Vec2;
use guillotiere::{size2, Allocation, AtlasAllocator};
use std::collections::HashMap;

pub struct TextureAtlasBuilder {
    pub texture_allocations: Vec<(Handle<Texture>, Allocation)>,
    pub atlas_allocator: AtlasAllocator,
    pub texture: Texture,
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self::new(Vec2::new(256., 256.))
    }
}

const FORMAT_SIZE: usize = 4; // TODO: get this from an actual format type
impl TextureAtlasBuilder {
    pub fn new(initial_size: Vec2) -> Self {
        let width = initial_size.x() as usize;
        let height = initial_size.y() as usize;
        Self {
            texture_allocations: Default::default(),
            atlas_allocator: AtlasAllocator::new(size2(width as i32, height as i32)),
            texture: Texture::new(vec![0; width * height * FORMAT_SIZE], initial_size),
        }
    }

    pub fn add_texture(&mut self, texture_handle: Handle<Texture>, texture: &Texture) {
        // TODO: resize if allocation fails
        let allocation = self
            .atlas_allocator
            .allocate(size2(texture.size.x() as i32, texture.size.y() as i32))
            .unwrap();
        let rect = allocation.rectangle;
        let atlas_width = self.texture.size.x() as usize;
        let rect_width = rect.width() as usize;

        for (texture_y, bound_y) in (rect.min.y..rect.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + rect.min.x as usize) * FORMAT_SIZE;
            let end = begin + rect_width * FORMAT_SIZE;
            let texture_begin = texture_y * rect_width * FORMAT_SIZE;
            let texture_end = texture_begin + rect_width * FORMAT_SIZE;
            self.texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
        }
        self.texture_allocations.push((texture_handle, allocation));
    }

    pub fn remove_texture(&mut self, texture_handle: Handle<Texture>) {
        if let Some(position) = self.texture_allocations.iter().position(|(handle, _)| *handle == texture_handle) {
            let (_, allocation) = self.texture_allocations.remove(position);
            self.atlas_allocator.deallocate(allocation.id);
        }
    }

    pub fn finish(self, textures: &mut Assets<Texture>) -> TextureAtlas {
        let mut texture_rects = Vec::with_capacity(self.texture_allocations.len());
        let mut texture_handles = HashMap::with_capacity(self.texture_allocations.len());
        for (index, (handle, allocation)) in self.texture_allocations.iter().enumerate() {
            texture_rects.push(allocation.rectangle.into());
            texture_handles.insert(*handle, index);
        }
        TextureAtlas {
            dimensions: to_vec2(self.atlas_allocator.size()),
            texture: textures.add(self.texture),
            textures: texture_rects,
            texture_handles: Some(texture_handles),
        }
    }
}

impl From<guillotiere::Rectangle> for Rect {
    fn from(rectangle: guillotiere::Rectangle) -> Self {
        Rect {
            min: Vec2::new(rectangle.min.x as f32, rectangle.min.y as f32),
            max: Vec2::new(rectangle.max.x as f32, rectangle.max.y as f32),
        }
    }
}

fn to_vec2(size: guillotiere::Size) -> Vec2 {
    Vec2::new(size.width as f32, size.height as f32)
}
    