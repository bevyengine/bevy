mod border_rect;
mod computed_slices;
mod slicer;

use bevy_math::{Rect, Vec2};
pub use border_rect::BorderRect;
pub use slicer::{SliceScaleMode, TextureSlicer};

pub(crate) use computed_slices::{
    compute_slices_on_asset_event, compute_slices_on_sprite_change, ComputedTextureSlices,
};

/// Single texture slice, representing a texture rect to draw in a given area
#[derive(Debug, Clone)]
pub struct TextureSlice {
    /// texture area to draw
    pub texture_rect: Rect,
    /// slice draw size
    pub draw_size: Vec2,
    /// offset of the slice
    pub offset: Vec2,
}

impl TextureSlice {
    /// Transforms the given slice in an collection of tiled subdivisions.
    ///
    /// # Arguments
    ///
    /// * `stretch_value` - The slice will repeat when the ratio between the *drawing dimensions* of texture and the
    /// *original texture size* (rect) are above `stretch_value`.
    /// - `tile_x` - should the slice be tiled horizontally
    /// - `tile_y` - should the slice be tiled vertically
    #[must_use]
    pub fn tiled(self, stretch_value: f32, (tile_x, tile_y): (bool, bool)) -> Vec<Self> {
        if !tile_x && !tile_y {
            return vec![self];
        }
        let stretch_value = stretch_value.max(0.001);
        let rect_size = self.texture_rect.size();
        // Each tile expected size
        let expected_size = Vec2::new(
            if tile_x {
                // No slice should be less than 1 pixel wide
                (rect_size.x * stretch_value).max(1.0)
            } else {
                self.draw_size.x
            },
            if tile_y {
                // No slice should be less than 1 pixel high
                (rect_size.y * stretch_value).max(1.0)
            } else {
                self.draw_size.y
            },
        )
        .min(self.draw_size);
        let mut slices = Vec::new();
        let base_offset = Vec2::new(
            -self.draw_size.x / 2.0,
            self.draw_size.y / 2.0, // Start from top
        );
        let mut offset = base_offset;

        let mut remaining_columns = self.draw_size.y;
        while remaining_columns > 0.0 {
            let size_y = expected_size.y.min(remaining_columns);
            offset.x = base_offset.x;
            offset.y -= size_y / 2.0;
            let mut remaining_rows = self.draw_size.x;
            while remaining_rows > 0.0 {
                let size_x = expected_size.x.min(remaining_rows);
                offset.x += size_x / 2.0;
                let draw_size = Vec2::new(size_x, size_y);
                let delta = draw_size / expected_size;
                slices.push(Self {
                    texture_rect: Rect {
                        min: self.texture_rect.min,
                        max: self.texture_rect.min + self.texture_rect.size() * delta,
                    },
                    draw_size,
                    offset: self.offset + offset,
                });
                offset.x += size_x / 2.0;
                remaining_rows -= size_x;
            }
            offset.y -= size_y / 2.0;
            remaining_columns -= size_y;
        }
        if slices.len() > 1_000 {
            bevy_log::warn!("One of your tiled textures has generated {} slices. You might want to use higher stretch values to avoid a great performance cost", slices.len());
        }
        slices
    }
}
