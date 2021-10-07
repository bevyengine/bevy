use crate::Rect;
use bevy_asset::Handle;
use bevy_math::Vec2;
use bevy_reflect::TypeUuid;
use bevy_render2::image::Image;
use bevy_utils::HashMap;

/// An atlas containing multiple images stored in regions on the source [`Image`].
///
/// Usize ids are used to reference a specific image region of the atlas. They can
/// be retrieved via the [`ImageAtlas::get_image_index`] methode.
/// [Example usage animating sprite.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// [Example usage loading sprite sheet.](https://github.com/bevyengine/bevy/blob/latest/examples/2d/image_atlas.rs)
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
pub struct ImageAtlas {
    // Todo: maybe find a better name
    /// The source image in which all images of the atlas are stored.
    pub source_image: Handle<Image>,
    // TODO: add support to Uniforms derive to write dimensions and sprites to the same buffer
    pub size: Vec2,
    /// The specific areas of the atlas where each image can be found.
    pub regions: Vec<Rect>,
    /// Mapping of image handles to the ids of the regions. This is useful for retrieving the
    /// region id from the handle to the original image or file path.
    pub image_handles: Option<HashMap<Handle<Image>, usize>>,
}

impl ImageAtlas {
    /// Create a new atlas from the `source_image`, that does not have any individual
    /// image `regions` specified.
    pub fn new_empty(source_image: Handle<Image>, size: Vec2) -> Self {
        Self {
            source_image,
            size,
            regions: Vec::new(),
            image_handles: None,
        }
    }

    /// Generate an atlas by splitting the `source_image` into a grid where each cell of the grid of
    /// `tile_size` is one of the image `regions` in the atlas.
    /// The region ids are assigned from left to right and top to bottom in a Z-pattern.
    pub fn from_grid(
        source_image: Handle<Image>,
        tile_size: Vec2,
        columns: usize,
        rows: usize,
    ) -> ImageAtlas {
        Self::from_grid_with_padding(
            source_image,
            tile_size,
            columns,
            rows,
            Vec2::new(0f32, 0f32),
        )
    }

    /// Generate an atlas by splitting the `source_image` into a grid where each cell of the grid of
    /// `tile_size` is one of the image `regions` in the atlas and is separated by some `padding`.
    /// The region ids are assigned from left to right and top to bottom in a Z-pattern.
    pub fn from_grid_with_padding(
        source_image: Handle<Image>,
        tile_size: Vec2,
        columns: usize,
        rows: usize,
        padding: Vec2,
    ) -> ImageAtlas {
        let mut regions = Vec::new();
        let mut x_padding = 0.0;
        let mut y_padding = 0.0;

        for y in 0..rows {
            if y > 0 {
                y_padding = padding.y;
            }
            for x in 0..columns {
                if x > 0 {
                    x_padding = padding.x;
                }

                let rect_min = Vec2::new(
                    (tile_size.x + x_padding) * x as f32,
                    (tile_size.y + y_padding) * y as f32,
                );

                regions.push(Rect {
                    min: rect_min,
                    max: Vec2::new(rect_min.x + tile_size.x, rect_min.y + tile_size.y),
                })
            }
        }

        ImageAtlas {
            source_image,
            size: Vec2::new(
                ((tile_size.x + x_padding) * columns as f32) - x_padding,
                ((tile_size.y + y_padding) * rows as f32) - y_padding,
            ),
            regions,
            image_handles: None,
        }
    }

    /// Adds a new image `region` to the atlas. This `region` spans the section of the
    /// `source_image` from the top-left corner of the image to the bottom-right corner.
    pub fn add_region(&mut self, region: Rect) {
        self.regions.push(region);
    }

    /// The number of image `regions` contained in the atlas.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Returns `true` if the atlas contains no image `regions`.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Returns the region index of the `image` inside the atlas.
    pub fn get_region_index(&self, image: &Handle<Image>) -> Option<usize> {
        self.image_handles
            .as_ref()
            .and_then(|image_handles| image_handles.get(image).cloned())
    }
}
