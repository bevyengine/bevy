use core::{
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

use bevy_reflect::Reflect;

use crate::Time;

/// A momentary measurement of some [`Time<C>`](crate::Time).
///
/// Bevy's equivalent of Rust's [`Instant`](std::time::Instant) but measured from a specific [`Time<C>`](crate::Time)
/// resource instead of the platforms monotonic clock.
///
/// `Moments` are captured from clocks, see [`Time::capture`].
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Moment<C = ()> {
    // How much time has elapsed since the source clock has been added to the world.
    elapsed: Duration,
    boo: PhantomData<C>,
}

impl<C> Moment<C> {
    /// Casts a moment from one clock source to another.
    ///
    /// Clocks may run at different rates meaning they may produce numerically
    /// different moments during the same world tick. This method is provided
    /// under the assumption you have a way to deal with this.
    pub fn cast<T>(self) -> Moment<T> {
        Moment {
            elapsed: self.elapsed,
            boo: PhantomData,
        }
    }

    /// Returns how much has passed from the first time the source clock started ticking
    /// to this moment.
    pub fn elapsed(self) -> Duration {
        self.elapsed
    }

    /// Offsets self by `offset`. Returns `None` if this would cause an overflow.
    pub fn checked_add(self, offset: Duration) -> Option<Self> {
        let elapsed = self.elapsed.checked_add(offset)?;

        Some(Self {
            elapsed,
            boo: PhantomData,
        })
    }

    /// Offsets self backwards by `offset`. Returns `None` if this would cause an overflow.
    pub fn checked_sub(self, offset: Duration) -> Option<Self> {
        let elapsed = self.elapsed.checked_sub(offset)?;

        Some(Self {
            elapsed,
            boo: PhantomData,
        })
    }

    /// Returns the amount of time since the other moment or zero duration if that moment came after
    /// this one.
    pub fn duration_since(self, earlier: Moment<C>) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    /// Returns the amount of time since the other moment or `None` if that moment came after
    /// this one.
    pub fn checked_duration_since(self, earlier: Moment<C>) -> Option<Duration> {
        if earlier > self {
            return None;
        }

        Some(self.elapsed - earlier.elapsed)
    }
}

impl<C: Default> Moment<C> {
    pub(crate) fn new(time: &Time<C>) -> Self {
        Self {
            elapsed: time.elapsed(),
            boo: PhantomData,
        }
    }
}

impl<C> Clone for Moment<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for Moment<C> {}

impl<C> Debug for Moment<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Moment")
            .field("elapsed", &self.elapsed)
            .finish()
    }
}

impl<C> Add<Duration> for Moment<C> {
    type Output = Self;

    fn add(mut self, rhs: Duration) -> Self::Output {
        self.elapsed += rhs;

        self
    }
}

impl<C> AddAssign<Duration> for Moment<C> {
    fn add_assign(&mut self, rhs: Duration) {
        self.elapsed += rhs;
    }
}

impl<C> PartialEq for Moment<C> {
    fn eq(&self, other: &Self) -> bool {
        self.elapsed == other.elapsed
    }
}

impl<C> Eq for Moment<C> {}

impl<C> PartialOrd for Moment<C> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<C> Ord for Moment<C> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.elapsed.cmp(&other.elapsed)
    }
}

impl<C> Hash for Moment<C> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.elapsed.hash(state);
    }
}

impl<C> Sub for Moment<C> {
    type Output = Duration;

    /// Returns the amount of time elapsed since the other moment or zero duration if that moment came after
    /// this one.
    fn sub(self, other: Self) -> Self::Output {
        self.duration_since(other)
    }
}
impl<C> Sub<Duration> for Moment<C> {
    type Output = Self;

    fn sub(mut self, rhs: Duration) -> Self::Output {
        self.elapsed -= rhs;

        self
    }
}

impl<C> SubAssign<Duration> for Moment<C> {
    fn sub_assign(&mut self, rhs: Duration) {
        self.elapsed -= rhs;
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use crate::Time;

    #[test]
    fn moments_are_ordered() {
        const TIME_OFFSET: Duration = Duration::from_secs(1);

        let mut time: Time<()> = Time::default();

        let moment_0 = time.capture();

        time.advance_by(TIME_OFFSET);

        let moment_1_1 = time.capture();
        let moment_1_2 = time.capture();

        time.advance_by(TIME_OFFSET);

        let moment_2 = time.capture();

        assert!(moment_0 < moment_1_1);
        // Capturing the same moment again should be idempotent
        assert!(moment_1_1 == moment_1_2);
        // Might as well check the cmp implementation invariants
        assert!(moment_1_1 <= moment_1_2);
        assert!(moment_1_1 >= moment_1_2);

        assert!(moment_1_1 < moment_2);
        assert!(moment_1_2 < moment_2);
        assert!(moment_0 < moment_2);
    }

    #[test]
    fn moments_can_be_offset() {
        const TIME_OFFSET: Duration = Duration::from_secs(1);

        let mut time: Time<()> = Time::default();
        time.advance_to(TIME_OFFSET);

        let moment_0 = time.capture();

        assert_eq!(moment_0.checked_add(Duration::MAX), None);
        assert_eq!(
            moment_0.checked_add(TIME_OFFSET),
            Some(moment_0 + TIME_OFFSET)
        );
        assert_eq!(
            moment_0.checked_sub(TIME_OFFSET),
            Some(moment_0 - TIME_OFFSET)
        );

        assert_eq!(moment_0.checked_sub(TIME_OFFSET * 2), None);

        let moment_1 = moment_0 + TIME_OFFSET;
        let moment_neg_1 = moment_0 - TIME_OFFSET;

        assert!(moment_1 > moment_0);
        assert!(moment_0 > moment_neg_1);
        assert!(moment_1 > moment_neg_1);

        assert_eq!(moment_1 - TIME_OFFSET, moment_0);

        assert_eq!(moment_0 + Duration::ZERO, moment_0);
        assert_eq!(moment_0 - Duration::ZERO, moment_0);
        assert_eq!(moment_0.checked_add(Duration::ZERO), Some(moment_0));
        assert_eq!(moment_0.checked_sub(Duration::ZERO), Some(moment_0));
    }

    #[test]
    fn moments_offset_by_moments() {
        const TIME_OFFSET: Duration = Duration::from_secs(1);

        let mut time: Time<()> = Time::default();
        time.advance_by(TIME_OFFSET);

        let moment_0 = time.capture();

        time.advance_by(TIME_OFFSET);

        let moment_1 = time.capture();

        assert_eq!(moment_1 - moment_0 + moment_0.elapsed(), moment_1.elapsed());
    }
}
