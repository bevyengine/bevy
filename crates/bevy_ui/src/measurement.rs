use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use std::fmt::Formatter;
pub use taffy::style::AvailableSpace;
use taffy::{node::MeasureFunc, prelude::Size as TaffySize};

impl std::fmt::Debug for ContentSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentSize").finish()
    }
}

/// Determines if the size of a UI node should be rounded
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Reflect)]
pub enum SizeRounding {
    /// The node's size should be rounded after layout recomputation
    #[default]
    Enabled,
    /// The node's size should be not rounded after layout recomputation
    Disabled,
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
    pub(crate) measure_func: Option<MeasureFunc>,
    /// Determines if the size of the UI node with this `ContentSize` should be rounded after layout recomputation
    pub(crate) rounding: SizeRounding,
}

impl ContentSize {
    /// Set a `Measure` for the UI node entity with this component
    pub fn set(&mut self, measure: impl Measure) {
        let measure_func = move |size: TaffySize<_>, available: TaffySize<_>| {
            let size = measure.measure(size.width, size.height, available.width, available.height);
            TaffySize {
                width: size.x,
                height: size.y,
            }
        };
        self.measure_func = Some(MeasureFunc::Boxed(Box::new(measure_func)));
    }

    /// Set a `Measure` for the UI node entity with this component with size rounding disabled
    pub fn set_unrounded(&mut self, measure: impl Measure) {
        let measure_func = move |size: TaffySize<_>, available: TaffySize<_>| {
            let size = measure.measure(size.width, size.height, available.width, available.height);
            TaffySize {
                width: size.x,
                height: size.y,
            }
        };
        self.measure_func = Some(MeasureFunc::Boxed(Box::new(measure_func)));
        self.rounding = SizeRounding::Disabled;
    }

    /// Creates a `ContentSize` with a `Measure` that always returns given `size` argument, regardless of the UI layout's constraints.
    pub fn fixed_size(size: Vec2) -> ContentSize {
        let mut content_size = Self::default();
        content_size.set(FixedMeasure { size });
        content_size
    }
}
