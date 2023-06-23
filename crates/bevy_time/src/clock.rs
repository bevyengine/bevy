use bevy_reflect::{FromReflect, Reflect};
use bevy_utils::Duration;

#[derive(Default, Debug, Copy, Clone, Reflect, FromReflect)]
pub struct Clock {
    pub wrap_seconds: u64,
    pub delta: Duration,
    pub delta_seconds: f32,
    pub delta_seconds_f64: f64,
    pub elapsed: Duration,
    pub elapsed_seconds: f32,
    pub elapsed_seconds_f64: f64,
    pub elapsed_wrapped: Duration,
    pub elapsed_seconds_wrapped: f32,
    pub elapsed_seconds_wrapped_f64: f64,
}

impl Clock {
    pub fn new(wrap_seconds: u64) -> Clock {
        Clock {
            wrap_seconds,
            ..Default::default()
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
}
