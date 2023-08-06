use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::{Vec2, Vec2Swizzles};
use bevy_reflect::Reflect;
use std::fmt::Formatter;
pub use taffy::style::AvailableSpace;
use taffy::{node::MeasureFunc, prelude::Size as TaffySize};

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

/// An adapter implementing measure and wrapping another measure. Its `measure` method calls the `measure` method of `inner_measure` with swapped width and height values.
/// Then the components of the returned `Vec2` are swapped back before it is returned to the caller.
#[derive(Default, Clone)]
pub struct SwappedDimensionsMeasure<M: Measure> {
    /// The wrapped measure. It is called with swapped width and height values, and then the value it returns is swapped back.
    pub inner_measure: M,
}

impl<M: Measure> Measure for SwappedDimensionsMeasure<M> {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        available_width: AvailableSpace,
        available_height: AvailableSpace,
    ) -> Vec2 {
        self.inner_measure
            .measure(height, width, available_height, available_width)
            .yx()
    }
}

/// A node with a `ContentSize` component is a node where its size
/// is based on its content.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ContentSize {
    /// The `Measure` used to compute the intrinsic size
    #[reflect(ignore)]
    pub(crate) measure_func: Option<MeasureFunc>,
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
}

impl Default for ContentSize {
    fn default() -> Self {
        Self {
            measure_func: Some(MeasureFunc::Raw(|_, _| TaffySize::ZERO)),
        }
    }
}
