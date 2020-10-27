use core::default::Default;

#[derive(Debug, Clone)]
pub struct ButtonSettings {
    pub press: f32,
    pub release: f32,
}

impl Default for ButtonSettings {
    fn default() -> Self {
        ButtonSettings {
            press: 0.75,
            release: 0.65,
        }
    }
}

impl ButtonSettings {
    pub(crate) fn is_pressed(&self, value: f32) -> bool {
        value >= self.press
    }

    pub(crate) fn is_released(&self, value: f32) -> bool {
        value <= self.release
    }
}
