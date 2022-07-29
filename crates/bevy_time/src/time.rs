use bevy_ecs::reflect::ReflectResource;
use bevy_reflect::Reflect;
use bevy_utils::{Duration, Instant};

/// Tracks how much time has advanced (and also the raw CPU time elapsed) since its previous update
/// and since the app was started.
#[derive(Reflect, Debug, Clone)]
#[reflect(Resource)]
pub struct Time {
    startup: Instant,
    first_update: Option<Instant>,
    last_update: Option<Instant>,
    relative_speed: f64, // using `f64` instead of `f32` to minimize drift from rounding errors
    delta: Duration,
    delta_seconds: f32,
    delta_seconds_f64: f64,
    elapsed_since_startup: Duration,
    seconds_since_startup: f32,
    seconds_since_startup_f64: f64,
    raw_delta: Duration,
    raw_delta_seconds: f32,
    raw_delta_seconds_f64: f64,
    raw_elapsed_since_startup: Duration,
    raw_seconds_since_startup: f32,
    raw_seconds_since_startup_f64: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            startup: Instant::now(),
            first_update: None,
            last_update: None,
            relative_speed: 1.0,
            delta: Duration::ZERO,
            delta_seconds: 0.0,
            delta_seconds_f64: 0.0,
            elapsed_since_startup: Duration::ZERO,
            seconds_since_startup: 0.0,
            seconds_since_startup_f64: 0.0,
            raw_delta: Duration::ZERO,
            raw_delta_seconds: 0.0,
            raw_delta_seconds_f64: 0.0,
            raw_elapsed_since_startup: Duration::ZERO,
            raw_seconds_since_startup: 0.0,
            raw_seconds_since_startup_f64: 0.0,
        }
    }
}

impl Time {
    /// Constructs a new `Time` instance with a specific startup `Instant`.
    pub fn new(startup: Instant) -> Self {
        Self {
            startup,
            ..Default::default()
        }
    }

    /// Updates the internal time measurements.
    pub fn update(&mut self) {
        let now = Instant::now();
        self.update_with_instant(now);
    }

    /// Updates time with a specified [`Instant`].
    ///
    /// This method is provided for use in tests. Calling this method in a normal app will result
    /// in inaccurate timekeeping, as the resource is ordinarily managed by the [`TimePlugin`](crate::TimePlugin).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_time::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_utils::Duration;
    /// # fn main () {
    /// #     test_health_system();
    /// # }
    /// struct Health {
    ///     // Health value between 0.0 and 1.0
    ///     health_value: f32,
    /// }
    ///
    /// fn health_system(time: Res<Time>, mut health: ResMut<Health>) {
    ///     // Increase health value by 0.1 per second, independent of frame rate,
    ///     // but not beyond 1.0
    ///     health.health_value = (health.health_value + 0.1 * time.delta_seconds()).min(1.0);
    /// }
    ///
    /// // Mock time in tests
    /// fn test_health_system() {
    ///     let mut world = World::default();
    ///     let mut time = Time::default();
    ///     time.update();
    ///     world.insert_resource(time);
    ///     world.insert_resource(Health { health_value: 0.2 });
    ///
    ///     let mut update_stage = SystemStage::single_threaded();
    ///     update_stage.add_system(health_system);
    ///
    ///     // Simulate that 30 ms have passed
    ///     let mut time = world.resource_mut::<Time>();
    ///     let last_update = time.last_update().unwrap();
    ///     time.update_with_instant(last_update + Duration::from_millis(30));
    ///
    ///     // Run system
    ///     update_stage.run(&mut world);
    ///
    ///     // Check that 0.003 has been added to the health value
    ///     let expected_health_value = 0.2 + 0.1 * 0.03;
    ///     let actual_health_value = world.resource::<Health>().health_value;
    ///     assert_eq!(expected_health_value, actual_health_value);
    /// }
    /// ```
    pub fn update_with_instant(&mut self, instant: Instant) {
        let raw_delta = if let Some(last_update) = self.last_update {
            instant - last_update
        } else {
            instant - self.startup
        };

        // Avoid rounding errors when the relative speed is 1.
        let delta = if self.relative_speed != 1.0 {
            raw_delta.mul_f64(self.relative_speed)
        } else {
            raw_delta
        };

        if self.last_update.is_some() {
            self.delta = delta;
            self.delta_seconds = self.delta.as_secs_f32();
            self.delta_seconds_f64 = self.delta.as_secs_f64();
            self.raw_delta = raw_delta;
            self.raw_delta_seconds = self.raw_delta.as_secs_f32();
            self.raw_delta_seconds_f64 = self.raw_delta.as_secs_f64();
        } else {
            self.first_update = Some(instant);
        }

        self.elapsed_since_startup += delta;
        self.seconds_since_startup = self.elapsed_since_startup.as_secs_f32();
        self.seconds_since_startup_f64 = self.elapsed_since_startup.as_secs_f64();
        self.raw_elapsed_since_startup += raw_delta;
        self.raw_seconds_since_startup = self.raw_elapsed_since_startup.as_secs_f32();
        self.raw_seconds_since_startup_f64 = self.raw_elapsed_since_startup.as_secs_f64();
        self.last_update = Some(instant);
    }

