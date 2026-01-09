use bevy_ecs::resource::Resource;
use core::{
    ops::{Add, AddAssign, Sub},
    time::Duration,
};

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::reflect::ReflectResource,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

/// A generic clock resource that tracks how much it has advanced since its
/// previous update and since its creation.
///
/// Multiple instances of this resource are inserted automatically by
/// [`TimePlugin`](crate::TimePlugin):
///
/// - [`Time<Real>`](crate::real::Real) tracks real wall-clock time elapsed.
/// - [`Time<Virtual>`](crate::virt::Virtual) tracks virtual game time that may
///   be paused or scaled.
/// - [`Time<Fixed>`](crate::fixed::Fixed) tracks fixed timesteps based on
///   virtual time.
/// - [`Time`] is a generic clock that corresponds to "current" or "default"
///   time for systems. It contains [`Time<Virtual>`](crate::virt::Virtual)
///   except inside the [`FixedMain`](bevy_app::FixedMain) schedule when it
///   contains [`Time<Fixed>`](crate::fixed::Fixed).
///
/// The time elapsed since the previous time this clock was advanced is saved as
/// [`delta()`](Time::delta) and the total amount of time the clock has advanced
/// is saved as [`elapsed()`](Time::elapsed). Both are represented as exact
/// [`Duration`] values with fixed nanosecond precision. The clock does not
/// support time moving backwards, but it can be updated with [`Duration::ZERO`]
/// which will set [`delta()`](Time::delta) to zero.
///
/// These values are also available in seconds as `f32` via
/// [`delta_secs()`](Time::delta_secs) and
/// [`elapsed_secs()`](Time::elapsed_secs), and also in seconds as `f64`
/// via [`delta_secs_f64()`](Time::delta_secs_f64) and
/// [`elapsed_secs_f64()`](Time::elapsed_secs_f64).
///
/// Since [`elapsed_secs()`](Time::elapsed_secs) will grow constantly and
/// is `f32`, it will exhibit gradual precision loss. For applications that
/// require an `f32` value but suffer from gradual precision loss there is
/// [`elapsed_secs_wrapped()`](Time::elapsed_secs_wrapped) available. The
/// same wrapped value is also available as [`Duration`] and `f64` for
/// consistency. The wrap period is by default 1 hour, and can be set by
/// [`set_wrap_period()`](Time::set_wrap_period).
///
/// # Accessing clocks
///
/// By default, any systems requiring current [`delta()`](Time::delta) or
/// [`elapsed()`](Time::elapsed) should use `Res<Time>` to access the default
/// time configured for the program. By default, this refers to
/// [`Time<Virtual>`](crate::virt::Virtual) except during the
/// [`FixedMain`](bevy_app::FixedMain) schedule when it refers to
/// [`Time<Fixed>`](crate::fixed::Fixed). This ensures your system can be used
/// either in [`Update`](bevy_app::Update) or
/// [`FixedUpdate`](bevy_app::FixedUpdate) schedule depending on what is needed.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_time::prelude::*;
/// #
/// fn ambivalent_system(time: Res<Time>) {
///     println!("this how I see time: delta {:?}, elapsed {:?}", time.delta(), time.elapsed());
/// }
/// ```
///
/// If your system needs to react based on real time (wall clock time), like for
/// user interfaces, it should use `Res<Time<Real>>`. The
/// [`delta()`](Time::delta) and [`elapsed()`](Time::elapsed) values will always
/// correspond to real time and will not be affected by pause, time scaling or
/// other tweaks.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_time::prelude::*;
/// #
/// fn real_time_system(time: Res<Time<Real>>) {
///     println!("this will always be real time: delta {:?}, elapsed {:?}", time.delta(), time.elapsed());
/// }
/// ```
///
/// If your system specifically needs to access fixed timestep clock, even when
/// placed in `Update` schedule, you should use `Res<Time<Fixed>>`. The
/// [`delta()`](Time::delta) and [`elapsed()`](Time::elapsed) values will
/// correspond to the latest fixed timestep that has been run.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_time::prelude::*;
/// #
/// fn fixed_time_system(time: Res<Time<Fixed>>) {
///     println!("this will always be the last executed fixed timestep: delta {:?}, elapsed {:?}", time.delta(), time.elapsed());
/// }
/// ```
///
/// Finally, if your system specifically needs to know the current virtual game
/// time, even if placed inside [`FixedUpdate`](bevy_app::FixedUpdate), for
/// example to know if the game is [`was_paused()`](Time::was_paused) or to use
/// [`effective_speed()`](Time::effective_speed), you can use
/// `Res<Time<Virtual>>`. However, if the system is placed in
/// [`FixedUpdate`](bevy_app::FixedUpdate), extra care must be used because your
/// system might be run multiple times with the same [`delta()`](Time::delta)
/// and [`elapsed()`](Time::elapsed) values as the virtual game time has not
/// changed between the iterations.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_time::prelude::*;
/// #
/// fn fixed_time_system(time: Res<Time<Virtual>>) {
///     println!("this will be virtual time for this update: delta {:?}, elapsed {:?}", time.delta(), time.elapsed());
///     println!("also the relative speed of the game is now {}", time.effective_speed());
/// }
/// ```
///
/// If you need to change the settings for any of the clocks, for example to
/// [`pause()`](Time::pause) the game, you should use `ResMut<Time<Virtual>>`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_time::prelude::*;
/// #
/// #[derive(Message)]
/// struct Pause(bool);
///
/// fn pause_system(mut time: ResMut<Time<Virtual>>, mut pause_reader: MessageReader<Pause>) {
///     for pause in pause_reader.read() {
///         if pause.0 {
///             time.pause();
///         } else {
///             time.unpause();
///         }
///     }
/// }
/// ```
///
/// # Adding custom clocks
///
/// New custom clocks can be created by creating your own struct as a context
/// and passing it to [`new_with()`](Time::new_with). These clocks can be
/// inserted as resources as normal and then accessed by systems. You can use
/// the [`advance_by()`](Time::advance_by) or [`advance_to()`](Time::advance_to)
/// methods to move the clock forwards based on your own logic.
///
/// If you want to add methods for your time instance and they require access to
/// both your context and the generic time part, it's probably simplest to add a
/// custom trait for them and implement it for `Time<Custom>`.
///
/// Your context struct will need to implement the [`Default`] trait because
/// [`Time`] structures support reflection. It also makes initialization trivial
/// by being able to call `app.init_resource::<Time<Custom>>()`.
///
/// You can also replace the "generic" `Time` clock resource if the "default"
/// time for your game should not be the default virtual time provided. You can
/// get a "generic" snapshot of your clock by calling `as_other()` and then
/// overwrite the [`Time`] resource with it. The default systems added by
/// [`TimePlugin`](crate::TimePlugin) will overwrite the [`Time`] clock during
/// [`First`](bevy_app::First) and [`FixedUpdate`](bevy_app::FixedUpdate)
/// schedules.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_time::prelude::*;
/// # use bevy_platform::time::Instant;
/// #
/// #[derive(Debug)]
/// struct Custom {
///     last_external_time: Instant,
/// }
///
/// impl Default for Custom {
///     fn default() -> Self {
///         Self {
///             last_external_time: Instant::now(),
///         }
///     }
/// }
///
/// trait CustomTime {
///     fn update_from_external(&mut self, instant: Instant);
/// }
///
/// impl CustomTime for Time<Custom> {
///     fn update_from_external(&mut self, instant: Instant) {
///          let delta = instant - self.context().last_external_time;
///          self.advance_by(delta);
///          self.context_mut().last_external_time = instant;
///     }
/// }
/// ```
#[derive(Resource, Debug, Copy, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Resource, Default))]
pub struct Time<CONTEXT: Default = (), STEP: TimeDuration = Duration> {
    pub(crate) context: CONTEXT,
    pub(crate) wrap_period: STEP,
    pub(crate) delta: STEP,
    pub(crate) elapsed: STEP,
    pub(crate) elapsed_wrapped: STEP,
    pub(crate) precompute: STEP::Precompute,
}

