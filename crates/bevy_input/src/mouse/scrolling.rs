/// Unit of scroll
#[derive(Debug, Clone, Copy)]
pub enum MouseScrollUnit {
    Line,
    Pixel,
}

/// A mouse scroll wheel event, where x represents horizontal scroll and y represents vertical
/// scroll.
#[derive(Debug, Clone)]
pub struct MouseWheel {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
}
