use bevy_math::{Rect, Vec2};
use bevy_reflect::{FromReflect, Reflect};

/// Struct defining a [`Sprite`](crate::Sprite) border with padding values
#[derive(Default, Clone, Copy, Debug, Reflect, FromReflect)]
pub struct BorderRect {
    /// Pixel padding to the left
    pub left: f32,
    /// Pixel padding to the right
    pub right: f32,
    /// Pixel padding to the top
    pub top: f32,
    /// Pixel padding to the bottom
    pub bottom: f32,
}

impl BorderRect {
    /// Creates a new border as a square, with identical pixel padding values on every direction
    #[must_use]
    #[inline]
    pub const fn square(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Creates a new border as a rectangle, with:
    /// - `horizontal` for left and right pixel padding
    /// - `vertical` for top and bottom pixel padding
    #[must_use]
    #[inline]
    pub const fn rectangle(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }
}

impl From<f32> for BorderRect {
    fn from(v: f32) -> Self {
        Self::square(v)
    }
}

impl From<[f32; 4]> for BorderRect {
    fn from([left, right, top, bottom]: [f32; 4]) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }
}

/// Slices a texture using the **9-slicing** technique. This allows to reuse an image at various sizes
/// without needing to prepare multiple assets. The associated texture will be split into nine portions,
/// so that on resize the different portions scale or tile in different ways to keep the texture in proportion.
///
/// For example, when resizing a 9-sliced texture the corners will remain unscaled while the other
/// sections will be scaled or tiled.
///
/// See [9-sliced](https://en.wikipedia.org/wiki/9-slice_scaling) textures.
#[derive(Debug, Clone, Reflect, FromReflect)]
pub struct TextureSlicer {
    /// The sprite borders, defining the 9 sections of the image
    pub border: BorderRect,
    /// Defines how the center part of the 9 slices will scale
    pub center_scale_mode: SliceScaleMode,
    /// Defines how the 4 side parts of the 9 slices will scale
    pub sides_scale_mode: SliceScaleMode,
    /// Defines the maximum scale of the 4 corner slices (default to `1.0`)
    pub max_corner_scale: f32,
}

