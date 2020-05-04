#[derive(Debug, Clone)]
pub struct Anchors {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

impl Anchors {
    pub const CENTER: Anchors = Anchors::new(0.5, 0.5, 0.5, 0.5);
    pub const CENTER_LEFT: Anchors = Anchors::new(0.0, 0.0, 0.5, 0.5);
    pub const CENTER_RIGHT: Anchors = Anchors::new(1.0, 1.0, 0.5, 0.5);
    pub const CENTER_TOP: Anchors = Anchors::new(0.5, 0.5, 1.0, 1.0);
    pub const CENTER_BOTTOM: Anchors = Anchors::new(0.5, 0.5, 0.0, 0.0);
    pub const CENTER_FULL_VERTICAL: Anchors = Anchors::new(0.5, 0.5, 0.0, 1.0);
    pub const CENTER_FULL_HORIZONTAL: Anchors = Anchors::new(0.0, 1.0, 0.5, 0.5);
    pub const LEFT_FULL: Anchors = Anchors::new(0.0, 0.0, 0.0, 1.0);
    pub const RIGHT_FULL: Anchors = Anchors::new(1.0, 1.0, 0.0, 1.0);
    pub const TOP_FULL: Anchors = Anchors::new(0.0, 1.0, 1.0, 1.0);
    pub const BOTTOM_FULL: Anchors = Anchors::new(0.0, 1.0, 0.0, 0.0);
    pub const BOTTOM_LEFT: Anchors = Anchors::new(0.0, 0.0, 0.0, 0.0);
    pub const BOTTOM_RIGHT: Anchors = Anchors::new(1.0, 1.0, 0.0, 0.0);
    pub const TOP_RIGHT: Anchors = Anchors::new(1.0, 1.0, 1.0, 1.0);
    pub const TOP_LEFT: Anchors = Anchors::new(0.0, 0.0, 1.0, 1.0);
    pub const FULL: Anchors = Anchors::new(0.0, 1.0, 0.0, 1.0);

    pub const fn new(left: f32, right: f32, bottom: f32, top: f32) -> Self {
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
