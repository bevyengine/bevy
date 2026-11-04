use super::BorderRect;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Slices a texture using the **9-slicing** technique. This allows to reuse an image at various sizes
/// without needing to prepare multiple assets. The associated texture will be split into nine portions,
/// so that on resize the different portions scale or tile in different ways to keep the texture in proportion.
///
/// For example, when resizing a 9-sliced texture the corners will remain unscaled while the other
/// sections will be scaled or tiled.
///
/// See [9-sliced](https://en.wikipedia.org/wiki/9-slice_scaling) textures.
#[derive(Debug, Clone, Reflect, PartialEq)]
#[reflect(Clone, PartialEq)]
pub struct TextureSlicer {
    /// Inset values in pixels that define the four slicing lines dividing the texture into nine sections.
    pub border: BorderRect,
    /// Defines how the center part of the 9 slices will scale
    pub center_scale_mode: SliceScaleMode,
    /// Defines how the 4 side parts of the 9 slices will scale
    pub sides_scale_mode: SliceScaleMode,
    /// Defines the maximum scale of the 4 corner slices (default to `1.0`)
    pub max_corner_scale: f32,
}

/// Defines how a texture slice scales when resized
#[derive(Debug, Copy, Clone, Default, Reflect, PartialEq)]
#[reflect(Clone, PartialEq, Default)]
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
