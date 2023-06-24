use bevy_ecs::{reflect::ReflectResource, system::Resource};
use bevy_reflect::{FromReflect, Reflect};
use bevy_utils::tracing::warn;
use bevy_utils::{default, Duration, Instant};

use crate::clock::Clock;

/// A clock that tracks how much time has advanced since the last update and since startup.
///
/// **NOTE:** This clock can be set to advance faster or slower than real-time. It can also be paused.
/// For a clock that tracks the actual time that elapses, see [`RealTime`].
#[derive(Resource, Reflect, FromReflect, Debug, Clone)]
#[reflect(Resource)]
pub struct Time {
    context: TimeContext,
    update: Clock,
    fixed_update: Clock,
    // settings
    paused: bool,
    next_paused: Option<bool>,
    relative_speed: f64, // using `f64` instead of `f32` to minimize drift from rounding errors
    next_relative_speed: Option<f64>,
}

/// [`Time`] stores two clocks that are synchronized but advance at different rates.
/// This value determines which one is shown.
#[derive(Debug, Default, Clone, Copy, Reflect, FromReflect, PartialEq, Eq)]
pub enum TimeContext {
    #[default]
    Update,
    FixedUpdate,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            context: TimeContext::Update,
            update: default(),
            fixed_update: default(),
            paused: false,
            next_paused: None,
            relative_speed: 1.0,
            next_relative_speed: None,
        }
    }
}

impl Time {
    /// Constructs a new `Time` instance with a specific startup `Instant`.
    pub fn new(startup: Instant) -> Self {
        let clock = Clock::new(startup);
        Self {
            update: clock,
            fixed_update: clock,
            ..default()
        }
    }

    pub fn clock(&self, context: TimeContext) -> &Clock {
        match context {
            TimeContext::Update => &self.update,
            TimeContext::FixedUpdate => &self.fixed_update,
        }
    }

    pub(crate) fn clock_mut(&mut self, context: TimeContext) -> &mut Clock {
        match context {
            TimeContext::Update => &mut self.update,
            TimeContext::FixedUpdate => &mut self.fixed_update,
        }
    }

    pub(crate) fn current_clock(&self) -> &Clock {
        self.clock(self.context)
    }

    pub(crate) fn current_clock_mut(&mut self) -> &mut Clock {
        self.clock_mut(self.context)
    }

    /// Returns the current [`TimeContext`].
    pub fn context(&self) -> TimeContext {
        self.context
    }

    /// Changes the current [`TimeContext`].
    pub fn set_context(&mut self, context: TimeContext) {
        self.context = context;
    }

    /// Updates the internal time measurements.
    ///
    /// Calling this method as part of your app will most likely result in inaccurate timekeeping,
    /// as the `Time` resource is ordinarily managed by the [`TimePlugin`](crate::TimePlugin).
    pub fn update(&mut self) {
        let now = Instant::now();
        self.update_with_instant(now);
    }

    /// Updates time with a specified [`Instant`].
    ///
    /// This method is provided for use in tests. Calling this method as part of your app will most
    /// likely result in inaccurate timekeeping, as the `Time` resource is ordinarily managed by the
    /// [`TimePlugin`](crate::TimePlugin).
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
    /// #[derive(Resource)]
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
    ///     let mut schedule = Schedule::new();
    ///     schedule.add_systems(health_system);
    ///
    ///     // Simulate that 30 ms have passed
    ///     let mut time = world.resource_mut::<Time>();
    ///     let last_update = time.last_update().unwrap();
    ///     time.update_with_instant(last_update + Duration::from_millis(30));
    ///
    ///     // Run system
    ///     schedule.run(&mut world);
    ///
    ///     // Check that 0.003 has been added to the health value
    ///     let expected_health_value = 0.2 + 0.1 * 0.03;
    ///     let actual_health_value = world.resource::<Health>().health_value;
    ///     assert_eq!(expected_health_value, actual_health_value);
    /// }
    /// ```
    pub fn update_with_instant(&mut self, instant: Instant) {
        match self.context {
            TimeContext::Update => {
                // apply pending pause and relative speed changes
                self.apply_pending_changes();

                // zero for first update
                let dt = instant - self.current_clock().last_update().unwrap_or(instant);
                let dt = if self.paused {
                    Duration::ZERO
                } else if self.relative_speed != 1.0 {
                    dt.mul_f64(self.relative_speed)
                } else {
                    // avoid rounding when at normal speed
                    dt
                };

                self.update.tick(dt, instant);
            }
            TimeContext::FixedUpdate => {
                warn!("In the `FixedUpdate` context, `Time` can only be advanced via `tick`.");
            }
        }
    }

