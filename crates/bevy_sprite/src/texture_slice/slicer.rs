use core::panic;

use super::{BorderRect, TextureSlice};
use bevy_math::{vec2, Rect, Vec2};
use bevy_reflect::Reflect;

/// Slices a texture using the **9-slicing** technique. This allows to reuse an image at various sizes
/// without needing to prepare multiple assets. The associated texture will be split into nine portions,
/// so that on resize the different portions scale or tile in different ways to keep the texture in proportion.
///
/// For example, when resizing a 9-sliced texture the corners will remain unscaled while the other
/// sections will be scaled or tiled.
///
/// See [9-sliced](https://en.wikipedia.org/wiki/9-slice_scaling) textures.
#[derive(Debug, Clone, Reflect)]
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
#[derive(Debug, Copy, Clone, Default, Reflect)]
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

impl TextureSlicer {
    /// Computes the 4 corner slices
    #[must_use]
    fn corner_slices(&self, base_rect: Rect, render_size: Vec2) -> [TextureSlice; 4] {
        let coef = render_size / base_rect.size();
        let BorderRect {
            left,
            right,
            top,
            bottom,
        } = self.border;
        let min_coef = coef.x.min(coef.y).min(self.max_corner_scale);
        [
            // Top Left Corner
            TextureSlice {
                texture_rect: Rect {
                    min: base_rect.min,
                    max: base_rect.min + vec2(left, top),
                },
                draw_size: vec2(left, top) * min_coef,
                offset: vec2(
                    -render_size.x + left * min_coef,
                    render_size.y - top * min_coef,
                ) / 2.0,
            },
            // Top Right Corner
            TextureSlice {
                texture_rect: Rect {
                    min: vec2(base_rect.max.x - right, base_rect.min.y),
                    max: vec2(base_rect.max.x, top),
                },
                draw_size: vec2(right, top) * min_coef,
                offset: vec2(
                    render_size.x - right * min_coef,
                    render_size.y - top * min_coef,
                ) / 2.0,
            },
            // Bottom Left
            TextureSlice {
                texture_rect: Rect {
                    min: vec2(base_rect.min.x, base_rect.max.y - bottom),
                    max: vec2(base_rect.min.x + left, base_rect.max.y),
                },
                draw_size: vec2(left, bottom) * min_coef,
                offset: vec2(
                    -render_size.x + left * min_coef,
                    -render_size.y + bottom * min_coef,
                ) / 2.0,
            },
            // Bottom Right Corner
            TextureSlice {
                texture_rect: Rect {
                    min: vec2(base_rect.max.x - right, base_rect.max.y - bottom),
                    max: base_rect.max,
                },
                draw_size: vec2(right, bottom) * min_coef,
                offset: vec2(
                    render_size.x - right * min_coef,
                    -render_size.y + bottom * min_coef,
                ) / 2.0,
            },
        ]
    }

    /// Computes the 2 horizontal side slices (left and right borders)
    #[must_use]
    fn horizontal_side_slices(
        &self,
        [tl_corner, tr_corner, bl_corner, br_corner]: &[TextureSlice; 4],
        base_rect: Rect,
        render_size: Vec2,
    ) -> [TextureSlice; 2] {
        [
            // left
            TextureSlice {
                texture_rect: Rect {
                    min: base_rect.min + vec2(0.0, self.border.top),
                    max: vec2(
                        base_rect.min.x + self.border.left,
                        base_rect.max.y - self.border.bottom,
                    ),
                },
                draw_size: vec2(
                    bl_corner.draw_size.x,
                    render_size.y - bl_corner.draw_size.y - tl_corner.draw_size.y,
                ),
                offset: vec2(-render_size.x + bl_corner.draw_size.x, 0.0) / 2.0,
            },
            // right
            TextureSlice {
                texture_rect: Rect {
                    min: vec2(
                        base_rect.max.x - self.border.right,
                        base_rect.min.y + self.border.bottom,
                    ),
                    max: vec2(base_rect.max.x, base_rect.max.y - self.border.top),
                },
                draw_size: vec2(
                    br_corner.draw_size.x,
                    render_size.y - (br_corner.draw_size.y + tr_corner.draw_size.y),
                ),
                offset: vec2(render_size.x - br_corner.draw_size.x, 0.0) / 2.0,
            },
        ]
    }

    /// Computes the 2 vertical side slices (bottom and top borders)
    #[must_use]
    fn vertical_side_slices(
        &self,
        [tl_corner, tr_corner, bl_corner, br_corner]: &[TextureSlice; 4],
        base_rect: Rect,
        render_size: Vec2,
    ) -> [TextureSlice; 2] {
        [
            // Bottom
            TextureSlice {
                texture_rect: Rect {
                    min: vec2(
                        base_rect.min.x + self.border.left,
                        base_rect.max.y - self.border.bottom,
                    ),
                    max: vec2(base_rect.max.x - self.border.right, base_rect.max.y),
                },
                draw_size: vec2(
                    render_size.x - (bl_corner.draw_size.x + br_corner.draw_size.x),
                    bl_corner.draw_size.y,
                ),
                offset: vec2(0.0, bl_corner.offset.y),
            },
            // Top
            TextureSlice {
                texture_rect: Rect {
                    min: base_rect.min + vec2(self.border.left, 0.0),
                    max: vec2(
                        base_rect.max.x - self.border.right,
                        base_rect.min.y + self.border.top,
                    ),
                },
                draw_size: vec2(
                    render_size.x - (tl_corner.draw_size.x + tr_corner.draw_size.x),
                    tl_corner.draw_size.y,
                ),
                offset: vec2(0.0, tl_corner.offset.y),
            },
        ]
    }

    /// Slices the given `rect` into at least 9 sections. If the center and/or side parts are set to tile,
    /// a bigger number of sections will be computed.
    ///
    /// # Arguments
    ///
    /// * `rect` - The section of the texture to slice in 9 parts
    /// * `render_size` - The optional draw size of the texture. If not set the `rect` size will be used.
    ///
    /// # Panics
    ///
    /// Panics if any border values are bigger than `rect` half extents
    #[must_use]
    pub fn compute_slices(&self, rect: Rect, render_size: Option<Vec2>) -> Vec<TextureSlice> {
        let render_size = render_size.unwrap_or_else(|| rect.size());
        let rect_size = rect.size() / 2.0;
        if self.border.left >= rect_size.x
            || self.border.right >= rect_size.x
            || self.border.top >= rect_size.y
            || self.border.bottom >= rect_size.y
        {
            bevy_log::error!(
                "TextureSlicer::border has out of bounds values. No slicing will be applied"
            );
            return vec![TextureSlice {
                texture_rect: rect,
                draw_size: render_size,
                offset: Vec2::ZERO,
            }];
        }
        let mut slices = Vec::with_capacity(9);
        // Corners
        let corners = self.corner_slices(rect, render_size);
        // Sides
        let vertical_sides = self.vertical_side_slices(&corners, rect, render_size);
        let horizontal_sides = self.horizontal_side_slices(&corners, rect, render_size);
        // Center
        let center = TextureSlice {
            texture_rect: Rect {
                min: rect.min + vec2(self.border.left, self.border.bottom),
                max: vec2(rect.max.x - self.border.right, rect.max.y - self.border.top),
            },
            draw_size: vec2(
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