/// Type that represents a duration of time.
pub trait TimeDuration:
    Default + Clone + Copy + PartialOrd + Add<Self> + AddAssign<Self> + Sub<Self, Output = Self>
{
    /// Used to store precomputed value about the time elapsed and the duration
    type Precompute: TimeDurationPrecompute<Self>;
    /// Empty duration
    const ZERO: Self;
    /// Default duration for a wrap period
    const DEFAULT_WRAP_PERIOD: Self;

    /// Wrap the given duration within the specified period.
    fn wrap(value: Self, wrap_period: Self) -> Self;

    /// Is this duration empty
    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

/// Type that represents a precomputed value about the time elapsed and the duration
pub trait TimeDurationPrecompute<STEP>: Default + Clone + Copy {
    /// Update the precomputed value with the given delta.
    fn update_delta(&mut self, delta: STEP);
    /// Update the precomputed value with the given elapsed time.
    fn update_elapsed(&mut self, elapsed: STEP);
    /// Update the precomputed value with the given elapsed time and wrap period.
    fn update_elapsed_wrapped(&mut self, elapsed: STEP);
}

impl<STEP> TimeDurationPrecompute<STEP> for () {
    fn update_delta(&mut self, _delta: STEP) {}
    fn update_elapsed(&mut self, _elapsed: STEP) {}
    fn update_elapsed_wrapped(&mut self, _elapsed: STEP) {}
}

#[derive(Default, Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
pub struct DurationPreCompute {
    delta_f32: f32,
    delta_f64: f64,
    elapsed_f32: f32,
    elapsed_f64: f64,
    elapsed_wrapped_f32: f32,
    elapsed_wrapped_f64: f64,
}

impl TimeDurationPrecompute<Duration> for DurationPreCompute {
    #[inline]
    fn update_delta(&mut self, delta: Duration) {
        self.delta_f32 = delta.as_secs_f32();
        self.delta_f64 = delta.as_secs_f64();
    }

    #[inline]
    fn update_elapsed(&mut self, elapsed: Duration) {
        self.elapsed_f32 = elapsed.as_secs_f32();
        self.elapsed_f64 = elapsed.as_secs_f64();
    }

    #[inline]
    fn update_elapsed_wrapped(&mut self, elapsed_wrapped: Duration) {
        self.elapsed_wrapped_f32 = elapsed_wrapped.as_secs_f32();
        self.elapsed_wrapped_f64 = elapsed_wrapped.as_secs_f64();
    }
}

impl TimeDuration for Duration {
    type Precompute = DurationPreCompute;
    const ZERO: Duration = Duration::ZERO;
    const DEFAULT_WRAP_PERIOD: Duration = Duration::from_secs(3600); // 1 hour

    fn wrap(dividend: Duration, divisor: Duration) -> Duration {
        // `Duration` does not have a built-in modulo operation
        let quotient = (dividend.as_nanos() / divisor.as_nanos()) as u32;
        dividend - (quotient * divisor)
    }

    fn is_zero(&self) -> bool {
        self.is_zero()
    }
}

impl<CONTEXT: Default, STEP: TimeDuration> Time<CONTEXT, STEP> {
    /// Create a new clock from context with [`Self::delta`] and [`Self::elapsed`] starting from
    /// zero.
    pub fn new_with(context: CONTEXT) -> Self {
        Self {
            context,
            ..Default::default()
        }
    }

    /// Advance this clock by adding a `delta` duration to it.
    ///
    /// The added duration will be returned by [`Self::delta`] and
    /// [`Self::elapsed`] will be increased by the duration. Adding
    /// [`Duration::ZERO`] is allowed and will set [`Self::delta`] to zero.
    pub fn advance_by(&mut self, delta: STEP) {
        self.delta = delta;
        self.precompute.update_delta(self.delta);
        self.elapsed += delta;
        self.precompute.update_elapsed(self.elapsed);
        self.elapsed_wrapped = STEP::wrap(self.elapsed, self.wrap_period);
        self.precompute.update_elapsed_wrapped(self.elapsed_wrapped);
    }

    /// Advance this clock to a specific `elapsed` time.
    ///
    /// [`Self::delta()`] will return the amount of time the clock was advanced
    /// and [`Self::elapsed()`] will be the `elapsed` value passed in. Cannot be
    /// used to move time backwards.
    ///
    /// # Panics
    ///
    /// Panics if `elapsed` is less than `Self::elapsed()`.
    pub fn advance_to(&mut self, elapsed: STEP) {
        assert!(
            elapsed >= self.elapsed,
            "tried to move time backwards to an earlier elapsed moment"
        );
        self.advance_by(elapsed - self.elapsed);
    }

    /// Returns the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** The default modulus is one hour.
    #[inline]
    pub fn wrap_period(&self) -> STEP {
        self.wrap_period
    }

    /// Sets the modulus used to calculate [`elapsed_wrapped`](#method.elapsed_wrapped).
    ///
    /// **Note:** This will not take effect until the next update.
    ///
    /// # Panics
    ///
    /// Panics if `wrap_period` is a zero-length duration.
    #[inline]
    pub fn set_wrap_period(&mut self, wrap_period: STEP) {
        assert!(!wrap_period.is_zero(), "division by zero");
        self.wrap_period = wrap_period;
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as a
    /// [`Duration`].
    #[inline]
    pub fn delta(&self) -> STEP {
        self.delta
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`Duration`].
    #[inline]
    pub fn elapsed(&self) -> STEP {
        self.elapsed
    }

    /// Returns how much time has advanced since [`startup`](#method.startup) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`Duration`].
    #[inline]
    pub fn elapsed_wrapped(&self) -> STEP {
        self.elapsed_wrapped
    }

    /// Returns a reference to the context of this specific clock.
    #[inline]
    pub fn context(&self) -> &CONTEXT {
        &self.context
    }

    /// Returns a mutable reference to the context of this specific clock.
    #[inline]
    pub fn context_mut(&mut self) -> &mut CONTEXT {
        &mut self.context
    }

    /// Returns a copy of this clock as fully generic clock without context.
    #[inline]
    pub fn as_other<OtherContext: Default>(&self) -> Time<OtherContext, STEP> {
        Time {
            context: OtherContext::default(),
            wrap_period: self.wrap_period,
            delta: self.delta,
            elapsed: self.elapsed,
            elapsed_wrapped: self.elapsed_wrapped,
            precompute: self.precompute,
        }
    }
}

impl<CONTEXT: Default> Time<CONTEXT, Duration> {
    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f32`]
    /// seconds.
    #[inline]
    pub fn delta_secs(&self) -> f32 {
        self.precompute.delta_f32
    }

    /// Returns how much time has advanced since the last [`update`](#method.update), as [`f64`]
    /// seconds.
    #[inline]
    pub fn delta_secs_f64(&self) -> f64 {
        self.precompute.delta_f64
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`f32`] seconds.
    ///
    /// **Note:** This is a monotonically increasing value. Its precision will degrade over time.
    /// If you need an `f32` but that precision loss is unacceptable,
    /// use [`elapsed_secs_wrapped`](#method.elapsed_secs_wrapped).
    #[inline]
    pub fn elapsed_secs(&self) -> f32 {
        self.precompute.elapsed_f32
    }

    /// Returns how much time has advanced since [`startup`](#method.startup), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_secs_f64(&self) -> f64 {
        self.precompute.elapsed_f64
    }

    /// Returns how much time has advanced since [`startup`](#method.startup) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f32`] seconds.
    ///
    /// This method is intended for applications (e.g. shaders) that require an [`f32`] value but
    /// suffer from the gradual precision loss of [`elapsed_secs`](#method.elapsed_secs).
    #[inline]
    pub fn elapsed_secs_wrapped(&self) -> f32 {
        self.precompute.elapsed_wrapped_f32
    }

    /// Returns how much time has advanced since [`startup`](#method.startup) modulo
    /// the [`wrap_period`](#method.wrap_period), as [`f64`] seconds.
    #[inline]
    pub fn elapsed_secs_wrapped_f64(&self) -> f64 {
        self.precompute.elapsed_wrapped_f64
    }
}

impl<CONTEXT: Default, STEP: Default + TimeDuration> Default for Time<CONTEXT, STEP> {
    fn default() -> Self {
        Self {
            context: Default::default(),
            wrap_period: STEP::DEFAULT_WRAP_PERIOD,
            delta: STEP::ZERO,
            elapsed: STEP::ZERO,
            elapsed_wrapped: STEP::ZERO,
            precompute: STEP::Precompute::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_initial_state() {
        let time: Time = Time::default();

        assert_eq!(time.wrap_period(), Duration::DEFAULT_WRAP_PERIOD);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_secs(), 0.0);
        assert_eq!(time.delta_secs_f64(), 0.0);
        assert_eq!(time.elapsed(), Duration::ZERO);
        assert_eq!(time.elapsed_secs(), 0.0);
        assert_eq!(time.elapsed_secs_f64(), 0.0);
        assert_eq!(time.elapsed_wrapped(), Duration::ZERO);
        assert_eq!(time.elapsed_secs_wrapped(), 0.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 0.0);
    }

    #[test]
    fn test_advance_by() {
        let mut time: Time = Time::default();

        time.advance_by(Duration::from_millis(250));

        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.delta_secs(), 0.25);
        assert_eq!(time.delta_secs_f64(), 0.25);
        assert_eq!(time.elapsed(), Duration::from_millis(250));
        assert_eq!(time.elapsed_secs(), 0.25);
        assert_eq!(time.elapsed_secs_f64(), 0.25);

        time.advance_by(Duration::from_millis(500));

        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.delta_secs(), 0.5);
        assert_eq!(time.delta_secs_f64(), 0.5);
        assert_eq!(time.elapsed(), Duration::from_millis(750));
        assert_eq!(time.elapsed_secs(), 0.75);
        assert_eq!(time.elapsed_secs_f64(), 0.75);

        time.advance_by(Duration::ZERO);

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_secs(), 0.0);
        assert_eq!(time.delta_secs_f64(), 0.0);
        assert_eq!(time.elapsed(), Duration::from_millis(750));
        assert_eq!(time.elapsed_secs(), 0.75);
        assert_eq!(time.elapsed_secs_f64(), 0.75);
    }

    #[test]
    fn test_advance_to() {
        let mut time: Time = Time::default();

        time.advance_to(Duration::from_millis(250));

        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.delta_secs(), 0.25);
        assert_eq!(time.delta_secs_f64(), 0.25);
        assert_eq!(time.elapsed(), Duration::from_millis(250));
        assert_eq!(time.elapsed_secs(), 0.25);
        assert_eq!(time.elapsed_secs_f64(), 0.25);

        time.advance_to(Duration::from_millis(750));

        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.delta_secs(), 0.5);
        assert_eq!(time.delta_secs_f64(), 0.5);
        assert_eq!(time.elapsed(), Duration::from_millis(750));
        assert_eq!(time.elapsed_secs(), 0.75);
        assert_eq!(time.elapsed_secs_f64(), 0.75);

        time.advance_to(Duration::from_millis(750));

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.delta_secs(), 0.0);
        assert_eq!(time.delta_secs_f64(), 0.0);
        assert_eq!(time.elapsed(), Duration::from_millis(750));
        assert_eq!(time.elapsed_secs(), 0.75);
        assert_eq!(time.elapsed_secs_f64(), 0.75);
    }

    #[test]
    #[should_panic]
    fn test_advance_to_backwards_panics() {
        let mut time: Time = Time::default();

        time.advance_to(Duration::from_millis(750));

        time.advance_to(Duration::from_millis(250));
    }

    #[test]
    fn test_wrapping() {
        let mut time: Time = Time::default();
        time.set_wrap_period(Duration::from_secs(3));

        time.advance_by(Duration::from_secs(2));

        assert_eq!(time.elapsed_wrapped(), Duration::from_secs(2));
        assert_eq!(time.elapsed_secs_wrapped(), 2.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 2.0);

        time.advance_by(Duration::from_secs(2));

        assert_eq!(time.elapsed_wrapped(), Duration::from_secs(1));
        assert_eq!(time.elapsed_secs_wrapped(), 1.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 1.0);

        time.advance_by(Duration::from_secs(2));

        assert_eq!(time.elapsed_wrapped(), Duration::ZERO);
        assert_eq!(time.elapsed_secs_wrapped(), 0.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 0.0);

        time.advance_by(Duration::new(3, 250_000_000));

        assert_eq!(time.elapsed_wrapped(), Duration::from_millis(250));
        assert_eq!(time.elapsed_secs_wrapped(), 0.25);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 0.25);
    }

    #[test]
    fn test_wrapping_change() {
        let mut time: Time = Time::default();
        time.set_wrap_period(Duration::from_secs(5));

        time.advance_by(Duration::from_secs(8));

        assert_eq!(time.elapsed_wrapped(), Duration::from_secs(3));
        assert_eq!(time.elapsed_secs_wrapped(), 3.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 3.0);

        time.set_wrap_period(Duration::from_secs(2));

        assert_eq!(time.elapsed_wrapped(), Duration::from_secs(3));
        assert_eq!(time.elapsed_secs_wrapped(), 3.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 3.0);

        time.advance_by(Duration::ZERO);

        // Time will wrap to modulo duration from full `elapsed()`, not to what
        // is left in `elapsed_wrapped()`. This test of values is here to ensure
        // that we notice if we change that behavior.
        assert_eq!(time.elapsed_wrapped(), Duration::from_secs(0));
        assert_eq!(time.elapsed_secs_wrapped(), 0.0);
        assert_eq!(time.elapsed_secs_wrapped_f64(), 0.0);
    }
}
