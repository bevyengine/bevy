use crate::{Rect, TextureAtlas};
use bevy_asset::{Assets, Handle};
use bevy_render::texture::Texture;
use glam::Vec2;
use guillotiere::{size2, AllocId, Allocation, AtlasAllocator};
use std::collections::HashMap;

pub struct TextureAtlasBuilder {
    pub texture_allocations: HashMap<Handle<Texture>, Allocation>,
    pub allocation_textures: HashMap<AllocId, Handle<Texture>>,
    pub atlas_allocator: AtlasAllocator,
    pub atlas_texture: Texture,
    pub max_size: Vec2,
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self::new(Vec2::new(256., 256.), Vec2::new(2048., 2048.))
    }
}

const FORMAT_SIZE: usize = 4; // TODO: get this from an actual format type
impl TextureAtlasBuilder {
    pub fn new(initial_size: Vec2, max_size: Vec2) -> Self {
        Self {
            texture_allocations: Default::default(),
            allocation_textures: Default::default(),
            atlas_allocator: AtlasAllocator::new(to_size2(initial_size)),
            atlas_texture: Texture::new_fill(initial_size, &[0,0,0,0]),
            max_size,
        }
    }

    pub fn add_texture(&mut self, texture_handle: Handle<Texture>, textures: &Assets<Texture>) {
        let texture = textures.get(&texture_handle).unwrap();
        let mut queued_textures= vec![texture_handle];
        loop {
            let mut failed_textures = Vec::new();
            while let Some(texture_handle) = queued_textures.pop() {
                let allocation = self
                    .atlas_allocator
                    .allocate(size2(texture.size.x() as i32, texture.size.y() as i32));
                if let Some(allocation) = allocation {
                    self.place_texture(allocation, texture_handle, texture);
                } else {
                    failed_textures.push(texture_handle);
                }
            }

            if failed_textures.len() == 0 {
                break;
            }

            queued_textures = failed_textures;

            // if allocation failed, resize the atlas
            let new_size = self.atlas_texture.size * 2.0;
            if new_size > self.max_size {
                panic!(
                    "Ran out of space in Atlas. This atlas cannot be larger than: {:?}",
                    self.max_size
                );
            }

            let new_size2 = to_size2(new_size);
            self.atlas_texture = Texture::new_fill(new_size, &[0,0,0,0]);
            let change_list = self.atlas_allocator.resize_and_rearrange(new_size2);

            for change in change_list.changes {
                if let Some(changed_texture_handle) = self.allocation_textures.remove(&change.old.id) {
                    self.texture_allocations.remove(&changed_texture_handle);
                    let changed_texture = textures.get(&changed_texture_handle).unwrap();
                    self.place_texture(change.new, changed_texture_handle, changed_texture);
                }
            }

            for failure in change_list.failures {
                let failed_texture = self.allocation_textures.remove(&failure.id).unwrap();
                queued_textures.push(failed_texture);
            }
        }
    }

    fn place_texture(&mut self, allocation: Allocation, texture_handle: Handle<Texture>, texture: &Texture) {
        let rect = allocation.rectangle;
        let atlas_width = self.atlas_texture.size.x() as usize;
        let rect_width = rect.width() as usize;

        for (texture_y, bound_y) in (rect.min.y..rect.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + rect.min.x as usize) * FORMAT_SIZE;
            let end = begin + rect_width * FORMAT_SIZE;
            let texture_begin = texture_y * rect_width * FORMAT_SIZE;
            let texture_end = texture_begin + rect_width * FORMAT_SIZE;
            self.atlas_texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
        }

        self.allocation_textures.insert(allocation.id, texture_handle);
        self.texture_allocations.insert(texture_handle, allocation);
    }

    pub fn remove_texture(&mut self, texture_handle: Handle<Texture>) {
        if let Some(allocation) = self.texture_allocations.remove(&texture_handle) {
            self.allocation_textures.remove(&allocation.id);
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
            texture: textures.add(self.atlas_texture),
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

fn to_size2(vec2: Vec2) -> guillotiere::Size {
    guillotiere::Size::new(vec2.x() as i32, vec2.y() as i32)
}
