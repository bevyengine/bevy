#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::{tracing::debug, Duration};

use crate::{real::Real, time::Time};

/// The virtual game clock representing game time.
///
/// A specialization of the [`Time`] structure. **For method documentation, see
/// [`Time<Virtual>#impl-Time<Virtual>`].**
///
/// Normally used as `Time<Virtual>`. It is automatically inserted as a resource
/// by [`TimePlugin`](crate::TimePlugin) and updated based on
/// [`Time<Real>`](Real). The virtual clock is automatically set as the default
/// generic [`Time`] resource for the update.
///
/// The virtual clock differs from real time clock in that it can be paused, sped up
/// and slowed down. It also limits how much it can advance in a single update
/// in order to prevent unexpected behavior in cases where updates do not happen
/// at regular intervals (e.g. coming back after the program was suspended a long time).
///
/// The virtual clock can be paused by calling [`pause()`](Time::pause) and
/// unpaused by calling [`unpause()`](Time::unpause). When the game clock is
/// paused [`delta()`](Time::delta) will be zero on each update, and
/// [`elapsed()`](Time::elapsed) will not grow.
/// [`effective_speed()`](Time::effective_speed) will return `0.0`. Calling
/// [`pause()`](Time::pause) will not affect value the [`delta()`](Time::delta)
/// value for the update currently being processed.
///
/// The speed of the virtual clock can be changed by calling
/// [`set_relative_speed()`](Time::set_relative_speed). A value of `2.0` means
/// that virtual clock should advance twice as fast as real time, meaning that
/// [`delta()`](Time::delta) values will be double of what
/// [`Time<Real>::delta()`](Time::delta) reports and
/// [`elapsed()`](Time::elapsed) will go twice as fast as
/// [`Time<Real>::elapsed()`](Time::elapsed). Calling
/// [`set_relative_speed()`](Time::set_relative_speed) will not affect the
/// [`delta()`](Time::delta) value for the update currently being processed.
///
/// The maximum amount of delta time that can be added by a single update can be
/// set by [`set_max_delta()`](Time::set_max_delta). This value serves a dual
/// purpose in the virtual clock.
///
/// If the game temporarily freezes due to any reason, such as disk access, a
/// blocking system call, or operating system level suspend, reporting the full
/// elapsed delta time is likely to cause bugs in game logic. Usually if a
/// laptop is suspended for an hour, it doesn't make sense to try to simulate
/// the game logic for the elapsed hour when resuming. Instead it is better to
/// lose the extra time and pretend a shorter duration of time passed. Setting
/// [`max_delta()`](Time::max_delta) to a relatively short time means that the
/// impact on game logic will be minimal.
///
/// If the game lags for some reason, meaning that it will take a longer time to
/// compute a frame than the real time that passes during the computation, then
/// we would fall behind in processing virtual time. If this situation persists,
/// and computing a frame takes longer depending on how much virtual time has
/// passed, the game would enter a "death spiral" where computing each frame
/// takes longer and longer and the game will appear to freeze. By limiting the
/// maximum time that can be added at once, we also limit the amount of virtual
/// time the game needs to compute for each frame. This means that the game will
/// run slow, and it will run slower than real time, but it will not freeze and
/// it will recover as soon as computation becomes fast again.
///
/// You should set [`max_delta()`](Time::max_delta) to a value that is
/// approximately the minimum FPS your game should have even if heavily lagged
/// for a moment. The actual FPS when lagged will be somewhat lower than this,
/// depending on how much more time it takes to compute a frame compared to real
/// time. You should also consider how stable your FPS is, as the limit will
/// also dictate how big of an FPS drop you can accept without losing time and
/// falling behind real time.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Virtual {
    max_delta: Duration,
    paused: bool,
    relative_speed: f64,
    effective_speed: f64,
}

impl Time<Virtual> {
    /// The default amount of time that can added in a single update.
    ///
    /// Equal to 250 milliseconds.
    const DEFAULT_MAX_DELTA: Duration = Duration::from_millis(250);

    /// Create new virtual clock with given maximum delta step [`Duration`]
    ///
    /// # Panics
    ///
    /// Panics if `max_delta` is zero.
    pub fn from_max_delta(max_delta: Duration) -> Self {
        let mut ret = Self::default();
        ret.set_max_delta(max_delta);
        ret
    }

    /// Returns the maximum amount of time that can be added to this clock by a
    /// single update, as [`Duration`].
    ///
    /// This is the maximum value [`Self::delta()`] will return and also to
    /// maximum time [`Self::elapsed()`] will be increased by in a single
    /// update.
    ///
    /// This ensures that even if no updates happen for an extended amount of time,
    /// the clock will not have a sudden, huge advance all at once. This also indirectly
    /// limits the maximum number of fixed update steps that can run in a single update.
    ///
    /// The default value is 250 milliseconds.
    #[inline]
    pub fn max_delta(&self) -> Duration {
        self.context().max_delta
    }