    pub(crate) fn tick(&mut self, dt: Duration, instant: Instant) {
        self.current_clock_mut().tick(dt, instant);
    }

    /// Applies pending pause or relative speed changes.
    ///
    /// This method is provided for use in tests. Calling this method as part of your app will most
    /// likely result in inaccurate timekeeping, as the `Time` resource is ordinarily managed by the
    /// [`TimePlugin`](crate::TimePlugin).
    pub fn apply_pending_changes(&mut self) {
        if let Some(value) = self.next_paused.take() {
            self.paused = value;
        }

        if let Some(value) = self.next_relative_speed.take() {
            self.relative_speed = value;
        }
    }

    /// Returns the [`Instant`] the clock was created.
    ///
    /// This usually represents when the app was started.
    #[inline]
    pub fn startup(&self) -> Instant {
        self.current_clock().startup()
    }

    /// Returns the [`Instant`] when [`update`](#method.update) was first called, if it exists.
    ///
    /// This usually represents when the first app update started.
    #[inline]
    pub fn first_update(&self) -> Option<Instant> {
        self.current_clock().first_update()
    }

    /// Returns the [`Instant`] when [`update`](#method.update) was last called, if it exists.
    ///
    /// This usually represents when the current app update started.
    #[inline]
    pub fn last_update(&self) -> Option<Instant> {
        self.current_clock().last_update()
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        self.current_clock().delta()
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f32`] seconds.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.current_clock().delta_seconds()
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f64`] seconds.
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.current_clock().delta_seconds_f64()
    }

    /// Returns how much time has advanced since [`first_update`](#method.first_update), as [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.current_clock().elapsed()
    }

    /// Returns how much time has advanced since [`first_update`](#method.first_update), as [`f32`] seconds.
    ///
    /// **Note:** This is a monotonically increasing value. It's precision will degrade over time.
    /// If you need an `f32` but that precision loss is unacceptable,
    /// use [`elapsed_seconds_wrapped`](#method.elapsed_seconds_wrapped).
    #[inline]
    pub fn elapsed_seconds(&self) -> f32 {
        self.current_clock().elapsed_seconds()
    }

    /// Returns how much time has advanced since [`first_update`](#method.first_update), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_f64(&self) -> f64 {
        self.current_clock().elapsed_seconds_f64()
    }

    /// Returns how much time has advanced since [`first_update`](#method.first_update) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`Duration`].
    #[inline]
    pub fn elapsed_wrapped(&self) -> Duration {
        self.current_clock().elapsed_wrapped()
    }

    /// Returns how much time has advanced since [`first_update`](#method.first_update) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f32`] seconds.
    ///
    /// This method is intended for applications (e.g. shaders) that require an [`f32`] value but
    /// suffer from the gradual precision loss of [`elapsed_seconds`](#method.elapsed_seconds).
    #[inline]
    pub fn elapsed_seconds_wrapped(&self) -> f32 {
        self.current_clock().elapsed_seconds_wrapped()
    }

    /// Returns how much time has advanced since [`first_update`](#method.first_update) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_wrapped_f64(&self) -> f64 {
        self.current_clock().elapsed_seconds_wrapped_f64()
    }

    /// Returns the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped) and
    /// [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** The default modulus is one hour.
    #[inline]
    pub fn wrap_period(&self) -> Duration {
        self.current_clock().wrap_period()
    }

    /// Sets the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped) and
    /// [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** This will not take effect until the next update.
    ///
    /// # Panics
    ///
    /// Panics if `wrap_period` is a zero-length duration.
    #[inline]
    pub fn set_wrap_period(&mut self, wrap_period: Duration) {
        self.current_clock_mut().set_wrap_period(wrap_period);
    }

    /// Returns the speed the clock advances relative to your system clock, as [`f32`].
    /// This is known as "time scaling" or "time dilation" in other engines.
    ///
    /// **Note:** This function will return zero when time is paused.
    #[inline]
    pub fn relative_speed(&self) -> f32 {
        self.relative_speed_f64() as f32
    }

    /// Returns the speed the clock advances relative to your system clock, as [`f64`].
    /// This is known as "time scaling" or "time dilation" in other engines.
    ///
    /// **Note:** This function will return zero when time is paused.
    #[inline]
    pub fn relative_speed_f64(&self) -> f64 {
        if self.paused {
            0.0
        } else {
            self.relative_speed
        }
    }

    /// Sets the speed the clock advances relative to its inputs, given as an [`f32`].
    ///
    /// For example, setting this to `2.0` will make the clock advance twice as fast as its inputs.
    ///
    /// **Note:** This will not take effect until the next update.
    ///
    /// # Panics
    ///
    /// Panics if `r` is negative or not finite.
    #[inline]
    pub fn set_relative_speed(&mut self, r: f32) {
        self.set_relative_speed_f64(r as f64);
    }

    /// Sets the speed the clock advances relative to its inputs, given as an [`f64`].
    ///
    /// For example, setting this to `2.0` will make the clock advance twice as fast as its inputs.
    ///
    /// **Note:** This will not take effect until the next update.
    ///
    /// # Panics
    ///
    /// Panics if `r` is negative or not finite.
    #[inline]
    pub fn set_relative_speed_f64(&mut self, r: f64) {
        assert!(r.is_finite(), "tried to go infinitely fast");
        assert!(r >= 0.0, "tried to go back in time");
        self.next_relative_speed = Some(r);
    }

    /// Stops the clock, preventing it from advancing until resumed.
    ///
    /// **Note:** This will not take effect until the next update.
    #[inline]
    pub fn pause(&mut self) {
        self.next_paused = Some(true);
    }

    /// Resumes the clock if paused.
    ///
    /// **Note:** This will not take effect until the next update.
    #[inline]
    pub fn unpause(&mut self) {
        self.next_paused = Some(false);
    }

    /// Returns `true` if the clock is currently paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }
}

