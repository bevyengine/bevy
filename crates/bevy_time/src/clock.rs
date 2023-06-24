use bevy_reflect::{FromReflect, Reflect};
use bevy_utils::{default, Duration, Instant};

fn duration_div_rem(dividend: Duration, divisor: Duration) -> (u32, Duration) {
    // `Duration` does not have a built-in modulo operation
    let quotient = (dividend.as_nanos() / divisor.as_nanos()) as u32;
    let remainder = dividend - (quotient * divisor);
    (quotient, remainder)
}

/// A stopwatch that tracks the time since last update and the time since creation.
#[derive(Debug, Clone, Copy, Reflect, FromReflect, PartialEq)]
pub struct Clock {
    startup: Instant,
    first_update: Option<Instant>,
    last_update: Option<Instant>,
    delta: Duration,
    delta_seconds: f32,
    delta_seconds_f64: f64,
    elapsed: Duration,
    elapsed_seconds: f32,
    elapsed_seconds_f64: f64,
    wrap_period: Duration,
    elapsed_wrapped: Duration,
    elapsed_seconds_wrapped: f32,
    elapsed_seconds_wrapped_f64: f64,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            startup: Instant::now(),
            first_update: None,
            last_update: None,
            delta: Duration::ZERO,
            delta_seconds: 0.0,
            delta_seconds_f64: 0.0,
            elapsed: Duration::ZERO,
            elapsed_seconds: 0.0,
            elapsed_seconds_f64: 0.0,
            wrap_period: Duration::from_secs(3600), // 1 hour
            elapsed_wrapped: Duration::ZERO,
            elapsed_seconds_wrapped: 0.0,
            elapsed_seconds_wrapped_f64: 0.0,
        }
    }
}

impl Clock {
    /// Constructs a new `Clock` instance with a specific startup `Instant`.
    pub fn new(startup: Instant) -> Self {
        Self {
            startup,
            ..default()
        }
    }

    /// Advances the clock by `dt` and records it happening at `instant`.
    pub fn update(&mut self, dt: Duration, instant: Instant) {
        self.delta = dt;
        self.delta_seconds = self.delta.as_secs_f32();
        self.delta_seconds_f64 = self.delta.as_secs_f64();

        self.elapsed += dt;
        self.elapsed_seconds = self.elapsed.as_secs_f32();
        self.elapsed_seconds_f64 = self.elapsed.as_secs_f64();

        self.elapsed_wrapped = duration_div_rem(self.elapsed, self.wrap_period).1;
        self.elapsed_seconds_wrapped = self.elapsed_wrapped.as_secs_f32();
        self.elapsed_seconds_wrapped_f64 = self.elapsed_wrapped.as_secs_f64();

        if self.last_update.is_none() {
            self.first_update = Some(instant);
        }

        self.last_update = Some(instant);
    }

    /// Returns the [`Instant`] the clock was created.
    #[inline]
    pub fn startup(&self) -> Instant {
        self.startup
    }

    /// Returns the [`Instant`] when [`update`](#method.update) was first called, if it exists.
    #[inline]
    pub fn first_update(&self) -> Option<Instant> {
        self.first_update
    }

    /// Returns the [`Instant`] when [`update`](#method.update) was last called, if it exists.
    #[inline]
    pub fn last_update(&self) -> Option<Instant> {
        self.last_update
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as a [`Duration`].
    #[inline]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f32`] seconds.
    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f64`] seconds.
    #[inline]
    pub fn delta_seconds_f64(&self) -> f64 {
        self.delta_seconds_f64
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`f32`] seconds.
    ///
    /// **Note:** This is a monotonically increasing value. It's precision will degrade over time.
    /// If you need an `f32` but that precision loss is unacceptable,
    /// use [`elapsed_seconds_wrapped`](#method.elapsed_seconds_wrapped).
    #[inline]
    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_seconds
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_f64(&self) -> f64 {
        self.elapsed_seconds_f64
    }

    /// Returns how much time has advanced since [`startup`](#method.startup) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`Duration`].
    #[inline]
    pub fn elapsed_wrapped(&self) -> Duration {
        self.elapsed_wrapped
    }

    /// Returns how much time has advanced since [`startup`](#method.startup) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f32`] seconds.
    ///
    /// This method is intended for applications (e.g. shaders) that require an [`f32`] value but
    /// suffer from the gradual precision loss of [`elapsed_seconds`](#method.elapsed_seconds).
    #[inline]
    pub fn elapsed_seconds_wrapped(&self) -> f32 {
        self.elapsed_seconds_wrapped
    }

    /// Returns how much time has advanced since [`startup`](#method.startup) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_seconds_wrapped_f64(&self) -> f64 {
        self.elapsed_seconds_wrapped_f64
    }

    /// Returns the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped) and
    /// [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** The default modulus is one hour.
    #[inline]
    pub fn wrap_period(&self) -> Duration {
        self.wrap_period
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
        assert!(!wrap_period.is_zero(), "division by zero");
        self.wrap_period = wrap_period;
    }
}