    /// Returns the [`Instant`] the app was started.
    #[inline]
    pub fn startup(&self) -> Instant {
        self.startup
    }

    /// Returns the [`Instant`] when [`update`](Self::update) was first called, if it exists.
    #[inline]
    pub fn first_update(&self) -> Option<Instant> {
        self.first_update
    }

    /// Returns the [`Instant`] when [`update`](Self::update) was last called, if it exists.
    #[inline]
    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    /// Returns the rate that time advances relative to raw CPU time, as [`f32`].
    ///
    /// `1.0` by default.
    #[inline]
    pub fn relative_speed(&self) -> f32 {
        self.relative_speed as f32
    }

    /// Returns the rate that time advances relative to raw CPU time, as [`f64`].
    ///
    /// `1.0` by default.
    #[inline]
    pub fn relative_speed_f64(&self) -> f64 {
        self.relative_speed
    }

    /// Sets the rate that time advances relative to raw CPU time, given as [`f32`].
    ///
    /// # Panics
    ///
    /// Panics if `ratio` is negative or not finite.
    pub fn set_relative_speed(&mut self, ratio: f32) {
        assert!(ratio.is_finite(), "tried to go infinitely fast");
        assert!(ratio.is_sign_positive(), "tried to go back in time");
        self.relative_speed = ratio as f64;
    }

    /// Sets the rate that time advances relative to raw CPU time, given as [`f64`].
    ///
    /// # Panics
    ///
    /// Panics if `ratio` is negative or not finite.
    pub fn set_relative_speed_f64(&mut self, ratio: f64) {
        assert!(ratio.is_finite(), "tried to go infinitely fast");
        assert!(ratio.is_sign_positive(), "tried to go back in time");
        self.relative_speed = ratio;
    }

