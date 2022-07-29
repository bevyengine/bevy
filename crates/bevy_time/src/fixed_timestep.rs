use crate::Time;

use bevy_ecs::world::{FromWorld, World};
use bevy_utils::{Duration, Instant};

/// A [`Time`] variant that is synchronized to the main `Time` resource, but only advances
/// in increments of a constant [`delta`](FixedTime::delta).
#[derive(Debug, Clone)]
pub struct FixedTime {
    startup: Instant,
    first_update: Option<Instant>,
    last_update: Option<Instant>,
    delta: Duration,
    delta_seconds: f32,
    delta_seconds_f64: f64,
    elapsed_since_startup: Duration,
    seconds_since_startup: f32,
    seconds_since_startup_f64: f64,
}

impl FromWorld for FixedTime {
    fn from_world(world: &mut World) -> Self {
        let time = world.resource::<Time>();
        Self {
            startup: time.startup(),
            first_update: None,
            last_update: None,
            delta: Self::DEFAULT_STEP_SIZE,
            delta_seconds: Self::DEFAULT_STEP_SIZE.as_secs_f32(),
            delta_seconds_f64: Self::DEFAULT_STEP_SIZE.as_secs_f64(),
            elapsed_since_startup: Duration::ZERO,
            seconds_since_startup: 0.0,
            seconds_since_startup_f64: 0.0,
        }
    }
}

impl FixedTime {
    /// The default step size.
    // 60Hz is a popular tick rate, but it can't be expressed as an exact float.
    // The nearby power of two, 64Hz, is more stable for numerical integration.
    pub const DEFAULT_STEP_SIZE: Duration = Duration::from_micros(15625); // 64Hz

    /// Constructs a new `FixedTime` instance with a specific step size [`Duration`] and startup [`Instant`].
    pub fn new(step_size: Duration, startup: Instant) -> Self {
        Self {
            startup,
            first_update: None,
            last_update: None,
            delta: step_size,
            delta_seconds: step_size.as_secs_f32(),
            delta_seconds_f64: step_size.as_secs_f64(),
            elapsed_since_startup: Duration::ZERO,
            seconds_since_startup: 0.0,
            seconds_since_startup_f64: 0.0,
        }
    }

    /// Updates internal time measurements.
    pub fn update(&mut self) {
        let now = Instant::now();
        self.update_with_instant(now);
    }

    /// Updates time with a specified [`Instant`].
    pub fn update_with_instant(&mut self, instant: Instant) {
        if self.last_update.is_none() {
            self.first_update = Some(instant);
        }
        self.last_update = Some(instant);
        self.elapsed_since_startup += self.delta;
        self.seconds_since_startup = self.elapsed_since_startup.as_secs_f32();
        self.seconds_since_startup_f64 = self.elapsed_since_startup.as_secs_f64();
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

    /// Returns how much time advances with each [`update`](Self::update), as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Returns how much time advances with each [`update`](Self::update), as [`f32`] seconds.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// Returns how much time advances with each [`update`](Self::update), as [`f64`] seconds.
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.delta_seconds_f64
    }

    /// Sets [`delta`](Self::delta) to the given step size ([`Duration`]).
    ///
    /// **Note:** Outside of startup, users should strongly prefer using [`Time::set_relative_speed`].
    /// Changing the step size itself will likely result in unstable numerical behavior.
    ///
    /// # Panics
    ///
    /// Panics if `step_size` is a zero-length duration.
    pub fn set_delta(&mut self, step_size: Duration) {
        assert!(!step_size.is_zero(), "division by zero");
        self.delta = step_size;
        self.delta_seconds = self.delta.as_secs_f32();
        self.delta_seconds_f64 = self.delta.as_secs_f64();
    }

    /// Sets [`delta`](Self::delta) to the given step size ([`f32`] seconds).
    ///
    /// **Note:** This should only be set *once* (i.e. at startup).
    /// Afterwards, only [`Time::set_relative_speed`] should be used to adjust simulation speed.
    /// Changing [`delta`](Self::delta) directly will likely result in unstable numerical behavior.
    ///
    /// # Panics
    ///
    /// Panics if `step_size` is less than or equal to zero, not finite, or overflows a `Duration`.
    pub fn set_delta_seconds(&mut self, step_size: f32) {
        self.set_delta(Duration::from_secs_f32(step_size));
    }

