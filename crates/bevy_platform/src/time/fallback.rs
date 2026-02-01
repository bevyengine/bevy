//! Provides a fallback implementation of `Instant` from the standard library.

#![expect(
    unsafe_code,
    reason = "Instant fallback requires unsafe to allow users to update the internal value"
)]

use crate::sync::atomic::{AtomicPtr, Ordering};

use core::{
    fmt,
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

static ELAPSED_GETTER: AtomicPtr<()> = AtomicPtr::new(unset_getter as *mut _);

/// Fallback implementation of `Instant` suitable for a `no_std` environment.
///
/// If you are on any of the following target architectures, this is a drop-in replacement:
///
/// - `x86`
/// - `x86_64`
/// - `aarch64`
///
/// On any other architecture, you must call [`Instant::set_elapsed`], providing a method
/// which when called supplies a monotonically increasing count of elapsed nanoseconds relative
/// to some arbitrary point in time.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(Duration);

impl Instant {
    /// Returns an instant corresponding to "now".
    #[must_use]
    pub fn now() -> Instant {
        let getter = ELAPSED_GETTER.load(Ordering::Acquire);

        // SAFETY: Function pointer is always valid
        let getter = unsafe { core::mem::transmute::<*mut (), fn() -> Duration>(getter) };

        Self((getter)())
    }

    /// Provides a function returning the amount of time that has elapsed since execution began.
    /// The getter provided to this method will be used by [`now`](Instant::now).
    ///
    /// # Safety
    ///
    /// - The function provided must accurately represent the elapsed time.
    /// - The function must preserve all invariants of the [`Instant`] type.
    /// - The pointer to the function must be valid whenever [`Instant::now`] is called.
    pub unsafe fn set_elapsed(getter: fn() -> Duration) {
        ELAPSED_GETTER.store(getter as *mut _, Ordering::Release);
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    #[must_use]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.saturating_duration_since(earlier)
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or None if that instant is later than this one.
    ///
    /// Due to monotonicity bugs, even under correct logical ordering of the passed `Instant`s,
    /// this method can return `None`.
    #[must_use]
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    #[must_use]
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    /// Returns the amount of time elapsed since this instant.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        Instant::now().saturating_duration_since(*self)
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

impl Add<Duration> for Instant {
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

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, other: Duration) -> Instant {
        self.checked_sub(other)
            .expect("overflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

fn unset_getter() -> Duration {
    crate::cfg::switch! {
        #[cfg(target_arch = "x86")] => {
            // SAFETY: standard technique for getting a nanosecond counter on x86
            let nanos = unsafe {
                core::arch::x86::_rdtsc()
            };
            Duration::from_nanos(nanos)
        }
        #[cfg(target_arch = "x86_64")] => {
            // SAFETY: standard technique for getting a nanosecond counter on x86_64
            let nanos = unsafe {
                core::arch::x86_64::_rdtsc()
            };
            Duration::from_nanos(nanos)
        }
        #[cfg(target_arch = "aarch64")] => {
            // SAFETY: standard technique for getting a nanosecond counter of aarch64
            let nanos = unsafe {
                let mut ticks: u64;
                core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks);
                ticks
            };
            Duration::from_nanos(nanos)
        }
        _ => {
            panic!("An elapsed time getter has not been provided to `Instant`. Please use `Instant::set_elapsed(...)` before calling `Instant::now()`")
        }
    }
}
