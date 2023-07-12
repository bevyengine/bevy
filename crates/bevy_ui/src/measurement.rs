use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_text::TextMeasureInfo;
pub use taffy::style::AvailableSpace;

/// The content-inherent size data.
///
/// In bevy UI, text and images may control their own size, outside of
/// the taffy-based layout algorithm.
///
/// For example, this keeps track of image pixel sizes (adjusted for UI scale)
/// and text size.
///
/// It is set in `measure_text_system` and `update_image_content_size_system`.
/// It is read by `ui_layout_system`.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub enum ContentSize {
    Image { size: Vec2 },
    Text(#[reflect(ignore)] TextMeasureInfo),
    Fixed { size: Vec2 },
}

impl ContentSize {
    /// Take ownership of the `measure`, leaving behind the default
    //// [`BevyUiMeasure`] of `BevyUiMeasure::Fixed { size: Vec2::ZERO }`.
    pub fn take(&mut self) -> Option<ContentSize> {
        if self.is_default() {
            None
        } else {
            Some(std::mem::take(self))
        }
    }
    pub fn is_default(&self) -> bool {
        matches!(
            self ,
            ContentSize::Fixed { size } if size == &Vec2::ZERO
        )
    }
    /// Compute the actual size of this `ContentSize` given the `width` and
    /// `height` optionaly set bounds and the provided available space.
    pub fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        available_width: AvailableSpace,
        _available_height: AvailableSpace,
    ) -> Vec2 {
        use AvailableSpace::{Definite, MaxContent, MinContent};
        match self {
            ContentSize::Image { size } => match (width, height) {
                (None, None) => *size,
                (Some(width), None) => Vec2::new(width, size.y + width * size.y / size.x),
                (None, Some(height)) => Vec2::new(height * size.x / size.y, height),
                (Some(width), Some(height)) => Vec2::new(width, height),
            },
            ContentSize::Text(text) => {
                let computed_width = match available_width {
                    Definite(x) => x.clamp(text.min.x, text.max.x),
                    MinContent => text.min.x,
                    MaxContent => text.max.x,
                };
                let width = width.unwrap_or(computed_width);
                let compute_height = || match available_width {
                    Definite(_) => text.compute_size(Vec2::new(width, f32::MAX)).y,
                    MinContent => text.min.y,
                    MaxContent => text.max.y,
                };
                Vec2::new(width, height.unwrap_or_else(compute_height))
            }
            ContentSize::Fixed { size } => *size,
        }
    }
}

impl Default for ContentSize {
    fn default() -> Self {
        ContentSize::Fixed { size: Vec2::ZERO }
    }
}
