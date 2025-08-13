use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::CosmicFontSystem;
use core::fmt::Formatter;
pub use taffy::style::AvailableSpace;

use crate::widget::ImageMeasure;

use crate::widget::TextMeasure;

impl core::fmt::Debug for ContentSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ContentSize").finish()
    }
}

pub struct MeasureArgs<'a> {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub available_width: AvailableSpace,
    pub available_height: AvailableSpace,
    pub font_system: &'a mut CosmicFontSystem,
    pub buffer: Option<&'a mut bevy_text::ComputedTextBlock>,
}

/// A `Measure` is used to compute the size of a ui node
/// when the size of that node is based on its content.
pub trait Measure: Send + Sync + 'static {
    /// Calculate the size of the node given the constraints.
    fn measure(&mut self, measure_args: MeasureArgs<'_>, style: &taffy::Style) -> Vec2;
}

/// A type to serve as Taffy's node context (which allows the content size of leaf nodes to be computed)
///
/// It has specific variants for common built-in types to avoid making them opaque and needing to box them
/// by wrapping them in a closure and a Custom variant that allows arbitrary measurement closures if required.
pub enum NodeMeasure {
    Fixed(FixedMeasure),

    Text(TextMeasure),
    Image(ImageMeasure),
    Custom(Box<dyn Measure>),
}

impl Measure for NodeMeasure {
    fn measure(&mut self, measure_args: MeasureArgs, style: &taffy::Style) -> Vec2 {
        match self {
            NodeMeasure::Fixed(fixed) => fixed.measure(measure_args, style),

            NodeMeasure::Text(text) => text.measure(measure_args, style),
            NodeMeasure::Image(image) => image.measure(measure_args, style),
            NodeMeasure::Custom(custom) => custom.measure(measure_args, style),
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
    fn measure(&mut self, _: MeasureArgs, _: &taffy::Style) -> Vec2 {
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
