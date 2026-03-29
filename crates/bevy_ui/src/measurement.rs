use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::FontCx;
use core::fmt::Formatter;
pub use taffy::style::AvailableSpace;
use taffy::MaybeMath;
use taffy::MaybeResolve;

use crate::widget::ImageMeasure;

use crate::widget::TextMeasure;

impl core::fmt::Debug for ContentSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ContentSize").finish()
    }
}

/// Inputs provided to [`Measure::measure`].
pub struct MeasureArgs<'a> {
    /// The known width from Taffy if this size is definite.
    pub known_width: Option<f32>,
    /// The known height from Taffy if this size is definite.
    pub known_height: Option<f32>,
    /// The horizontal space available for this UI node.
    pub available_width: AvailableSpace,
    /// The vertical space available for this UI node.
    pub available_height: AvailableSpace,
    /// Parley font database, needed for text measurement.
    pub font_system: &'a mut FontCx,
    /// Text layout buffer used to compute intrinsic text size when required.
    pub buffer: Option<&'a mut bevy_text::ComputedTextBlock>,
    /// Resolved style for this node in physical pixels.
    pub style: &'a taffy::Style,
}

#[derive(Copy, Clone)]
/// Resolved values for per-axis size constraints.
pub struct ResolvedAxis {
    /// Resolved minimum size along this axis.
    pub min: Option<f32>,
    /// Resolved preferred size along this axis.
    pub preferred: Option<f32>,
    /// Resolved maximum size along this axis.
    pub max: Option<f32>,
    /// Effective size along this axis after applying known size and min/max clamping.
    pub effective: Option<f32>,
}

fn resolve_axis(
    known_size: Option<f32>,
    available_space: AvailableSpace,
    min_dim: taffy::style::Dimension,
    size_dim: taffy::style::Dimension,
    max_dim: taffy::style::Dimension,
) -> ResolvedAxis {
    let calc = |_, _| 0.;
    let available = available_space.into_option();
    let min = min_dim.maybe_resolve(available, calc);
    let preferred = size_dim.maybe_resolve(available, calc);
    let max = max_dim.maybe_resolve(available, calc);
    ResolvedAxis {
        min,
        preferred,
        max,
        effective: known_size.or(preferred.or(min).maybe_clamp(min, max)),
    }
}

impl MeasureArgs<'_> {
    /// Resolve the node's width constraints and the effective width.
    pub fn resolve_width(&self) -> ResolvedAxis {
        resolve_axis(
            self.known_width,
            self.available_width,
            self.style.min_size.width,
            self.style.size.width,
            self.style.max_size.width,
        )
    }

    /// Resolve the node's height constraints and the effective height.
    pub fn resolve_height(&self) -> ResolvedAxis {
        resolve_axis(
            self.known_height,
            self.available_height,
            self.style.min_size.height,
            self.style.size.height,
            self.style.max_size.height,
        )
    }
}

/// A `Measure` is used to compute the size of a ui node
/// when the size of that node is based on its content.
pub trait Measure: Send + Sync + 'static {
    /// Calculate the size of the node given the constraints.
    fn measure(&mut self, measure_args: MeasureArgs<'_>) -> Vec2;
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
    fn measure(&mut self, measure_args: MeasureArgs) -> Vec2 {
        match self {
            NodeMeasure::Fixed(fixed) => fixed.measure(measure_args),
            NodeMeasure::Text(text) => text.measure(measure_args),
            NodeMeasure::Image(image) => image.measure(measure_args),
            NodeMeasure::Custom(custom) => custom.measure(measure_args),
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
    fn measure(&mut self, _: MeasureArgs) -> Vec2 {
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

    /// Clear the current `Measure` for this UI node.
    pub fn clear(&mut self) {
        self.measure = None;
    }

    /// Creates a `ContentSize` with a `Measure` that always returns given `size` argument, regardless of the UI layout's constraints.
    pub fn fixed_size(size: Vec2) -> ContentSize {
        let mut content_size = Self::default();
        content_size.set(NodeMeasure::Fixed(FixedMeasure { size }));
        content_size
    }
}
