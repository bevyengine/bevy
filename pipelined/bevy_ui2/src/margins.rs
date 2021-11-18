#[derive(Debug, Clone)]
pub struct Margins {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

impl Margins {
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
