#[derive(Debug, Clone)]
pub struct Anchors {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

impl Anchors {
    pub fn new(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Anchors {
            left,
            right,
            bottom,
            top,
        }
    }
}

impl Default for Anchors {
    fn default() -> Self {
        Anchors {
            left: 0.0,
            right: 0.0,
            bottom: 0.0,
            top: 0.0,
        }
    }
}