    /// Sets [`delta`](Self::delta) to the given step size ([`f64`] seconds).
    ///
    /// **Note:** This should only be set *once* (i.e. at startup).
    /// Afterwards, only [`Time::set_relative_speed`] should be used to adjust simulation speed.
    /// Changing [`delta`](Self::delta) directly will likely result in unstable numerical behavior.
    ///
    /// # Panics
    ///
    /// Panics if `step_size` is less than or equal to zero, not finite, or overflows a `Duration`.
    pub fn set_delta_seconds_f64(&mut self, step_size: f64) {
        self.set_delta(Duration::from_secs_f64(step_size));
    }

    /// Returns the nominal update rate (reciprocal of [`delta`](Self::delta)) as [`f32`].
    #[inline]
    pub fn steps_per_second(&self) -> f32 {
        1.0 / self.delta_seconds
    }

    /// Returns the nominal update rate (reciprocal of [`delta`](Self::delta)) as [`f64`].
    #[inline]
    pub fn steps_per_second_f64(&self) -> f64 {
        1.0 / self.delta_seconds_f64
    }

    /// Sets [`delta`](Self::delta) to the reciprocal of `rate`, given as [`f32`].
    ///
    /// **Note:** This should only be set *once* (i.e. at startup).
    /// Afterwards, only [`Time::set_relative_speed`] should be used to adjust simulation speed.
    /// Changing [`delta`](Self::delta) directly will likely result in unstable numerical behavior.
    ///
    /// # Panics
    ///
    /// Panics if `rate` is less than or equal to zero or not finite.
    pub fn set_steps_per_second(&mut self, rate: f32) {
        assert!(rate.is_finite(), "tried to go infinitely fast");
        assert!(rate.is_sign_positive(), "tried to go back in time");
        self.set_delta(Duration::from_secs_f32(1.0 / rate));
    }

    /// Sets [`delta`](Self::delta) to the reciprocal of `rate`, given as [`f64`].
    ///
    /// **Note:** This should only be set *once* (i.e. at startup).
    /// Afterwards, only [`Time::set_relative_speed`] should be used to adjust simulation speed.
    /// Changing [`delta`](Self::delta) directly will likely result in unstable numerical behavior.
    ///
    /// # Panics
    ///
    /// Panics if `rate` is less than or equal to zero or not finite.
    pub fn set_steps_per_second_f64(&mut self, rate: f64) {
        assert!(rate.is_finite(), "tried to go infinitely fast");
        assert!(rate.is_sign_positive(), "tried to go back in time");
        self.set_delta(Duration::from_secs_f64(1.0 / rate));
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
}

/// Accumulates time and converts it into steps: one step per `timestep`.
///
/// Used to advance [`FixedTime`].
#[derive(Debug, Clone)]
pub struct FixedTimestepState {
    steps: u32,
    overstep: Duration,
}

impl Default for FixedTimestepState {
    fn default() -> Self {
        Self {
            steps: 0,
            overstep: Duration::ZERO,
        }
    }
}

impl FixedTimestepState {
    /// Constructs a new `FixedTimestepState`.
    pub fn new(steps: u32, overstep: Duration) -> Self {
        Self { steps, overstep }
    }

    /// Returns the number of steps accumulated.
    #[inline]
    pub fn steps(&self) -> u32 {
        self.steps
    }

    /// Returns the amount of time accumulated toward new steps, as a [`Duration`].
    #[inline]
    pub fn overstep(&self) -> Duration {
        self.overstep
    }

    /// Returns the amount of time accumulated toward new steps, as an [`f32`] fraction of `timestep`.
    ///
    /// Useful for interpolating data between consecutive updates.
    ///
    /// # Panics
    ///
    /// Panics if `timestep` is a zero-length duration.
    pub fn overstep_percentage(&self, timestep: Duration) -> f32 {
        assert!(!timestep.is_zero(), "division by zero");
        self.overstep.as_secs_f32() / timestep.as_secs_f32()
    }

    /// Returns the amount of time accumulated toward new steps, as an [`f64`] fraction of `timestep`.
    ///
    /// Useful for interpolating data between consecutive updates.
    ///
    /// # Panics
    ///
    /// Panics if `timestep` is a zero-length duration.
    pub fn overstep_percentage_f64(&self, timestep: Duration) -> f64 {
        assert!(!timestep.is_zero(), "division by zero");
        self.overstep.as_secs_f64() / timestep.as_secs_f64()
    }

    /// Adds `time` to the internal `overstep`, then converts the `overstep` accumulated into
    /// as many `timestep`-sized steps as possible.
    ///
    /// # Panics
    ///
    /// Panics if `timestep` is a zero-length duration.
    pub fn add_time(&mut self, time: Duration, timestep: Duration) {
        assert!(!timestep.is_zero(), "division by zero");
        self.overstep += time;
        while self.overstep >= timestep {
            self.overstep -= timestep;
            self.steps += 1;
        }
    }

