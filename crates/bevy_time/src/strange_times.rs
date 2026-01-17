use core::time::Duration;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

use crate::{Time, TimeDuration, TimeDurationPrecompute};

/// Stepped time, augmented by 1 every frame
#[derive(Debug, Copy, Clone, Default)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Clone))]
pub struct Stepped;

/// Stepped time, augmented by a relative speed every frame
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Clone))]
pub struct SteppedVirtual {
    paused: bool,
    relative_speed: u32,
    effective_speed: u32,
}

impl Default for SteppedVirtual {
    fn default() -> Self {
        Self {
            paused: false,
            relative_speed: 1,
            effective_speed: 1,
        }
    }
}

impl TimeDuration for u32 {
    type Precompute = ();

    const ZERO: Self = 0;

    const DEFAULT_WRAP_PERIOD: Self = u32::MAX;

    fn wrap(value: Self, wrap_period: Self) -> Self {
        value % wrap_period
    }
}

impl Time<SteppedVirtual, u32> {
    /// Returns the speed the clock advances relative to the number of frames.
    #[inline]
    pub fn relative_speed(&self) -> u32 {
        self.context().relative_speed
    }

    /// Returns the speed the clock advanced relative to the number of frames in
    /// this update.
    ///
    /// Returns `0` if the game was paused or what the `relative_speed` value
    /// was at the start of this update.
    #[inline]
    pub fn effective_speed(&self) -> u32 {
        self.context().effective_speed
    }

    /// Sets the speed the clock advances relative to the number of frames.
    ///
    /// For example, setting this to `2` will make the clock advance twice as fast as the
    /// number of frames.
    #[inline]
    pub fn set_relative_speed(&mut self, ratio: u32) {
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
        self.context().effective_speed == 0
    }

    pub(crate) fn advance_with_step_count(&mut self, step_count: u32) {
        let effective_speed = if self.context().paused {
            0
        } else {
            self.context().relative_speed
        };
        let delta = step_count * effective_speed;
        self.context_mut().effective_speed = effective_speed;
        self.advance_by(delta);
    }
}

/// Stepped time that can go back.
#[derive(Debug, Copy, Clone, Default)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Clone))]
pub struct SteppedTimeTravel;

impl Time<SteppedTimeTravel, u32> {
    /// Set the time to a specific value.
    pub fn set_to(&mut self, target: u32) {
        if target > self.elapsed {
            self.advance_by(target - self.elapsed);
        } else {
            assert!(
                self.elapsed > target,
                "tried to move time backwards to before the start"
            );
            self.recede_by(self.elapsed - target);
        }
    }

    /// Recede the time by a specific amount.
    pub fn recede_by(&mut self, delta: u32) {
        self.delta = delta;
        self.elapsed -= delta;
        self.elapsed_wrapped = u32::wrap(self.elapsed, self.wrap_period);
    }
}

/// Time that can go back.
#[derive(Debug, Copy, Clone, Default)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Clone))]
pub struct TimeTravel;

impl Time<TimeTravel, Duration> {
    /// Set the time to a specific duration.
    pub fn set_to(&mut self, target: Duration) {
        if self.elapsed <= target {
            self.advance_by(target - self.elapsed);
        } else {
            assert!(
                self.elapsed > target,
                "tried to move time backwards to before the start"
            );
            self.recede_by(self.elapsed - target);
        }
    }

    /// Recede the time by a specific amount.
    pub fn recede_by(&mut self, delta: Duration) {
        self.delta = delta;
        self.precompute.update_delta(self.delta);
        self.elapsed -= delta;
        self.precompute.update_elapsed(self.elapsed);
        self.elapsed_wrapped = Duration::wrap(self.elapsed, self.wrap_period);
        self.precompute.update_elapsed_wrapped(self.elapsed_wrapped);
    }
}
