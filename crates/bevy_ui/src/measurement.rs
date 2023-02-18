use bevy_ecs::prelude::Component;
use bevy_math::Vec2;
use std::fmt::Formatter;
pub use taffy::layout::AvailableSpace;

/// The calculated size of the node
#[derive(Component)]
pub struct CalculatedSize {
    pub size: Vec2,
    /// The measure function used to calculate the size
    pub measure: Box<dyn Measure>,
}

impl Default for CalculatedSize {
    fn default() -> Self {
        Self {
            size: Default::default(),
            measure: Box::new(|w: Option<f32>, h: Option<f32>, _, _| {
                Vec2::new(w.unwrap_or(0.), h.unwrap_or(0.))
            }),
        }
    }
}

impl std::fmt::Debug for CalculatedSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalculatedSize")
            .field("size", &self.size)
            .finish()
    }
}

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

#[derive(Clone)]
pub struct BasicMeasure {
    /// Prefered size
    pub size: Vec2,
}

impl Measure for BasicMeasure {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        _: AvailableSpace,
        _: AvailableSpace,
    ) -> Vec2 {
        match (width, height) {
            (Some(width), Some(height)) => Vec2::new(width, height),
            (Some(width), None) => Vec2::new(width, self.size.y),
            (None, Some(height)) => Vec2::new(self.size.x, height),
            (None, None) => self.size,
        }
    }

    fn dyn_clone(&self) -> Box<dyn Measure> {
        Box::new(self.clone())
    }
}

impl<F> Measure for F
where
    F: Fn(Option<f32>, Option<f32>, AvailableSpace, AvailableSpace) -> Vec2
        + Send
        + Sync
        + 'static
        + Clone,
{
    fn measure(
        &self,
        max_width: Option<f32>,
        max_height: Option<f32>,
        available_width: AvailableSpace,
        available_height: AvailableSpace,
    ) -> Vec2 {
        self(max_width, max_height, available_width, available_height)
    }

    fn dyn_clone(&self) -> Box<dyn Measure> {
        Box::new(self.clone())
    }
}