    /// Returns how much time has advanced since the last [`update`](Self::update), as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Returns how much time has advanced since the last [`update`](Self::update), as [`f32`] seconds.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// Returns how much time has advanced since the last [`update`](Self::update), as [`f64`] seconds.
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.delta_seconds_f64
    }

    /// Returns the exact CPU time elapsed since the last [`update`](Self::update), as a [`Duration`].
    #[inline]
    pub fn raw_delta(&self) -> Duration {
        self.raw_delta
    }

    /// Returns the exact CPU time elapsed since the last [`update`](Self::update), as [`f32`] seconds.
    #[inline]
    pub fn raw_delta_seconds(&self) -> f32 {
        self.raw_delta_seconds
    }

    /// Returns the exact CPU time elapsed since the last [`update`](Self::update), as [`f64`] seconds.
    #[inline]
    pub fn raw_delta_seconds_f64(&self) -> f64 {
        self.raw_delta_seconds_f64
    }

    /// Returns how much time has advanced since [`startup`](Self::startup), as [`Duration`].
    #[inline]
    pub fn elapsed_since_startup(&self) -> Duration {
        self.elapsed_since_startup
    }

    /// Returns how much time has advanced since [`startup`](Self::startup), as [`f32`] seconds.
    #[inline]
    pub fn seconds_since_startup(&self) -> f32 {
        self.seconds_since_startup
    }

    /// Returns how much time has advanced since [`startup`](Self::startup), as [`f64`] seconds.
    #[inline]
    pub fn seconds_since_startup_f64(&self) -> f64 {
        self.seconds_since_startup_f64
    }

    /// Returns the exact CPU time elapsed since [`startup`](Self::startup), as [`Duration`].
    #[inline]
    pub fn raw_elapsed_since_startup(&self) -> Duration {
        self.raw_elapsed_since_startup
    }

    /// Returns the exact CPU time elapsed since [`startup`](Self::startup), as [`f32`] seconds.
    #[inline]
    pub fn raw_seconds_since_startup(&self) -> f32 {
        self.raw_seconds_since_startup
    }

    /// Returns the exact CPU time elapsed since [`startup`](Self::startup), as [`f64`] seconds.
    #[inline]
    pub fn raw_seconds_since_startup_f64(&self) -> f64 {
        self.raw_seconds_since_startup_f64
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::Time;
    use bevy_utils::{Duration, Instant};

    #[test]
    fn update_test() {
        // Create a `Time` for testing.
        let start_instant = Instant::now();
        let mut time = Time::new(start_instant);

        // Ensure `time` was constructed correctly.
        assert_eq!(time.startup(), start_instant);
        assert_eq!(time.first_update(), None);
        assert_eq!(time.last_update(), None);
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.raw_delta(), Duration::ZERO);
        assert_eq!(time.raw_delta_seconds(), 0.0);
        assert_eq!(time.raw_delta_seconds_f64(), 0.0);
        assert_eq!(time.elapsed_since_startup(), Duration::ZERO);
        assert_eq!(time.seconds_since_startup(), 0.0);
        assert_eq!(time.seconds_since_startup_f64(), 0.0);
        assert_eq!(time.raw_elapsed_since_startup(), Duration::ZERO);
        assert_eq!(time.raw_seconds_since_startup(), 0.0);
        assert_eq!(time.raw_seconds_since_startup_f64(), 0.0);

        // Update `time` and check results.
        // The first update to `time` normally happens before other systems have run,
        // so the first delta doesn't appear until the second update.
        let first_update_instant = Instant::now();
        time.update_with_instant(first_update_instant);

        assert_eq!(time.startup(), start_instant);
        assert_eq!(time.first_update(), Some(first_update_instant));
        assert_eq!(time.last_update(), Some(first_update_instant));
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.raw_delta(), Duration::ZERO);
        assert_eq!(time.raw_delta_seconds(), 0.0);
        assert_eq!(time.raw_delta_seconds_f64(), 0.0);
        assert_eq!(
            time.elapsed_since_startup(),
            first_update_instant - start_instant,
        );
        assert_eq!(
            time.seconds_since_startup(),
            (first_update_instant - start_instant).as_secs_f32(),
        );
        assert_eq!(
            time.seconds_since_startup_f64(),
            (first_update_instant - start_instant).as_secs_f64(),
        );
        assert_eq!(
            time.raw_elapsed_since_startup(),
            first_update_instant - start_instant,
        );
        assert_eq!(
            time.raw_seconds_since_startup(),
            (first_update_instant - start_instant).as_secs_f32(),
        );
        assert_eq!(
            time.raw_seconds_since_startup_f64(),
            (first_update_instant - start_instant).as_secs_f64(),
        );

        // Update `time` again and check results.
        // At this point its safe to use time.delta().
        let second_update_instant = Instant::now();
        time.update_with_instant(second_update_instant);
        assert_eq!(time.startup(), start_instant);
        assert_eq!(time.first_update(), Some(first_update_instant));
        assert_eq!(time.last_update(), Some(second_update_instant));
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), second_update_instant - first_update_instant);
        assert_eq!(
            time.delta_seconds(),
            (second_update_instant - first_update_instant).as_secs_f32(),
        );
        assert_eq!(
            time.delta_seconds_f64(),
            (second_update_instant - first_update_instant).as_secs_f64(),
        );
        assert_eq!(
            time.raw_delta(),
            second_update_instant - first_update_instant,
        );
        assert_eq!(
            time.raw_delta_seconds(),
            (second_update_instant - first_update_instant).as_secs_f32(),
        );
        assert_eq!(
            time.raw_delta_seconds_f64(),
            (second_update_instant - first_update_instant).as_secs_f64(),
        );
        assert_eq!(
            time.elapsed_since_startup(),
            second_update_instant - start_instant,
        );
        assert_eq!(
            time.seconds_since_startup(),
            (second_update_instant - start_instant).as_secs_f32(),
        );
        assert_eq!(
            time.seconds_since_startup_f64(),
            (second_update_instant - start_instant).as_secs_f64(),
        );
        assert_eq!(
            time.raw_elapsed_since_startup(),
            second_update_instant - start_instant,
        );
        assert_eq!(
            time.raw_seconds_since_startup(),
            (second_update_instant - start_instant).as_secs_f32(),
        );
        assert_eq!(
            time.raw_seconds_since_startup_f64(),
            (second_update_instant - start_instant).as_secs_f64(),
        );

        // Make app time advance at 2x the rate of the system clock.
        time.set_relative_speed(2.0);

        // Update `time` again 1 second later.
        let elapsed = Duration::from_secs(1);
        let third_update_instant = second_update_instant + elapsed;
        time.update_with_instant(third_update_instant);

        // Since app is advancing 2x the system clock, expect elapsed time
        // to have advanced by twice the amount of raw CPU time.
        assert_eq!(time.startup(), start_instant);
        assert_eq!(time.first_update(), Some(first_update_instant));
        assert_eq!(time.last_update(), Some(third_update_instant));
        assert_eq!(time.relative_speed(), 2.0);
        assert_eq!(time.delta(), elapsed.mul_f32(2.0));
        assert_eq!(time.delta_seconds(), elapsed.mul_f32(2.0).as_secs_f32());
        assert_eq!(time.delta_seconds_f64(), elapsed.mul_f32(2.0).as_secs_f64());
        assert_eq!(time.raw_delta(), elapsed);
        assert_eq!(time.raw_delta_seconds(), elapsed.as_secs_f32());
        assert_eq!(time.raw_delta_seconds_f64(), elapsed.as_secs_f64());
        assert_eq!(
            time.elapsed_since_startup(),
            second_update_instant - start_instant + elapsed.mul_f32(2.0),
        );
        assert_eq!(
            time.seconds_since_startup(),
            (second_update_instant - start_instant + elapsed.mul_f32(2.0)).as_secs_f32(),
        );
        assert_eq!(
            time.seconds_since_startup_f64(),
            (second_update_instant - start_instant + elapsed.mul_f32(2.0)).as_secs_f64(),
        );
        assert_eq!(
            time.raw_elapsed_since_startup(),
            second_update_instant - start_instant + elapsed,
        );
        assert_eq!(
            time.raw_seconds_since_startup(),
            (second_update_instant - start_instant + elapsed).as_secs_f32(),
        );
        assert_eq!(
            time.raw_seconds_since_startup_f64(),
            (second_update_instant - start_instant + elapsed).as_secs_f64(),
        );
    }
}
