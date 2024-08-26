#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::{Duration, Instant};

use crate::time::Time;

/// Real time clock representing elapsed wall clock time.
///
/// A specialization of the [`Time`] structure. **For method documentation, see
/// [`Time<Real>#impl-Time<Real>`].**
///
/// It is automatically inserted as a resource by
/// [`TimePlugin`](crate::TimePlugin) and updated with time instants according
/// to [`TimeUpdateStrategy`](crate::TimeUpdateStrategy).
///
/// The [`delta()`](Time::delta) and [`elapsed()`](Time::elapsed) values of this
/// clock should be used for anything which deals specifically with real time
/// (wall clock time). It will not be affected by relative game speed
/// adjustments, pausing or other adjustments.
///
/// The clock does not count time from [`startup()`](Time::startup) to
/// [`first_update()`](Time::first_update()) into elapsed, but instead will
/// start counting time from the first update call. [`delta()`](Time::delta) and
/// [`elapsed()`](Time::elapsed) will report zero on the first update as there
/// is no previous update instant. This means that a [`delta()`](Time::delta) of
/// zero must be handled without errors in application logic, as it may
/// theoretically also happen at other times.
///
/// [`Instant`]s for [`startup()`](Time::startup),
/// [`first_update()`](Time::first_update) and
/// [`last_update()`](Time::last_update) are recorded and accessible.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Real {
    startup: Instant,
    first_update: Option<Instant>,
    last_update: Option<Instant>,
}

impl Default for Real {
    fn default() -> Self {
        Self {
            startup: Instant::now(),
            first_update: None,
            last_update: None,
        }
    }
}

impl Time<Real> {
    /// Constructs a new `Time<Real>` instance with a specific startup
    /// [`Instant`].
    pub fn new(startup: Instant) -> Self {
        Self::new_with(Real {
            startup,
            ..Default::default()
        })
    }

    /// Updates the internal time measurements.
    ///
    /// Calling this method as part of your app will most likely result in
    /// inaccurate timekeeping, as the [`Time`] resource is ordinarily managed
    /// by the [`TimePlugin`](crate::TimePlugin).
    pub fn update(&mut self) {
        let instant = Instant::now();
        self.update_with_instant(instant);
    }

    /// Updates time with a specified [`Duration`].
    ///
    /// This method is provided for use in tests.
    ///
    /// Calling this method as part of your app will most likely result in
    /// inaccurate timekeeping, as the [`Time`] resource is ordinarily managed
    /// by the [`TimePlugin`](crate::TimePlugin).
    pub fn update_with_duration(&mut self, duration: Duration) {
        let last_update = self.context().last_update.unwrap_or(self.context().startup);
        self.update_with_instant(last_update + duration);
    }

    /// Updates time with a specified [`Instant`].
    ///
    /// This method is provided for use in tests.
    ///
    /// Calling this method as part of your app will most likely result in inaccurate timekeeping,
    /// as the [`Time`] resource is ordinarily managed by the [`TimePlugin`](crate::TimePlugin).
    pub fn update_with_instant(&mut self, instant: Instant) {
        let Some(last_update) = self.context().last_update else {
            let context = self.context_mut();
            context.first_update = Some(instant);
            context.last_update = Some(instant);
            return;
        };
        let delta = instant - last_update;
        self.advance_by(delta);
        self.context_mut().last_update = Some(instant);
    }

    /// Returns the [`Instant`] the clock was created.
    ///
    /// This usually represents when the app was started.
    #[inline]
    pub fn startup(&self) -> Instant {
        self.context().startup
    }

    /// Returns the [`Instant`] when [`Self::update`] was first called, if it
    /// exists.
    ///
    /// This usually represents when the first app update started.
    #[inline]
    pub fn first_update(&self) -> Option<Instant> {
        self.context().first_update
    }

    /// Returns the [`Instant`] when [`Self::update`] was last called, if it
    /// exists.
    ///
    /// This usually represents when the current app update started.
    #[inline]
    pub fn last_update(&self) -> Option<Instant> {
        self.context().last_update
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_update() {
        let startup = Instant::now();
        let mut time = Time::<Real>::new(startup);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), None);
        assert_eq!(time.last_update(), None);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);

        time.update();

        assert_ne!(time.first_update(), None);
        assert_ne!(time.last_update(), None);
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);

        time.update();

        assert_ne!(time.first_update(), None);
        assert_ne!(time.last_update(), None);
        assert_ne!(time.last_update(), time.first_update());
        assert_ne!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), time.delta());

        let prev_elapsed = time.elapsed();
        time.update();

        assert_ne!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), prev_elapsed + time.delta());
    }

    #[test]
    fn test_update_with_instant() {
        let startup = Instant::now();
        let mut time = Time::<Real>::new(startup);

        let first_update = Instant::now();
        time.update_with_instant(first_update);

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(first_update));
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);

        let second_update = Instant::now();
        time.update_with_instant(second_update);

        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(second_update));
        assert_eq!(time.delta(), second_update - first_update);
        assert_eq!(time.elapsed(), second_update - first_update);

        let third_update = Instant::now();
        time.update_with_instant(third_update);

        assert_eq!(time.first_update(), Some(first_update));
        assert_eq!(time.last_update(), Some(third_update));
        assert_eq!(time.delta(), third_update - second_update);
        assert_eq!(time.elapsed(), third_update - first_update);
    }

    #[test]
    fn test_update_with_duration() {
        let startup = Instant::now();
        let mut time = Time::<Real>::new(startup);

        time.update_with_duration(Duration::from_secs(1));

        assert_eq!(time.startup(), startup);
        assert_eq!(time.first_update(), Some(startup + Duration::from_secs(1)));
        assert_eq!(time.last_update(), Some(startup + Duration::from_secs(1)));
        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);

        time.update_with_duration(Duration::from_secs(1));

        assert_eq!(time.first_update(), Some(startup + Duration::from_secs(1)));
        assert_eq!(time.last_update(), Some(startup + Duration::from_secs(2)));
        assert_eq!(time.delta(), Duration::from_secs(1));
        assert_eq!(time.elapsed(), Duration::from_secs(1));

        time.update_with_duration(Duration::from_secs(1));

        assert_eq!(time.first_update(), Some(startup + Duration::from_secs(1)));
        assert_eq!(time.last_update(), Some(startup + Duration::from_secs(3)));
        assert_eq!(time.delta(), Duration::from_secs(1));
        assert_eq!(time.elapsed(), Duration::from_secs(2));
    }
}
