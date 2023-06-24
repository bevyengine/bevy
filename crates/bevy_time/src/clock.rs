use bevy_reflect::{FromReflect, Reflect};
use bevy_utils::Duration;

#[derive(Debug, Copy, Clone, Reflect, FromReflect)]
pub struct Clock {
    wrap_seconds: u64,
    delta: Duration,
    delta_seconds: f32,
    delta_seconds_f64: f64,
    elapsed: Duration,
    elapsed_seconds: f32,
    elapsed_seconds_f64: f64,
    elapsed_wrapped: Duration,
    elapsed_seconds_wrapped: f32,
    elapsed_seconds_wrapped_f64: f64,
}

impl Clock {
    const DEFAULT_WRAP_SECONDS: u64 = 3600; // 1 hour

    pub fn new(wrap_seconds: u64) -> Clock {
        Clock {
            wrap_seconds,
            delta: Duration::ZERO,
            delta_seconds: 0.0,
            delta_seconds_f64: 0.0,
            elapsed: Duration::ZERO,
            elapsed_seconds: 0.0,
            elapsed_seconds_f64: 0.0,
            elapsed_wrapped: Duration::ZERO,
            elapsed_seconds_wrapped: 0.0,
            elapsed_seconds_wrapped_f64: 0.0,
        }
    }

    pub fn advance_by(&mut self, delta: Duration) {
        self.delta = delta;
        self.delta_seconds = self.delta.as_secs_f32();
        self.delta_seconds_f64 = self.delta.as_secs_f64();
        self.elapsed += delta;
        self.elapsed_seconds = self.elapsed.as_secs_f32();
        self.elapsed_seconds_f64 = self.elapsed.as_secs_f64();
        self.elapsed_wrapped = Duration::new(self.elapsed.as_secs() % self.wrap_seconds, self.elapsed.subsec_nanos());
        self.elapsed_seconds_wrapped = self.elapsed_wrapped.as_secs_f32();
        self.elapsed_seconds_wrapped_f64 = self.elapsed_wrapped.as_secs_f64();
    }

    pub fn advance_to(&mut self, elapsed: Duration) {
        self.advance_by(elapsed - self.elapsed)
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    pub fn delta_seconds_f64(&self) -> f64 {
        self.delta_seconds_f64
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    pub fn elapsed_seconds_f64(&self) -> f64 {
        self.elapsed_seconds_f64
    }

    pub fn elapsed_wrapped(&self) -> Duration {
        self.elapsed_wrapped
    }

    pub fn elapsed_seconds_wrapped(&self) -> f32 {
        self.elapsed_seconds_wrapped
    }

    pub fn elapsed_seconds_wrapped_f64(&self) -> f64 {
        self.elapsed_seconds_wrapped_f64
    }

    pub fn wrap_period(&self) -> Duration {
        Duration::from_secs(self.wrap_seconds)
    }

    pub fn set_wrap_period(&mut self, wrap_period: Duration) {
        assert!(!wrap_period.is_zero(), "division by zero");
        assert_eq!(wrap_period.subsec_nanos(), 0, "wrap period must be integral seconds");
        self.wrap_seconds = wrap_period.as_secs();
        self.elapsed_wrapped = Duration::new(self.elapsed.as_secs() % self.wrap_seconds, self.elapsed.subsec_nanos());
        self.elapsed_seconds_wrapped = self.elapsed_wrapped.as_secs_f32();
        self.elapsed_seconds_wrapped_f64 = self.elapsed_wrapped.as_secs_f64();        
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new(Self::DEFAULT_WRAP_SECONDS)
    }
}
