#[derive(Debug, Clone)]
pub struct ButtonAxisSettings {
    pub high: f32,
    pub low: f32,
    pub threshold: f32,
}

impl Default for ButtonAxisSettings {
    fn default() -> Self {
        ButtonAxisSettings {
            high: 0.95,
            low: 0.05,
            threshold: 0.01,
        }
    }
}

impl ButtonAxisSettings {
    fn filter(&self, new_value: f32, old_value: Option<f32>) -> f32 {
        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return old_value;
            }
        }
        if new_value <= self.low {
            return 0.0;
        }
        if new_value >= self.high {
            return 1.0;
        }
        new_value
    }
}
