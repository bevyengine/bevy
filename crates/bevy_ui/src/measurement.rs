use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use std::fmt::Formatter;
pub use taffy::style::AvailableSpace;

use crate::widget::ImageMeasure;

#[cfg(feature = "bevy_text")]
use crate::widget::TextMeasure;

impl std::fmt::Debug for ContentSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentSize").finish()
    }
}

/// A `Measure` is used to compute the size of a ui node
/// when the size of that node is based on its content.
pub trait Measure: Send + Sync + 'static {
    /// Calculate the size of the node given the constraints.
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        available_width: AvailableSpace,
        available_height: AvailableSpace,
    ) -> Vec2;
}

pub enum NodeMeasure {
    Fixed(FixedMeasure),
    #[cfg(feature = "bevy_text")]
    Text(TextMeasure),
    Image(ImageMeasure),
    Custom(Box<dyn Measure>),
}

impl Measure for NodeMeasure {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        available_width: AvailableSpace,
        available_height: AvailableSpace,
    ) -> Vec2 {
        match self {
            NodeMeasure::Fixed(fixed) => {
                fixed.measure(width, height, available_width, available_height)
            }
            #[cfg(feature = "bevy_text")]
            NodeMeasure::Text(text) => {
                text.measure(width, height, available_width, available_height)
            }
            NodeMeasure::Image(image) => {
                image.measure(width, height, available_width, available_height)
            }
            NodeMeasure::Custom(custom) => {
                custom.measure(width, height, available_width, available_height)
            }
        }
    }
}

/// A `FixedMeasure` is a `Measure` that ignores all constraints and
/// always returns the same size.
#[derive(Default, Clone)]
pub struct FixedMeasure {
    pub size: Vec2,
}

impl Measure for FixedMeasure {
    fn measure(
        &self,
        _: Option<f32>,
        _: Option<f32>,
        _: AvailableSpace,
        _: AvailableSpace,
    ) -> Vec2 {
        self.size
    }
}

/// A node with a `ContentSize` component is a node where its size
/// is based on its content.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
pub struct ContentSize {
    /// The `Measure` used to compute the intrinsic size
    #[reflect(ignore)]
    pub(crate) measure: Option<NodeMeasure>,
}

impl ContentSize {
    /// Set a `Measure` for the UI node entity with this component
    pub fn set(&mut self, measure: NodeMeasure) {
        self.measure = Some(measure);
    }

    /// Creates a `ContentSize` with a `Measure` that always returns given `size` argument, regardless of the UI layout's constraints.
    pub fn fixed_size(size: Vec2) -> ContentSize {
        let mut content_size = Self::default();
        content_size.set(NodeMeasure::Fixed(FixedMeasure { size }));
        content_size
    }
}
