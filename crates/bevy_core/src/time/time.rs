use bevy_ecs::system::ResMut;
use bevy_utils::{Duration, Instant};

/// Tracks elapsed time since the last update and since the App has started
#[derive(Debug, Clone)]
pub struct Time {
    delta: Duration,
    last_update: Option<Instant>,
    delta_seconds_f64: f64,
    delta_seconds: f32,
    seconds_since_startup: f64,
    startup: Instant,
}

impl Default for Time {
    fn default() -> Time {
        Time {
            delta: Duration::from_secs(0),
            last_update: None,
            startup: Instant::now(),
            delta_seconds_f64: 0.0,
            seconds_since_startup: 0.0,
            delta_seconds: 0.0,
        }
    }
}

impl Time {
    pub fn update(&mut self) {
        let now = Instant::now();
        self.update_with_instant(now);
    }

    pub(crate) fn update_with_instant(&mut self, instant: Instant) {
        if let Some(last_update) = self.last_update {
            self.delta = instant - last_update;
            self.delta_seconds_f64 = self.delta.as_secs_f64();
            self.delta_seconds = self.delta.as_secs_f32();
        }

        let duration_since_startup = instant - self.startup;
        self.seconds_since_startup = duration_since_startup.as_secs_f64();
        self.last_update = Some(instant);
    }

    /// The delta between the current tick and last tick as a [`Duration`]
    #[inline]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// The delta between the current and last tick as [`f32`] seconds
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// The delta between the current and last tick as [`f64`] seconds
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.delta_seconds_f64
    }

    /// The time since startup in seconds
    #[inline]
    pub fn seconds_since_startup(&self) -> f64 {
        self.seconds_since_startup
    }

    /// The [`Instant`] the app was started
    #[inline]
    pub fn startup(&self) -> Instant {
        self.startup
    }

    /// The ['Instant'] when [`Time::update`] was last called, if it exists
    #[inline]
    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    pub fn time_since_startup(&self) -> Duration {
        Instant::now() - self.startup
    }
}

pub(crate) fn time_system(mut time: ResMut<Time>) {
    time.update();
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::Time;
    use bevy_utils::{Duration, Instant};

    #[test]
    fn update_test() {
        let start_instant = Instant::now();

        // Create a `Time` for testing
        let mut time = Time {
            startup: start_instant,
            ..Default::default()
        };

        // Ensure `time` was constructed correctly
        assert_eq!(time.delta(), Duration::from_secs(0));
        assert_eq!(time.last_update(), None);
        assert_eq!(time.startup(), start_instant);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.seconds_since_startup(), 0.0);
        assert_eq!(time.delta_seconds(), 0.0);

        // Update `time` and check results
        let first_update_instant = Instant::now();

        time.update_with_instant(first_update_instant);

        assert_eq!(time.delta(), Duration::from_secs(0));
        assert_eq!(time.last_update(), Some(first_update_instant));
        assert_eq!(time.startup(), start_instant);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(
            time.seconds_since_startup(),
            (first_update_instant - start_instant).as_secs_f64()
        );
        assert_eq!(time.delta_seconds, 0.0);

        // Update `time` again and check results
        let second_update_instant = Instant::now();

        time.update_with_instant(second_update_instant);

        assert_eq!(time.delta(), second_update_instant - first_update_instant);
        assert_eq!(time.last_update(), Some(second_update_instant));
        assert_eq!(time.startup(), start_instant);
        // At this point its safe to use time.delta as a valid value
        // because it's been previously verified to be correct
        assert_eq!(time.delta_seconds_f64(), time.delta().as_secs_f64());
        assert_eq!(
            time.seconds_since_startup(),
            (second_update_instant - start_instant).as_secs_f64()
        );
        assert_eq!(time.delta_seconds(), time.delta().as_secs_f32());
    }
}
