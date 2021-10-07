use crate::{ImageAtlas, Rect};
use bevy_asset::Assets;
use bevy_math::Vec2;
use bevy_render2::image::{Image, TextureFormatPixelInfo};
use guillotiere::{size2, Allocation, AtlasAllocator};

pub struct DynamicImageAtlasBuilder {
    pub atlas_allocator: AtlasAllocator,
    pub padding: i32,
}

impl DynamicImageAtlasBuilder {
    pub fn new(size: Vec2, padding: i32) -> Self {
        Self {
            atlas_allocator: AtlasAllocator::new(to_size2(size)),
            padding,
        }
    }

    pub fn add_image(
        &mut self,
        image_atlas: &mut ImageAtlas,
        images: &mut Assets<Image>,
        image: &Image,
    ) -> Option<u32> {
        let allocation = self.atlas_allocator.allocate(size2(
            image.texture_descriptor.size.width as i32 + self.padding,
            image.texture_descriptor.size.height as i32 + self.padding,
        ));
        if let Some(allocation) = allocation {
            let atlas_image = images.get_mut(&image_atlas.source_image).unwrap();
            self.place_image(atlas_image, allocation, image);
            let mut region: Rect = allocation.rectangle.into();
            region.max.x -= self.padding as f32;
            region.max.y -= self.padding as f32;
            image_atlas.add_region(region);
            Some((image_atlas.len() - 1) as u32)
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

    // Todo: duplicate of ImageAtlasBuilder::copy_image_to_atlas
    fn place_image(&mut self, atlas_image: &mut Image, allocation: Allocation, image: &Image) {
        let mut region = allocation.rectangle;
        region.max.x -= self.padding;
        region.max.y -= self.padding;
        let atlas_width = atlas_image.texture_descriptor.size.width as usize;
        let region_width = region.width() as usize;
        let pixel_size = atlas_image.texture_descriptor.format.pixel_size();

        for (texture_y, bound_y) in (region.min.y..region.max.y).map(|i| i as usize).enumerate() {
            let begin = (bound_y * atlas_width + region.min.x as usize) * pixel_size;
            let end = begin + region_width * pixel_size;
            let image_begin = texture_y * region_width * pixel_size;
            let image_end = image_begin + region_width * pixel_size;
            atlas_image.data[begin..end].copy_from_slice(&image.data[image_begin..image_end]);
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
