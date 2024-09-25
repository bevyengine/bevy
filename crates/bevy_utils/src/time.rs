#[cfg(target_arch = "wasm32")]
pub use web_time::{Duration, Instant, SystemTime, SystemTimeError, TryFromFloatSecsError};

#[cfg(all(not(target_arch = "wasm32"), feature = "std"))]
pub use {
    core::time::{Duration, TryFromFloatSecsError},
    std::time::{Instant, SystemTime, SystemTimeError},
};

#[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
pub use no_std::{Duration, Instant, SystemTime, SystemTimeError, TryFromFloatSecsError};

#[cfg(all(not(target_arch = "wasm32"), not(feature = "std")))]
mod no_std {
    use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    pub use core::time::{Duration, TryFromFloatSecsError};

    /// Custom `no_std` compatible implementation of `Instant`.
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Instant(Duration);

    /// Custom `no_std` compatible implementation of `SystemTime`.
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SystemTime(Instant);

    /// Custom `no_std` compatible implementation of `SystemTimeError`.
    #[derive(Clone, Debug)]
    pub struct SystemTimeError(Duration);

    static SECONDS: AtomicU64 = AtomicU64::new(0);
    static SUBSECOND_NANOS: AtomicU32 = AtomicU32::new(0);

    impl Instant {
        /// Returns an instant corresponding to "now".
        #[must_use]
        pub fn now() -> Instant {
            let seconds = SECONDS.load(Ordering::Relaxed);
            let subsecond_nanos = SUBSECOND_NANOS.load(Ordering::Relaxed);

            Self(Duration::new(seconds, subsecond_nanos))
        }

        /// Update the current time.
        ///
        /// # Safety
        ///
        /// The provided duration must _always_ be equal to or greater than the current
        /// value to preserve `Instant`'s monotonicity guarantees.
        pub unsafe fn update(duration: Duration) {}

        /// Returns the amount of time elapsed from another instant to this one,
        /// or zero duration if that instant is later than this one.
        #[must_use]
        pub fn duration_since(&self, earlier: Instant) -> Duration {
            self.checked_duration_since(earlier).unwrap_or_default()
        }

        /// Returns the amount of time elapsed from another instant to this one,
        /// or None if that instant is later than this one.
        ///
        /// Due to [monotonicity bugs], even under correct logical ordering of the passed `Instant`s,
        /// this method can return `None`.
        ///
        /// [monotonicity bugs]: Instant#monotonicity
        #[must_use]
        pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
            self.0.checked_sub(earlier.0)
        }

        /// Returns the amount of time elapsed from another instant to this one,
        /// or zero duration if that instant is later than this one.
        #[must_use]
        pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
            self.checked_duration_since(earlier).unwrap_or_default()
        }

        /// Returns the amount of time elapsed since this instant.
        #[must_use]
        pub fn elapsed(&self) -> Duration {
            Instant::now() - *self
        }

        /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented as
        /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
        /// otherwise.
        pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
            self.0.checked_add(duration).map(Instant)
        }

        /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented as
        /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
        /// otherwise.
        pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
            self.0.checked_sub(duration).map(Instant)
        }
    }

    impl core::ops::Add<Duration> for Instant {
        type Output = Instant;

        /// # Panics
        ///
        /// This function may panic if the resulting point in time cannot be represented by the
        /// underlying data structure. See [`Instant::checked_add`] for a version without panic.
        fn add(self, other: Duration) -> Instant {
            self.checked_add(other)
                .expect("overflow when adding duration to instant")
        }
    }

    impl core::ops::AddAssign<Duration> for Instant {
        fn add_assign(&mut self, other: Duration) {
            *self = *self + other;
        }
    }

    impl core::ops::Sub<Duration> for Instant {
        type Output = Instant;

        fn sub(self, other: Duration) -> Instant {
            self.checked_sub(other)
                .expect("overflow when subtracting duration from instant")
        }
    }

    impl core::ops::SubAssign<Duration> for Instant {
        fn sub_assign(&mut self, other: Duration) {
            *self = *self - other;
        }
    }

    impl core::ops::Sub<Instant> for Instant {
        type Output = Duration;

        /// Returns the amount of time elapsed from another instant to this one,
        /// or zero duration if that instant is later than this one.
        fn sub(self, other: Instant) -> Duration {
            self.duration_since(other)
        }
    }

    impl core::fmt::Debug for Instant {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            self.0.fmt(f)
        }
    }
}
