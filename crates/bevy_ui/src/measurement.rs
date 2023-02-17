use bevy_math::Vec2;
pub use taffy::layout::AvailableSpace;

pub trait MeasureNode: Send + Sync + 'static {
    /// Calculate the size of the node given the constraints.
    fn measure(
        &self,
        max_width: Option<f32>,
        max_height: Option<f32>,
        available_width: AvailableSpace,
        available_height: AvailableSpace,
    ) -> Vec2;
    /// Clone and box self.
    fn box_clone(&self) -> Box<dyn MeasureNode>;
}


#[derive(Clone)]
pub struct BasicMeasure {
    /// Prefered size
    pub size: Vec2,
}

impl MeasureNode for BasicMeasure {
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

    fn box_clone(&self) -> Box<dyn MeasureNode> {
        Box::new(self.clone())
    }
}
