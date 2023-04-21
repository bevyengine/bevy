use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use std::fmt::Formatter;
use taffy::node::Measurable;
pub use taffy::style::AvailableSpace;

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

/// A node with a `ContentSize` component is a node where its size
/// is based on its content.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ContentSize {
    /// The `Measure` used to compute the intrinsic size
    #[reflect(ignore)]
    pub(crate) measure_func: Option<taffy::node::MeasureFunc>,
}

impl ContentSize {
    /// Set a `Measure` for this function
    pub fn set(&mut self, measure: impl Measure) {
        let measure_func =
            move |size: taffy::prelude::Size<Option<f32>>,
                  available: taffy::prelude::Size<AvailableSpace>| {
                let size =
                    measure.measure(size.width, size.height, available.width, available.height);
                taffy::prelude::Size {
                    width: size.x,
                    height: size.y,
                }
            };
        self.measure_func = Some(taffy::node::MeasureFunc::Boxed(Box::new(measure_func)));
    }

    /// Use an unboxed function to compute the size of this content
    pub fn set_fn(
        &mut self,
        measure: impl Fn(Option<f32>, Option<f32>, AvailableSpace, AvailableSpace) -> Vec2,
    ) {
        self.measure_func = Some(taffy::node::MeasureFunc::Raw(measure));
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ContentSize {
    fn default() -> Self {
        Self {
            measure_func: Some(taffy::node::MeasureFunc::Raw(|_, _, _, _| Size::ZERO)),
        }
    }
}