    /// Sets the maximum amount of time that can be added to this clock by a
    /// single update, as [`Duration`].
    ///
    /// This is the maximum value [`Self::delta()`] will return and also to
    /// maximum time [`Self::elapsed()`] will be increased by in a single
    /// update.
    ///
    /// This is used to ensure that even if the game freezes for a few seconds,
    /// or is suspended for hours or even days, the virtual clock doesn't
    /// suddenly jump forward for that full amount, which would likely cause
    /// gameplay bugs or having to suddenly simulate all the intervening time.
    ///
    /// If no updates happen for an extended amount of time, this limit prevents
    /// having a sudden, huge advance all at once. This also indirectly limits
    /// the maximum number of fixed update steps that can run in a single
    /// update.
    ///
    /// The default value is 250 milliseconds. If you want to disable this
    /// feature, set the value to [`Duration::MAX`].
    ///
    /// # Panics
    ///
    /// Panics if `max_delta` is zero.
    #[inline]
    pub fn set_max_delta(&mut self, max_delta: Duration) {
        assert_ne!(max_delta, Duration::ZERO, "tried to set max delta to zero");
        self.context_mut().max_delta = max_delta;
    }

    /// Returns the speed the clock advances relative to your system clock, as [`f32`].
    /// This is known as "time scaling" or "time dilation" in other engines.
    #[inline]
    pub fn relative_speed(&self) -> f32 {
        self.relative_speed_f64() as f32
    }

    /// Returns the speed the clock advances relative to your system clock, as [`f64`].
    /// This is known as "time scaling" or "time dilation" in other engines.
    #[inline]
    pub fn relative_speed_f64(&self) -> f64 {
        self.context().relative_speed
    }

    /// Returns the speed the clock advanced relative to your system clock in
    /// this update, as [`f32`].
    ///
    /// Returns `0.0` if the game was paused or what the `relative_speed` value
    /// was at the start of this update.
    #[inline]
    pub fn effective_speed(&self) -> f32 {
        self.context().effective_speed as f32
    }

    /// Returns the speed the clock advanced relative to your system clock in
    /// this update, as [`f64`].
    ///
    /// Returns `0.0` if the game was paused or what the `relative_speed` value
    /// was at the start of this update.
    #[inline]
    pub fn effective_speed_f64(&self) -> f64 {
        self.context().effective_speed
    }

    /// Sets the speed the clock advances relative to your system clock, given as an [`f32`].
    ///
    /// For example, setting this to `2.0` will make the clock advance twice as fast as your system
    /// clock.
    ///
    /// # Panics
    ///
    /// Panics if `ratio` is negative or not finite.
    #[inline]
    pub fn set_relative_speed(&mut self, ratio: f32) {
        self.set_relative_speed_f64(ratio as f64);
    }

    /// Sets the speed the clock advances relative to your system clock, given as an [`f64`].
    ///
    /// For example, setting this to `2.0` will make the clock advance twice as fast as your system
    /// clock.
    ///
    /// # Panics
    ///
    /// Panics if `ratio` is negative or not finite.
    #[inline]
    pub fn set_relative_speed_f64(&mut self, ratio: f64) {
        assert!(ratio.is_finite(), "tried to go infinitely fast");
        assert!(ratio >= 0.0, "tried to go back in time");
        self.context_mut().relative_speed = ratio;
    }

    /// Stops the clock, preventing it from advancing until resumed.
    #[inline]
    pub fn pause(&mut self) {
        self.context_mut().paused = true;
    }

    /// Resumes the clock if paused.
    #[inline]
    pub fn unpause(&mut self) {
        self.context_mut().paused = false;
    }

    /// Returns `true` if the clock is currently paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.context().paused
    }

    /// Returns `true` if the clock was paused at the start of this update.
    #[inline]
    pub fn was_paused(&self) -> bool {
        self.context().effective_speed == 0.0
    }

    /// Updates the elapsed duration of `self` by `raw_delta`, up to the `max_delta`.
    fn advance_with_raw_delta(&mut self, raw_delta: Duration) {
        let max_delta = self.context().max_delta;
        let clamped_delta = if raw_delta > max_delta {
            debug!(
                "delta time larger than maximum delta, clamping delta to {:?} and skipping {:?}",
                max_delta,
                raw_delta - max_delta
            );
            max_delta
        } else {
            raw_delta
        };
        let effective_speed = if self.context().paused {
            0.0
        } else {
            self.context().relative_speed
        };
        let delta = if effective_speed != 1.0 {
            clamped_delta.mul_f64(effective_speed)
        } else {
            // avoid rounding when at normal speed
            clamped_delta
        };
        self.context_mut().effective_speed = effective_speed;
        self.advance_by(delta);
    }
}

