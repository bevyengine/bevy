use bevy_ecs::prelude::Component;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use std::fmt::Formatter;
pub use taffy::style::AvailableSpace;

impl std::fmt::Debug for IntrinsicSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalculatedSize")
            .field("size", &self.size)
            .finish()
    }
}

/// A `Measure` is used to compute the size of a ui node
/// when the size of that node is based on its content.
pub trait Measure: Send + Sync + 'static {
    /// Calculate the size of the node given the constraints.
    fn measure(
        &self,
        max_width: Option<f32>,
        max_height: Option<f32>,
        available_width: AvailableSpace,
        available_height: AvailableSpace,
    ) -> Vec2;

    /// Clone and box self.
    fn dyn_clone(&self) -> Box<dyn Measure>;
}

/// A `FixedMeasure` is a `Measure` that ignores all constraints and
/// always returns the same size.
#[derive(Default, Clone)]
pub struct FixedMeasure {
    size: Vec2,
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

    fn dyn_clone(&self) -> Box<dyn Measure> {
        Box::new(self.clone())
    }
}

/// A node with an intrinsic size component is a node where its size
/// is based on its content.
#[derive(Component, Reflect)]
pub struct IntrinsicSize {
    /// Used to track changes
    pub size: Vec2,
    pub min_content: Vec2,
    pub max_content: Vec2,
    pub ideal: Vec2,
    /// The `Measure` used to compute the intrinsic size
    #[reflect(ignore)]
    pub measure: Box<dyn Measure>,
}

#[allow(clippy::derivable_impls)]
impl Default for IntrinsicSize {
    fn default() -> Self {
        Self {
            size: Default::default(),
            min_content: Default::default(),
            max_content: Default::default(),
            ideal: Default::default(),
            // Default `FixedMeasure` always returns zero size.
            measure: Box::<FixedMeasure>::default(),
        }
    }
}

impl Clone for IntrinsicSize {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            min_content: Default::default(),
            max_content: Default::default(),
            ideal: Default::default(),
            measure: self.measure.dyn_clone(),
        }
    }
}