/// A clock that tracks how much actual time has elasped since the last update and since startup.
///
/// This clock cannot be paused and will always advance at the same rate as [`Instant`].
#[derive(Resource, Debug)]
pub struct RealTime(Clock);

impl RealTime {
    /// Constructs a new `Time` instance with a specific startup `Instant`.
    pub(crate) fn new(startup: Instant) -> Self {
        Self(Clock::new(startup))
    }

    /// Updates the internal time measurements.
    ///
    /// Calling this method as part of your app will most likely result in inaccurate timekeeping,
    /// as this resource is ordinarily managed by the [`TimePlugin`](crate::TimePlugin).
    pub fn update(&mut self) {
        let now = Instant::now();
        self.update_with_instant(now);
    }

    /// Updates time with a specified [`Instant`].
    ///
    /// This method is provided for use in tests. Calling this method as part of your app will most
    /// likely result in inaccurate timekeeping, as this resource is ordinarily managed by the
    /// [`TimePlugin`](crate::TimePlugin).
    pub fn update_with_instant(&mut self, instant: Instant) {
        // zero for first update
        let dt = instant - self.0.last_update().unwrap_or(instant);
        self.0.tick(dt, instant);
    }

    /// Returns the [`Instant`] the clock was created.
    ///
    /// This usually represents when the app was started.
    #[inline]
    pub fn startup(&self) -> Instant {
        self.0.startup()
    }

    /// Returns the [`Instant`] when [`update`](#method.update) was first called, if it exists.
    ///
    /// This usually represents when the first app update started.
    #[inline]
    pub fn first_update(&self) -> Option<Instant> {
        self.0.first_update()
    }

    /// Returns the [`Instant`] when [`update`](#method.update) was last called, if it exists.
    ///
    /// This usually represents when the current app update started.
    #[inline]
    pub fn last_update(&self) -> Option<Instant> {
        self.0.last_update()
    }