/// Defines how a texture slice scales when resized
#[derive(Debug, Copy, Clone, Default, Reflect, FromReflect)]
pub enum SliceScaleMode {
    /// The slice will be stretched to fit the area
    #[default]
    Stretch,
    /// The slice will be tiled to fit the area
    Tile {
        /// The slice will repeat when the ratio between the *drawing dimensions* of texture and the
        /// *original texture size* are above `stretch_value`.
        ///
        /// Example: `1.0` means that a 10 pixel wide image would repeat after 10 screen pixels.
        /// `2.0` means it would repeat after 20 screen pixels.
        ///
        /// Note: The value should be inferior or equal to `1.0` to avoid quality loss.
        ///
        /// Note: the value will be clamped to `0.001` if lower
        stretch_value: f32,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct TextureSlice {
    /// texture area to draw
    pub texture_rect: Rect,
    /// slice draw size
    pub draw_size: Vec2,
    /// offset of the slice
    pub offset: Vec2,
}

impl TextureSlicer {
    /// Computes the 4 corner slices
    fn corner_slices(&self, base_rect: Rect, render_size: Vec2) -> [TextureSlice; 4] {
        let coef = render_size / base_rect.size();
        let min_coef = coef.x.min(coef.y).min(self.max_corner_scale);
        [
            // Top Left Corner
            TextureSlice {
                texture_rect: Rect {
                    min: base_rect.min,
                    max: base_rect.min + Vec2::new(self.border.left, self.border.top),
                },
                draw_size: Vec2::new(self.border.left, self.border.top) * min_coef,
                offset: Vec2::new(
                    -render_size.x + self.border.left * min_coef,
                    render_size.y - self.border.top * min_coef,
                ) / 2.0,
            },
            // Top Right Corner
            TextureSlice {
                texture_rect: Rect {
                    min: Vec2::new(base_rect.max.x - self.border.right, base_rect.min.y),
                    max: Vec2::new(base_rect.max.x, self.border.top),
                },
                draw_size: Vec2::new(self.border.right, self.border.top) * min_coef,
                offset: Vec2::new(
                    render_size.x - self.border.right * min_coef,
                    render_size.y - self.border.top * min_coef,
                ) / 2.0,
            },
            // Bottom Left
            TextureSlice {
                texture_rect: Rect {
                    min: Vec2::new(base_rect.min.x, base_rect.max.y - self.border.bottom),
                    max: Vec2::new(base_rect.min.x + self.border.left, base_rect.max.y),
                },
                draw_size: Vec2::new(self.border.left, self.border.bottom) * min_coef,
                offset: Vec2::new(
                    -render_size.x + self.border.left * min_coef,
                    -render_size.y + self.border.bottom * min_coef,
                ) / 2.0,
            },
            // Bottom Right Corner
            TextureSlice {
                texture_rect: Rect {
                    min: Vec2::new(
                        base_rect.max.x - self.border.right,
                        base_rect.max.y - self.border.bottom,
                    ),
                    max: base_rect.max,
                },
                draw_size: Vec2::new(self.border.right, self.border.bottom) * min_coef,
                offset: Vec2::new(
                    render_size.x - self.border.right * min_coef,
                    -render_size.y + self.border.bottom * min_coef,
                ) / 2.0,
            },
        ]
    }

    /// Computes the 2 horizontal side slices (left and right borders)
    fn horizontal_side_slices(
        &self,
        [tl_corner, tr_corner, bl_corner, br_corner]: &[TextureSlice; 4],
        base_rect: Rect,
        render_size: Vec2,
    ) -> [TextureSlice; 2] {
        // left
        let left_side = TextureSlice {
            texture_rect: Rect {
                min: base_rect.min + Vec2::new(0.0, self.border.top),
                max: Vec2::new(
                    base_rect.min.x + self.border.left,
                    base_rect.max.y - self.border.bottom,
                ),
            },
            draw_size: Vec2::new(
                bl_corner.draw_size.x,
                render_size.y - bl_corner.draw_size.y - tl_corner.draw_size.y,
            ),
            offset: Vec2::new(-render_size.x + bl_corner.draw_size.x, 0.0) / 2.0,
        };

        // right
        let right_side = TextureSlice {
            texture_rect: Rect {
                min: Vec2::new(
                    base_rect.max.x - self.border.right,
                    base_rect.min.y + self.border.bottom,
                ),
                max: Vec2::new(base_rect.max.x, base_rect.max.y - self.border.top),
            },
            draw_size: Vec2::new(
                br_corner.draw_size.x,
                render_size.y - (br_corner.draw_size.y + tr_corner.draw_size.y),
            ),
            offset: Vec2::new(render_size.x - br_corner.draw_size.x, 0.0) / 2.0,
        };
        [left_side, right_side]
    }

    /// Computes the 2 vertical side slices (top and bottom borders)
    fn vertical_side_slices(
        &self,
        [tl_corner, tr_corner, bl_corner, br_corner]: &[TextureSlice; 4],
        base_rect: Rect,
        render_size: Vec2,
    ) -> [TextureSlice; 2] {
        // Bottom
        let bot_side = TextureSlice {
            texture_rect: Rect {
                min: Vec2::new(
                    base_rect.min.x + self.border.left,
                    base_rect.max.y - self.border.bottom,
                ),
                max: Vec2::new(base_rect.max.x - self.border.right, base_rect.max.y),
            },
            draw_size: Vec2::new(
                render_size.x - (bl_corner.draw_size.x + br_corner.draw_size.x),
                bl_corner.draw_size.y,
            ),
            offset: Vec2::new(0.0, bl_corner.offset.y),
        };

        // Top
        let top_side = TextureSlice {
            texture_rect: Rect {
                min: base_rect.min + Vec2::new(self.border.left, 0.0),
                max: Vec2::new(
                    base_rect.max.x - self.border.right,
                    base_rect.min.y + self.border.top,
                ),
            },
            draw_size: Vec2::new(
                render_size.x - (tl_corner.draw_size.x + tr_corner.draw_size.x),
                tl_corner.draw_size.y,
            ),
            offset: Vec2::new(0.0, tl_corner.offset.y),
        };
        [bot_side, top_side]
    }

    /// Slices the given `rect` into at least 9 sections. If the center and/or side parts are set to tile,
    /// a bigger number of sections will be computed.
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the texture to slice in 9 parts
    /// * `render_size` - The optional draw size of the texture. If not set the `rect` size will be used.
    pub(crate) fn compute_slices(
        &self,
        rect: Rect,
        render_size: Option<Vec2>,
    ) -> Vec<TextureSlice> {
        let render_size = render_size.unwrap_or_else(|| rect.size());
        let mut slices = Vec::with_capacity(9);
        // Corners
        let corners = self.corner_slices(rect, render_size);
        // Sides
        let vertical_sides = self.vertical_side_slices(&corners, rect, render_size);
        let horizontal_sides = self.horizontal_side_slices(&corners, rect, render_size);
        // Center
        let center = TextureSlice {
            texture_rect: Rect {
                min: rect.min + Vec2::new(self.border.left, self.border.bottom),
                max: Vec2::new(rect.max.x - self.border.right, rect.max.y - self.border.top),
            },
            draw_size: Vec2::new(
                render_size.x - (corners[2].draw_size.x + corners[3].draw_size.x),
                render_size.y - (corners[2].draw_size.y + corners[0].draw_size.y),
            ),
            offset: Vec2::ZERO,
        };

        slices.extend(corners);
        match self.center_scale_mode {
            SliceScaleMode::Stretch => {
                slices.push(center);
            }
            SliceScaleMode::Tile { stretch_value } => {
                slices.extend(center.tiled(stretch_value, (true, true)));
            }
        }
        match self.sides_scale_mode {
            SliceScaleMode::Stretch => {
                slices.extend(horizontal_sides);
                slices.extend(vertical_sides);
            }
            SliceScaleMode::Tile { stretch_value } => {
                slices.extend(
                    horizontal_sides
                        .into_iter()
                        .flat_map(|s| s.tiled(stretch_value, (false, true))),
                );
                slices.extend(
                    vertical_sides
                        .into_iter()
                        .flat_map(|s| s.tiled(stretch_value, (true, false))),
                );
            }
        }
        slices
    }
}

impl TextureSlice {
    /// Transforms the given slice in an collection of tiled subdivisions.
    ///
    /// # Arguments
    ///
    /// * `stretch_value` - The slice will repeat when the ratio between the *drawing dimensions* of texture and the
    /// *original texture size* are above `stretch_value`.
    /// - `tile_x` - should the slice be tiled horizontally
    /// - `tile_y` - should the slice be tiled vertically
    pub fn tiled(self, stretch_value: f32, (tile_x, tile_y): (bool, bool)) -> Vec<Self> {
        if !tile_x && !tile_y {
            return vec![self];
        }
        let stretch_value = stretch_value.max(0.001);
        let rect_size = self.texture_rect.size();
        let expected_size = Vec2::new(
            if tile_x {
                rect_size.x * stretch_value
            } else {
                self.draw_size.x
            },
            if tile_y {
                rect_size.y * stretch_value
            } else {
                self.draw_size.y
            },
        );
        let mut slices = Vec::new();
        let base_offset = -self.draw_size / 2.0;
        let mut offset = base_offset;

        let mut remaining_columns = self.draw_size.y;
        while remaining_columns > 0.0 {
            let size_y = expected_size.y.min(remaining_columns);
            offset.x = base_offset.x;
            offset.y += size_y / 2.0;
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
            offset.y += size_y / 2.0;
            remaining_columns -= size_y;
        }
        slices
    }
}

impl Default for TextureSlicer {
    fn default() -> Self {
        Self {
            border: Default::default(),
            center_scale_mode: Default::default(),
            sides_scale_mode: Default::default(),
            max_corner_scale: 1.0,
        }
    }
}
