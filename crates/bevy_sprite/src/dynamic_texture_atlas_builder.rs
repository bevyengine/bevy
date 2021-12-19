use crate::{Rect, TextureAtlas};
use bevy_asset::Assets;
use bevy_math::Vec2;
use bevy_render::texture::{Image, TextureFormatPixelInfo};
use guillotiere::{size2, Allocation, AtlasAllocator};

pub struct DynamicTextureAtlasBuilder {
    pub atlas_allocator: AtlasAllocator,
    pub padding: i32,
}

impl DynamicTextureAtlasBuilder {
    pub fn new(size: Vec2, padding: i32) -> Self {
        Self {
            atlas_allocator: AtlasAllocator::new(to_size2(size)),
            padding,
        }
    }

    pub fn add_texture(
        &mut self,
        texture_atlas: &mut TextureAtlas,
        textures: &mut Assets<Image>,
        texture: &Image,
    ) -> Option<usize> {
        let allocation = self.atlas_allocator.allocate(size2(
            texture.texture_descriptor.size.width as i32 + self.padding,
            texture.texture_descriptor.size.height as i32 + self.padding,
        ));
        if let Some(allocation) = allocation {
            let atlas_texture = textures.get_mut(&texture_atlas.texture).unwrap();
            self.place_texture(atlas_texture, allocation, texture);
            let mut rect: Rect = allocation.rectangle.into();
            rect.max.x -= self.padding as f32;
            rect.max.y -= self.padding as f32;
            Some(texture_atlas.add_texture(rect))
        } else {
            None
        }
    }

    // fn resize(
    //     &mut self,
    //     texture_atlas: &mut TextureAtlas,
    //     textures: &mut Assets<Texture>,
    //     size: Vec2,
    // ) {
    //     let new_size2 = to_size2(new_size);
    //     self.atlas_texture = Texture::new_fill(new_size, &[0,0,0,0]);
    //     let change_list = self.atlas_allocator.resize_and_rearrange(new_size2);

    //     for change in change_list.changes {
    //         if let Some(changed_texture_handle) = self.allocation_textures.remove(&change.old.id)
    // {             let changed_texture = textures.get(&changed_texture_handle).unwrap();
    //             self.place_texture(change.new, changed_texture_handle, changed_texture);
    //         }
    //     }

    //     for failure in change_list.failures {
    //         let failed_texture = self.allocation_textures.remove(&failure.id).unwrap();
    //         queued_textures.push(failed_texture);
    //     }
    // }

    fn place_texture(
        &mut self,
        atlas_texture: &mut Image,
        allocation: Allocation,
        texture: &Image,
    ) {
        let mut rect = allocation.rectangle;
        rect.max.x -= self.padding;
        rect.max.y -= self.padding;
        let atlas_width = atlas_texture.texture_descriptor.size.width as usize;
        let rect_width = rect.width() as usize;
        let format_size = atlas_texture.texture_descriptor.format.pixel_size();

        for (texture_y, bound_y) in (rect.min.y..rect.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + rect.min.x as usize) * format_size;
            let end = begin + rect_width * format_size;
            let texture_begin = texture_y * rect_width * format_size;
            let texture_end = texture_begin + rect_width * format_size;
            atlas_texture.data[begin..end]
                .copy_from_slice(&texture.data[texture_begin..texture_end]);
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

fn to_size2(vec2: Vec2) -> guillotiere::Size {
    guillotiere::Size::new(vec2.x as i32, vec2.y as i32)
}