    /// Returns how much real time has elapsed since the last [`update`](#method.update), as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        self.0.delta()
    }

    /// Returns how much real time has elapsed since the last [`update`](#method.update), as [`f32`] seconds.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.0.delta_seconds()
    }

    /// Returns how much real time has elapsed since the last [`update`](#method.update), as [`f64`] seconds.
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.0.delta_seconds_f64()
    }

    /// Returns how much real time has elapsed since [`first_update`](#method.first_update), as [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    /// Returns how much real time has elapsed since [`first_update`](#method.first_update), as [`f32`] seconds.
    ///
    /// **Note:** This is a monotonically increasing value. It's precision will degrade over time.
    /// If you need an `f32` but that precision loss is unacceptable,
    /// use [`elapsed_seconds_wrapped`](#method.elapsed_seconds_wrapped).
    #[inline]
    pub fn elapsed_seconds(&self) -> f32 {
        self.0.elapsed_seconds()
    }

    /// Returns how much real time has elapsed since [`first_update`](#method.first_update), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_f64(&self) -> f64 {
        self.0.elapsed_seconds_f64()
    }

    /// Returns how much real time has elapsed since [`first_update`](#method.first_update) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`Duration`].
    #[inline]
    pub fn elapsed_wrapped(&self) -> Duration {
        self.0.elapsed_wrapped()
    }

    /// Returns how much real time has elapsed since [`first_update`](#method.first_update) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f32`] seconds.
    ///
    /// This method is intended for applications (e.g. shaders) that require an [`f32`] value but
    /// suffer from the gradual precision loss of [`elapsed_seconds`](#method.elapsed_seconds).
    #[inline]
    pub fn elapsed_seconds_wrapped(&self) -> f32 {
        self.0.elapsed_seconds_wrapped()
    }

    /// Returns how much real time has elapsed since [`first_update`](#method.first_update) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_wrapped_f64(&self) -> f64 {
        self.0.elapsed_seconds_wrapped_f64()
    }

    /// Returns the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped) and
    /// [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** The default modulus is one hour.
    #[inline]
    pub fn wrap_period(&self) -> Duration {
        self.0.wrap_period()
    }

    /// Sets the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped) and
    /// [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** This will not take effect until the next update.
    ///
    /// # Panics
    ///
    /// Panics if `wrap_period` is a zero-length duration.
    #[inline]
    pub fn set_wrap_period(&mut self, wrap_period: Duration) {
        self.0.set_wrap_period(wrap_period);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use bevy_utils::{Duration, Instant};

    use super::{RealTime, Time};

    fn assert_float_eq(a: f32, b: f32) {
        assert!((a - b).abs() <= f32::EPSILON, "{a} != {b}");
    }

    #[test]
    fn update_test() {
        let startup = Instant::now();
        let mut time = Time::new(startup);

        // Ensure `time` was constructed correctly.
        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), None);
        assert_eq!(time.last_update(), None);
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.elapsed(), Duration::ZERO);
        assert_eq!(time.elapsed_seconds(), 0.0);
        assert_eq!(time.elapsed_seconds_f64(), 0.0);

        // Update `time` and check results.
        // The first update to `time` normally happens before other systems have run,
        // so the first delta doesn't appear until the second update.
        let first_update = Instant::now();
        time.update_with_instant(first_update);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(first_update));
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_seconds(), 0.0);
        assert_eq!(time.delta_seconds_f64(), 0.0);
        assert_eq!(time.elapsed(), first_update - first_update);
        assert_eq!(
            time.elapsed_seconds(),
            (first_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            time.elapsed_seconds_f64(),
            (first_update - first_update).as_secs_f64(),
        );

        // Update `time` again and check results.
        // At this point its safe to use time.delta().
        let second_update = Instant::now();
        time.update_with_instant(second_update);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(second_update));
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), second_update - first_update);
        assert_eq!(
            time.delta_seconds(),
            (second_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            time.delta_seconds_f64(),
            (second_update - first_update).as_secs_f64(),
        );
        assert_eq!(time.elapsed(), second_update - first_update);
        assert_eq!(
            time.elapsed_seconds(),
            (second_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            time.elapsed_seconds_f64(),
            (second_update - first_update).as_secs_f64(),
        );
    }

    #[test]
    fn wrapping_test() {
        let startup = Instant::now();

        let mut time = Time::new(startup);
        time.set_wrap_period(Duration::from_secs(3));

        assert_eq!(time.elapsed_seconds_wrapped(), 0.0);

        // time starts counting from first update
        let first_update = Instant::now();
        time.update_with_instant(first_update);

        time.update_with_instant(first_update + Duration::from_secs(1));
        assert_float_eq(time.elapsed_seconds_wrapped(), 1.0);

        time.update_with_instant(first_update + Duration::from_secs(2));
        assert_float_eq(time.elapsed_seconds_wrapped(), 2.0);

        time.update_with_instant(first_update + Duration::from_secs(3));
        assert_float_eq(time.elapsed_seconds_wrapped(), 0.0);

        time.update_with_instant(first_update + Duration::from_secs(4));
        assert_float_eq(time.elapsed_seconds_wrapped(), 1.0);
    }

    #[test]
    fn relative_speed_test() {
        let startup = Instant::now();
        let mut time = Time::new(startup);
        let mut real_time = RealTime::new(startup);

        let first_update = Instant::now();
        time.update_with_instant(first_update);
        real_time.update_with_instant(first_update);

        // Update `time` again and check results.
        // At this point its safe to use time.delta().
        let second_update = Instant::now();
        time.update_with_instant(second_update);
        real_time.update_with_instant(second_update);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(second_update));
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.delta(), second_update - first_update);
        assert_eq!(
            time.delta_seconds(),
            (second_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            time.delta_seconds_f64(),
            (second_update - first_update).as_secs_f64(),
        );
        assert_eq!(real_time.delta(), second_update - first_update);
        assert_eq!(
            real_time.delta_seconds(),
            (second_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            real_time.delta_seconds_f64(),
            (second_update - first_update).as_secs_f64(),
        );
        assert_eq!(time.elapsed(), second_update - first_update);
        assert_eq!(
            time.elapsed_seconds(),
            (second_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            time.elapsed_seconds_f64(),
            (second_update - first_update).as_secs_f64(),
        );
        assert_eq!(real_time.elapsed(), second_update - first_update);
        assert_eq!(
            real_time.elapsed_seconds(),
            (second_update - first_update).as_secs_f32(),
        );
        assert_eq!(
            real_time.elapsed_seconds_f64(),
            (second_update - first_update).as_secs_f64(),
        );

        // Make app time advance at 2x the rate of your system clock.
        time.set_relative_speed(2.0);
        time.apply_pending_changes();

        // Update `time` again 1 second later.
        let elapsed = Duration::from_secs(1);
        let third_update = second_update + elapsed;
        time.update_with_instant(third_update);
        real_time.update_with_instant(third_update);

        // Since app is advancing 2x your system clock, expect time
        // to have advanced by twice the amount of real time elapsed.
        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(third_update));
        assert_eq!(time.relative_speed(), 2.0);
        assert_eq!(time.delta(), elapsed.mul_f32(2.0));
        assert_eq!(time.delta_seconds(), elapsed.mul_f32(2.0).as_secs_f32());
        assert_eq!(time.delta_seconds_f64(), elapsed.mul_f32(2.0).as_secs_f64());
        assert_eq!(real_time.delta(), elapsed);
        assert_eq!(real_time.delta_seconds(), elapsed.as_secs_f32());
        assert_eq!(real_time.delta_seconds_f64(), elapsed.as_secs_f64());
        assert_eq!(
            time.elapsed(),
            second_update - first_update + elapsed.mul_f32(2.0),
        );
        assert_eq!(
            time.elapsed_seconds(),
            (second_update - first_update + elapsed.mul_f32(2.0)).as_secs_f32(),
        );
        assert_eq!(
            time.elapsed_seconds_f64(),
            (second_update - first_update + elapsed.mul_f32(2.0)).as_secs_f64(),
        );
        assert_eq!(real_time.elapsed(), second_update - first_update + elapsed);
        assert_eq!(
            real_time.elapsed_seconds(),
            (second_update - first_update + elapsed).as_secs_f32(),
        );
        assert_eq!(
            real_time.elapsed_seconds_f64(),
            (second_update - first_update + elapsed).as_secs_f64(),
        );
    }

    #[test]
    fn pause_test() {
        let startup = Instant::now();
        let mut time = Time::new(startup);
        let mut real_time = RealTime::new(startup);

        let first_update = Instant::now();
        time.update_with_instant(first_update);
        real_time.update_with_instant(first_update);

        assert!(!time.is_paused());
        assert_eq!(time.relative_speed(), 1.0);

        time.pause();
        time.apply_pending_changes();

        assert!(time.is_paused());
        assert_eq!(time.relative_speed(), 0.0);

        let second_update = Instant::now();
        time.update_with_instant(second_update);
        real_time.update_with_instant(second_update);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(second_update));
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(real_time.delta(), second_update - first_update);
        assert_eq!(time.elapsed(), first_update - first_update);
        assert_eq!(real_time.elapsed(), second_update - first_update);

        time.unpause();
        time.apply_pending_changes();

        assert!(!time.is_paused());
        assert_eq!(time.relative_speed(), 1.0);

        let third_update = Instant::now();
        time.update_with_instant(third_update);
        real_time.update_with_instant(third_update);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(third_update));
        assert_eq!(time.delta(), third_update - second_update);
        assert_eq!(real_time.delta(), third_update - second_update);
        assert_eq!(
            time.elapsed(),
            (third_update - second_update) + (first_update - first_update),
        );
        assert_eq!(real_time.elapsed(), third_update - first_update);
    }
}
