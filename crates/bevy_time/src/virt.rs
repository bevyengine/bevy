#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::time::Duration;
use log::debug;

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
/// The virtual clock can be paused by calling [`pause()`](Time::pause),
/// unpaused by calling [`unpause()`](Time::unpause), or toggled by calling
/// [`toggle()`](Time::toggle). When the game clock is
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
/// The virtual clock has two independent limits, configured with
/// [`set_max_wall_delta()`](Time::set_max_wall_delta) and
/// [`set_max_delta()`](Time::set_max_delta).
///
/// [`max_wall_delta()`](Time::max_wall_delta) limits the amount of real time
/// considered by a single update, before relative speed is applied. This is
/// useful when the application freezes, blocks, or is suspended: instead of
/// including the entire time away in the next update, the excess is discarded.
/// Because this cap is applied before scaling, it behaves consistently at any
/// virtual clock speed.
///
/// [`max_delta()`](Time::max_delta) limits the amount of virtual time added in
/// a single update, after relative speed is applied. This protects against a
/// "death spiral" where a large virtual-time step requires excessive
/// catch-up work, making subsequent frames take even longer. When this cap is
/// reached, the virtual clock advances more slowly for that update, allowing
/// the game to recover once frame processing becomes faster.
///
/// Use `max_wall_delta` as a real-time hitch and suspension guard, and
/// `max_delta` to limit how much virtual time can be processed in one update.
/// The latter also indirectly limits how many fixed update steps can run in
/// one update.
///
/// You should set [`max_delta()`](Time::max_delta) to a value that is
/// approximately the minimum FPS your game should have even if heavily lagged
/// for a moment. The actual FPS when lagged will be somewhat lower than this,
/// depending on how much more time it takes to compute a frame compared to real
/// time. You should also consider how stable your FPS is, as the limit will
/// also dictate how big of an FPS drop you can accept without losing time and
/// falling behind real time.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Clone))]
pub struct Virtual {
    max_delta: Duration,
    max_wall_delta: Option<Duration>,
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

    /// Returns the optional maximum amount of real frame time that can be
    /// considered by a single update, before relative-speed scaling is applied.
    ///
    /// `None` means no real-time cap is configured.
    #[inline]
    pub fn max_wall_delta(&self) -> Option<Duration> {
        self.context().max_wall_delta
    }