impl Default for Virtual {
    fn default() -> Self {
        Self {
            max_delta: Time::<Virtual>::DEFAULT_MAX_DELTA,
            paused: false,
            relative_speed: 1.0,
            effective_speed: 1.0,
        }
    }
}

/// Advances [`Time<Virtual>`] and [`Time`] based on the elapsed [`Time<Real>`].
///
/// The virtual time will be advanced up to the provided [`Time::max_delta`].
pub fn update_virtual_time(current: &mut Time, virt: &mut Time<Virtual>, real: &Time<Real>) {
    let raw_delta = real.delta();
    virt.advance_with_raw_delta(raw_delta);
    *current = virt.as_generic();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default() {
        let time = Time::<Virtual>::default();

        assert!(!time.is_paused()); // false
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.max_delta(), Time::<Virtual>::DEFAULT_MAX_DELTA);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_advance() {
        let mut time = Time::<Virtual>::default();

        time.advance_with_raw_delta(Duration::from_millis(125));

        assert_eq!(time.delta(), Duration::from_millis(125));
        assert_eq!(time.elapsed(), Duration::from_millis(125));

        time.advance_with_raw_delta(Duration::from_millis(125));

        assert_eq!(time.delta(), Duration::from_millis(125));
        assert_eq!(time.elapsed(), Duration::from_millis(250));

        time.advance_with_raw_delta(Duration::from_millis(125));

        assert_eq!(time.delta(), Duration::from_millis(125));
        assert_eq!(time.elapsed(), Duration::from_millis(375));

        time.advance_with_raw_delta(Duration::from_millis(125));

        assert_eq!(time.delta(), Duration::from_millis(125));
        assert_eq!(time.elapsed(), Duration::from_millis(500));
    }

    #[test]
    fn test_relative_speed() {
        let mut time = Time::<Virtual>::default();

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);
        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.elapsed(), Duration::from_millis(250));

        time.set_relative_speed_f64(2.0);

        assert_eq!(time.relative_speed(), 2.0);
        assert_eq!(time.effective_speed(), 1.0);

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert_eq!(time.relative_speed(), 2.0);
        assert_eq!(time.effective_speed(), 2.0);
        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.elapsed(), Duration::from_millis(750));

        time.set_relative_speed_f64(0.5);

        assert_eq!(time.relative_speed(), 0.5);
        assert_eq!(time.effective_speed(), 2.0);

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert_eq!(time.relative_speed(), 0.5);
        assert_eq!(time.effective_speed(), 0.5);
        assert_eq!(time.delta(), Duration::from_millis(125));
        assert_eq!(time.elapsed(), Duration::from_millis(875));
    }

    #[test]
    fn test_pause() {
        let mut time = Time::<Virtual>::default();

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert!(!time.is_paused()); // false
        assert!(!time.was_paused()); // false
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);
        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.elapsed(), Duration::from_millis(250));

        time.pause();

        assert!(time.is_paused()); // true
        assert!(!time.was_paused()); // false
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert!(time.is_paused()); // true
        assert!(time.was_paused()); // true
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 0.0);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::from_millis(250));

        time.unpause();

        assert!(!time.is_paused()); // false
        assert!(time.was_paused()); // true
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 0.0);

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert!(!time.is_paused()); // false
        assert!(!time.was_paused()); // false
        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);
        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.elapsed(), Duration::from_millis(500));
    }

    #[test]
    fn test_max_delta() {
        let mut time = Time::<Virtual>::default();
        time.set_max_delta(Duration::from_millis(500));

        time.advance_with_raw_delta(Duration::from_millis(250));

        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.elapsed(), Duration::from_millis(250));

        time.advance_with_raw_delta(Duration::from_millis(500));

        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.elapsed(), Duration::from_millis(750));

        time.advance_with_raw_delta(Duration::from_millis(750));

        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.elapsed(), Duration::from_millis(1250));

        time.set_max_delta(Duration::from_secs(1));

        assert_eq!(time.max_delta(), Duration::from_secs(1));

        time.advance_with_raw_delta(Duration::from_millis(750));

        assert_eq!(time.delta(), Duration::from_millis(750));
        assert_eq!(time.elapsed(), Duration::from_millis(2000));

        time.advance_with_raw_delta(Duration::from_millis(1250));

        assert_eq!(time.delta(), Duration::from_millis(1000));
        assert_eq!(time.elapsed(), Duration::from_millis(3000));
    }
}
