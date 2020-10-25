#[derive(Debug, Clone)]
pub struct AxisSettings {
    pub positive_high: f32,
    pub positive_low: f32,
    pub negative_high: f32,
    pub negative_low: f32,
    pub threshold: f32,
}

impl Default for AxisSettings {
    fn default() -> Self {
        AxisSettings {
            positive_high: 0.95,
            positive_low: 0.05,
            negative_high: -0.95,
            negative_low: -0.05,
            threshold: 0.01,
        }
    }
}

impl AxisSettings {
    fn filter(&self, new_value: f32, old_value: Option<f32>) -> f32 {
        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return old_value;
            }
        }
        if new_value <= self.positive_low && new_value >= self.negative_low {
            return 0.0;
        }
        if new_value >= self.positive_high {
            return 1.0;
        }
        if new_value <= self.negative_high {
            return -1.0;
        }
        new_value
    }
}