    /// Sets the maximum amount of real frame time that can be considered by a
    /// single update, before relative-speed scaling is applied.
    ///
    /// This guards against large frame hitches and system suspensions regardless
    /// of the virtual clock's relative speed. For example, with a 150 ms cap and
    /// a relative speed of `10.0`, a 1 second stall advances virtual time by at
    /// most 1.5 seconds.
    ///
    /// The wall-time cap is applied first, followed by relative-speed scaling,
    /// and finally by [`max_delta()`](Self::max_delta).
    ///
    /// Pass `None` to disable the wall-time cap.
    ///
    /// # Panics
    ///
    /// Panics if `max_wall_delta` is `Some(Duration::ZERO)`.
    #[inline]
    pub fn set_max_wall_delta(&mut self, max_wall_delta: Option<Duration>) {
        assert_ne!(
            max_wall_delta,
            Some(Duration::ZERO),
            "tried to set max wall delta to zero"
        );
        self.context_mut().max_wall_delta = max_wall_delta;
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
    /// Returns `0.0` if the game was paused. Otherwise returns the multiplier
    /// actually applied to the `raw_delta` this update. This will usually equal
    /// [`relative_speed()`](Self::relative_speed), but if the delta was clamped by
    /// [`max_delta()`](Self::max_delta), it will be less than [`relative_speed()`](Self::relative_speed).
    #[inline]
    pub fn effective_speed(&self) -> f32 {
        self.context().effective_speed as f32
    }

    /// Returns the speed the clock advanced relative to your system clock in
    /// this update, as [`f64`].
    ///
    /// Returns `0.0` if the game was paused. Otherwise returns the multiplier
    /// actually applied to the `raw_delta` this update. This will usually equal
    /// [`relative_speed()`](Self::relative_speed), but if the delta was clamped by
    /// [`max_delta()`](Self::max_delta), it will be less than [`relative_speed()`](Self::relative_speed).
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

    /// Stops the clock if it is running, otherwise resumes the clock.
    #[inline]
    pub fn toggle(&mut self) {
        self.context_mut().paused ^= true;
    }

    /// Stops the clock, preventing it from advancing until resumed.
    #[inline]
    pub fn pause(&mut self) {
        self.context_mut().paused = true;
    }

    /// Resumes the clock.
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

    /// Updates the elapsed duration of `self` using `raw_delta`, first limiting
    /// the real-time delta by `max_wall_delta`, then applying `relative_speed`,
    /// and finally limiting the result by `max_delta`.
    fn advance_with_raw_delta(&mut self, raw_delta: Duration) {
        let max_delta = self.context().max_delta;
        let speed = if self.context().paused {
            0.0
        } else {
            self.context().relative_speed
        };
        let wall_delta = match self.context().max_wall_delta {
            Some(max_wall_delta) if raw_delta > max_wall_delta => {
                debug!(
                    "wall delta time larger than maximum wall delta, clamping it to {:?} and skipping {:?}",
                    max_wall_delta,
                    raw_delta - max_wall_delta
                );
                max_wall_delta
            }
            _ => raw_delta,
        };
        let scaled = if speed != 1.0 {
            wall_delta.mul_f64(speed)
        } else {
            // avoid rounding when at normal speed
            wall_delta
        };
        let (effective_speed, delta) = if scaled > max_delta {
            debug!(
                "delta time larger than maximum delta, clamping delta to {:?} and skipping {:?}",
                max_delta,
                scaled - max_delta
            );
            (max_delta.as_secs_f64() / raw_delta.as_secs_f64(), max_delta)
        } else if wall_delta < raw_delta {
            (scaled.as_secs_f64() / raw_delta.as_secs_f64(), scaled)
        } else {
            (speed, scaled)
        };
        self.context_mut().effective_speed = effective_speed;
        self.advance_by(delta);
    }
}

impl Default for Virtual {
    fn default() -> Self {
        Self {
            max_delta: Time::<Virtual>::DEFAULT_MAX_DELTA,
            max_wall_delta: None,
            paused: false,
            relative_speed: 1.0,
            effective_speed: 1.0,
        }
    }
}

/// Advances [`Time<Virtual>`] and [`Time`] based on the elapsed [`Time<Real>`].
///
/// The real delta is first limited by `max_wall_delta`, then scaled by the
/// virtual clock's relative speed, and finally limited by `max_delta`.
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
        assert_eq!(time.max_wall_delta(), None);
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
        time.set_max_delta(Duration::from_secs(1));

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

        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);
        assert_eq!(time.delta(), Duration::from_millis(250));
        assert_eq!(time.elapsed(), Duration::from_millis(250));

        time.advance_with_raw_delta(Duration::from_millis(500));

        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);
        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.elapsed(), Duration::from_millis(750));

        time.advance_with_raw_delta(Duration::from_millis(750));

        assert_eq!(time.relative_speed(), 1.0);
        assert!((time.effective_speed() - 500.0 / 750.0).abs() < f32::EPSILON);
        assert_eq!(time.delta(), Duration::from_millis(500));
        assert_eq!(time.elapsed(), Duration::from_millis(1250));

        time.set_max_delta(Duration::from_secs(1));

        assert_eq!(time.max_delta(), Duration::from_secs(1));

        time.advance_with_raw_delta(Duration::from_millis(750));

        assert_eq!(time.relative_speed(), 1.0);
        assert_eq!(time.effective_speed(), 1.0);
        assert_eq!(time.delta(), Duration::from_millis(750));
        assert_eq!(time.elapsed(), Duration::from_millis(2000));

        time.advance_with_raw_delta(Duration::from_millis(1250));

        assert_eq!(time.relative_speed(), 1.0);
        assert!((time.effective_speed() - 1000.0 / 1250.0).abs() < f32::EPSILON);
        assert_eq!(time.delta(), Duration::from_millis(1000));
        assert_eq!(time.elapsed(), Duration::from_millis(3000));
    }

    #[test]
    fn test_max_delta_clamps_after_relative_speed() {
        let mut time = Time::<Virtual>::default();
        time.set_relative_speed_f64(2000.0);
        time.set_max_delta(Duration::from_secs(1));

        time.advance_with_raw_delta(Duration::from_millis(16));

        assert_eq!(time.delta(), time.max_delta());
        // 62.5 = max_delta / raw_delta = 1000 / 16
        assert_eq!(time.effective_speed(), 62.5);
    }

    #[test]
    fn test_dont_overclamp_at_low_speed() {
        let mut time = Time::<Virtual>::default();
        time.set_relative_speed_f64(0.01);
        time.set_max_delta(Duration::from_millis(10));
        let delta = Duration::from_millis(16);

        time.advance_with_raw_delta(delta);

        assert_eq!(time.delta(), delta / 100);
    }

    #[test]
    fn test_max_wall_delta_clamps_before_relative_speed() {
        let mut time = Time::<Virtual>::default();
        time.set_max_delta(Duration::MAX);
        time.set_max_wall_delta(Some(Duration::from_millis(150)));

        let mut elapsed = Duration::ZERO;
        for speed in [10.0, 240.0, 0.1, 1.0] {
            time.set_relative_speed_f64(speed);
            time.advance_with_raw_delta(Duration::from_secs(1));

            let expected = Duration::from_millis(150).mul_f64(speed);
            elapsed += expected;
            assert_eq!(time.delta(), expected, "relative speed {speed}");
            assert_eq!(time.elapsed(), elapsed, "relative speed {speed}");
            assert!((time.effective_speed_f64() - 0.15 * speed).abs() < 1e-9);
        }
    }

    #[test]
    fn test_max_wall_delta_and_max_delta_both_apply() {
        let mut time = Time::<Virtual>::default();
        time.set_max_delta(Duration::from_secs(1));
        time.set_max_wall_delta(Some(Duration::from_millis(150)));
        time.set_relative_speed_f64(10.0);

        time.advance_with_raw_delta(Duration::from_millis(100));
        assert_eq!(time.delta(), Duration::from_secs(1));
        assert_eq!(time.effective_speed_f64(), 10.0);

        time.advance_with_raw_delta(Duration::from_secs(1));
        assert_eq!(time.delta(), Duration::from_secs(1));
        assert_eq!(time.effective_speed_f64(), 1.0);

        time.set_relative_speed_f64(2.0);
        time.advance_with_raw_delta(Duration::from_secs(1));
        assert_eq!(time.delta(), Duration::from_millis(300));
        assert!((time.effective_speed_f64() - 0.3).abs() < 1e-9);

        time.set_relative_speed_f64(10.0);
        time.set_max_wall_delta(None);
        time.set_max_delta(Duration::MAX);
        time.advance_with_raw_delta(Duration::from_secs(1));
        assert_eq!(time.delta(), Duration::from_secs(10));
        assert_eq!(time.effective_speed_f64(), 10.0);
    }

    #[test]
    fn test_max_wall_delta_while_paused() {
        let mut time = Time::<Virtual>::default();
        time.set_max_wall_delta(Some(Duration::from_millis(150)));
        time.pause();

        time.advance_with_raw_delta(Duration::from_secs(1));

        assert_eq!(time.delta(), Duration::ZERO);
        assert!(time.was_paused());
    }

    #[test]
    #[should_panic(expected = "tried to set max wall delta to zero")]
    fn test_max_wall_delta_zero_panics() {
        Time::<Virtual>::default().set_max_wall_delta(Some(Duration::ZERO));
    }
}