    /// Consumes one step and returns the number remaining. Returns `None` if there was
    /// no step to consume.
    pub fn sub_step(&mut self) -> Option<u32> {
        let remaining = self.steps.checked_sub(1);
        self.steps = self.steps.saturating_sub(1);
        remaining
    }

    /// Clears accumulated time and steps.
    pub fn reset(&mut self) {
        self.steps = 0;
        self.overstep = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use crate::{FixedTime, FixedTimestepState, Time};
    use bevy_utils::{Duration, Instant};
    #[test]
    fn test_fixed_timestep_state_methods() {
        let mut accumulator = FixedTimestepState::default();
        assert_eq!(accumulator.steps(), 0);
        assert_eq!(accumulator.overstep(), Duration::ZERO);

        accumulator.add_time(Duration::from_secs(5), Duration::from_secs(1));
        assert_eq!(accumulator.steps(), 5);
        assert_eq!(accumulator.overstep(), Duration::ZERO);

        let steps_remaining = accumulator.sub_step();
        assert_eq!(steps_remaining, Some(4));
        assert_eq!(accumulator.steps(), 4);
        assert_eq!(accumulator.overstep(), Duration::ZERO);

        accumulator.reset();
        assert_eq!(accumulator.steps(), 0);
        assert_eq!(accumulator.overstep(), Duration::ZERO);

        let steps_remaining = accumulator.sub_step();
        assert_eq!(steps_remaining, None);
        assert_eq!(accumulator.steps(), 0);
        assert_eq!(accumulator.overstep(), Duration::ZERO);
    }

    #[test]
    fn test_fixed_timestep() {
        let start_instant = Instant::now();
        let timestep = Duration::from_millis(20);

        // Create a `Time`, `FixedTime`, and `FixedTimestepState` for testing.
        let mut time = Time::new(start_instant);
        let mut fixed_time = FixedTime::new(timestep, start_instant);
        let mut accumulator = FixedTimestepState::default();

        // Confirm that the timestep is what we set it to be.
        assert_eq!(fixed_time.delta(), timestep);
        assert_eq!(fixed_time.delta_seconds(), timestep.as_secs_f32());
        assert_eq!(fixed_time.delta_seconds_f64(), timestep.as_secs_f64());

        // Get the first update out of the way, so time.delta() has a nonzero value next time.
        let first_update_instant = Instant::now();
        time.update_with_instant(first_update_instant);

        // Accumulate the time.
        let start_delay = first_update_instant - start_instant;
        accumulator.add_time(start_delay, fixed_time.delta());

        // 10.5x the timestep elapses before the second update.
        let ten = Duration::from_millis(200);
        let half = Duration::from_millis(10);

        let second_update_instant = first_update_instant + ten + half;
        time.update_with_instant(second_update_instant);
        assert_eq!(time.raw_delta(), ten + half);
        assert_eq!(time.delta(), ten + half);

        // Accumulate the time.
        accumulator.add_time(time.delta(), fixed_time.delta());

        // Confirm that 10.5 steps have accumulated.
        assert_eq!(accumulator.steps(), 10);
        assert_eq!(accumulator.overstep(), start_delay + half);

        // Confirm that fixed time has not been updated yet.
        assert_eq!(fixed_time.elapsed_since_startup(), Duration::ZERO);
        assert_eq!(
            fixed_time.seconds_since_startup(),
            Duration::ZERO.as_secs_f32()
        );
        assert_eq!(
            fixed_time.seconds_since_startup_f64(),
            Duration::ZERO.as_secs_f64()
        );

        // Consume accumulated steps and advanced the fixed time clock.
        while accumulator.sub_step().is_some() {
            fixed_time.update();
        }
        // Confirm that the timestep is still the same.
        assert_eq!(fixed_time.delta(), timestep);
        assert_eq!(fixed_time.delta_seconds(), timestep.as_secs_f32());
        assert_eq!(fixed_time.delta_seconds_f64(), timestep.as_secs_f64());

        // Confirm that the fixed time clock has advanced 10 steps worth of time.
        assert_eq!(fixed_time.elapsed_since_startup(), ten);
        assert_eq!(fixed_time.seconds_since_startup(), ten.as_secs_f32());
        assert_eq!(fixed_time.seconds_since_startup_f64(), ten.as_secs_f64());

        // Confirm that the fixed clock lags behind the normal clock by the sub-step amount.
        let diff = time.elapsed_since_startup() - fixed_time.elapsed_since_startup();
        assert_eq!(diff, start_delay + half);
    }
}
