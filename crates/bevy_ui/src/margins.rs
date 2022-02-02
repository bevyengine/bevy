/// Defines the margins of a UI node
#[derive(Debug, Clone)]
pub struct Margins {
    /// Left margin size in pixels
    pub left: f32,
    /// Right margin size in pixels
    pub right: f32,
    /// Bottom margin size in pixels
    pub bottom: f32,
    /// Top margin size in pixels
    pub top: f32,
}

impl Margins {
    /// Creates a new Margins based on the input
    pub fn new(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Margins {
            left,
            right,
            bottom,
            top,
        }
    }
}

impl Default for Margins {
    fn default() -> Self {
        Margins {
            left: 0.0,
            right: 0.0,
            bottom: 0.0,
            top: 0.0,
        }
    }
}
