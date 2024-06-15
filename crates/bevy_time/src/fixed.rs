use bevy_app::FixedMain;
use bevy_ecs::world::World;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::Duration;

use crate::{time::Time, virt::Virtual};

/// The fixed timestep game clock following virtual time.
///
/// A specialization of the [`Time`] structure. **For method documentation, see
/// [`Time<Fixed>#impl-Time<Fixed>`].**
///     
/// It is automatically inserted as a resource by
/// [`TimePlugin`](crate::TimePlugin) and updated based on
/// [`Time<Virtual>`](Virtual). The fixed clock is automatically set as the
/// generic [`Time`] resource during [`FixedUpdate`](bevy_app::FixedUpdate)
/// schedule processing.
///
/// The fixed timestep clock advances in fixed-size increments, which is
/// extremely useful for writing logic (like physics) that should have
/// consistent behavior, regardless of framerate.
///
/// The default [`timestep()`](Time::timestep) is 64 hertz, or 15625
/// microseconds. This value was chosen because using 60 hertz has the potential
/// for a pathological interaction with the monitor refresh rate where the game
/// alternates between running two fixed timesteps and zero fixed timesteps per
/// frame (for example when running two fixed timesteps takes longer than a
/// frame). Additionally, the value is a power of two which losslessly converts
/// into [`f32`] and [`f64`].
///
/// To run a system on a fixed timestep, add it to one of the [`FixedMain`]
/// schedules, most commonly [`FixedUpdate`](bevy_app::FixedUpdate).
///
/// This schedule is run a number of times between
/// [`PreUpdate`](bevy_app::PreUpdate) and [`Update`](bevy_app::Update)
/// according to the accumulated [`overstep()`](Time::overstep) time divided by
/// the [`timestep()`](Time::timestep). This means the schedule may run 0, 1 or
/// more times during a single update (which typically corresponds to a rendered
/// frame).
///
/// `Time<Fixed>` and the generic [`Time`] resource will report a
/// [`delta()`](Time::delta) equal to [`timestep()`](Time::timestep) and always
/// grow [`elapsed()`](Time::elapsed) by one [`timestep()`](Time::timestep) per
/// iteration.
///
/// The fixed timestep clock follows the [`Time<Virtual>`](Virtual) clock, which
/// means it is affected by [`pause()`](Time::pause),
/// [`set_relative_speed()`](Time::set_relative_speed) and
/// [`set_max_delta()`](Time::set_max_delta) from virtual time. If the virtual
/// clock is paused, the [`FixedUpdate`](bevy_app::FixedUpdate) schedule will
/// not run. It is guaranteed that the [`elapsed()`](Time::elapsed) time in
/// `Time<Fixed>` is always between the previous `elapsed()` and the current
/// `elapsed()` value in `Time<Virtual>`, so the values are compatible.
///
/// Changing the timestep size while the game is running should not normally be
/// done, as having a regular interval is the point of this schedule, but it may
/// be necessary for effects like "bullet-time" if the normal granularity of the
/// fixed timestep is too big for the slowed down time. In this case,
/// [`set_timestep()`](Time::set_timestep) and be called to set a new value. The
/// new value will be used immediately for the next run of the
/// [`FixedUpdate`](bevy_app::FixedUpdate) schedule, meaning that it will affect
/// the [`delta()`](Time::delta) value for the very next
/// [`FixedUpdate`](bevy_app::FixedUpdate), even if it is still during the same
/// frame. Any [`overstep()`](Time::overstep) present in the accumulator will be
/// processed according to the new [`timestep()`](Time::timestep) value.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Fixed {
    timestep: Duration,
    overstep: Duration,
}

impl Time<Fixed> {
    /// Corresponds to 64 Hz.
    const DEFAULT_TIMESTEP: Duration = Duration::from_micros(15625);

    /// Return new fixed time clock with given timestep as [`Duration`]
    ///
    /// # Panics
    ///
    /// Panics if `timestep` is zero.
    pub fn from_duration(timestep: Duration) -> Self {
        let mut ret = Self::default();
        ret.set_timestep(timestep);
        ret
    }

    /// Return new fixed time clock with given timestep seconds as `f64`
    ///
    /// # Panics
    ///
    /// Panics if `seconds` is zero, negative or not finite.
    pub fn from_seconds(seconds: f64) -> Self {
        let mut ret = Self::default();
        ret.set_timestep_seconds(seconds);
        ret
    }

    /// Return new fixed time clock with given timestep frequency in Hertz (1/seconds)
    ///
    /// # Panics
    ///
    /// Panics if `hz` is zero, negative or not finite.
    pub fn from_hz(hz: f64) -> Self {
        let mut ret = Self::default();
        ret.set_timestep_hz(hz);
        ret
    }

    /// Returns the amount of virtual time that must pass before the fixed
    /// timestep schedule is run again.
    #[inline]
    pub fn timestep(&self) -> Duration {
        self.context().timestep
    }

    /// Sets the amount of virtual time that must pass before the fixed timestep
    /// schedule is run again, as [`Duration`].
    ///
    /// Takes effect immediately on the next run of the schedule, respecting
    /// what is currently in [`Self::overstep`].
    ///
    /// # Panics
    ///
    /// Panics if `timestep` is zero.
    #[inline]
    pub fn set_timestep(&mut self, timestep: Duration) {
        assert_ne!(
            timestep,
            Duration::ZERO,
            "attempted to set fixed timestep to zero"
        );
        self.context_mut().timestep = timestep;
    }

    /// Sets the amount of virtual time that must pass before the fixed timestep
    /// schedule is run again, as seconds.
    ///
    /// Timestep is stored as a [`Duration`], which has fixed nanosecond
    /// resolution and will be converted from the floating point number.
    ///
    /// Takes effect immediately on the next run of the schedule, respecting
    /// what is currently in [`Self::overstep`].
    ///
    /// # Panics
    ///
    /// Panics if `seconds` is zero, negative or not finite.
    #[inline]
    pub fn set_timestep_seconds(&mut self, seconds: f64) {
        assert!(
            seconds.is_sign_positive(),
            "seconds less than or equal to zero"
        );
        assert!(seconds.is_finite(), "seconds is infinite");
        self.set_timestep(Duration::from_secs_f64(seconds));
    }

    /// Sets the amount of virtual time that must pass before the fixed timestep
    /// schedule is run again, as frequency.
    ///
    /// The timestep value is set to `1 / hz`, converted to a [`Duration`] which
    /// has fixed nanosecond resolution.
    ///
    /// Takes effect immediately on the next run of the schedule, respecting
    /// what is currently in [`Self::overstep`].
    ///
    /// # Panics
    ///
    /// Panics if `hz` is zero, negative or not finite.
    #[inline]
    pub fn set_timestep_hz(&mut self, hz: f64) {
        assert!(hz.is_sign_positive(), "Hz less than or equal to zero");
        assert!(hz.is_finite(), "Hz is infinite");
        self.set_timestep_seconds(1.0 / hz);
    }

    /// Returns the amount of overstep time accumulated toward new steps, as
    /// [`Duration`].
    #[inline]
    pub fn overstep(&self) -> Duration {
        self.context().overstep
    }

    /// Discard a part of the overstep amount.
    ///
    /// If `discard` is higher than overstep, the overstep becomes zero.
    #[inline]
    pub fn discard_overstep(&mut self, discard: Duration) {
        let context = self.context_mut();
        context.overstep = context.overstep.saturating_sub(discard);
    }

    /// Returns the amount of overstep time accumulated toward new steps, as an
    /// [`f32`] fraction of the timestep.
    #[inline]
    pub fn overstep_fraction(&self) -> f32 {
        self.context().overstep.as_secs_f32() / self.context().timestep.as_secs_f32()
    }

    /// Returns the amount of overstep time accumulated toward new steps, as an
    /// [`f64`] fraction of the timestep.
    #[inline]
    pub fn overstep_fraction_f64(&self) -> f64 {
        self.context().overstep.as_secs_f64() / self.context().timestep.as_secs_f64()
    }

    fn accumulate(&mut self, delta: Duration) {
        self.context_mut().overstep += delta;
    }

    fn expend(&mut self) -> bool {
        let timestep = self.timestep();
        if let Some(new_value) = self.context_mut().overstep.checked_sub(timestep) {
            // reduce accumulated and increase elapsed by period
            self.context_mut().overstep = new_value;
            self.advance_by(timestep);
            true
        } else {
            // no more periods left in accumulated
            false
        }
    }
}

impl Default for Fixed {
    fn default() -> Self {
        Self {
            timestep: Time::<Fixed>::DEFAULT_TIMESTEP,
            overstep: Duration::ZERO,
        }
    }
}

/// Runs [`FixedMain`] zero or more times based on delta of
/// [`Time<Virtual>`](Virtual) and [`Time::overstep`]
pub fn run_fixed_main_schedule(world: &mut World) {
    let delta = world.resource::<Time<Virtual>>().delta();
    world.resource_mut::<Time<Fixed>>().accumulate(delta);

    // Run the schedule until we run out of accumulated time
    let _ = world.try_schedule_scope(FixedMain, |world, schedule| {
        while world.resource_mut::<Time<Fixed>>().expend() {
            *world.resource_mut::<Time>() = world.resource::<Time<Fixed>>().as_generic();
            schedule.run(world);
        }
    });

    *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_set_timestep() {
        let mut time = Time::<Fixed>::default();

        assert_eq!(time.timestep(), Time::<Fixed>::DEFAULT_TIMESTEP);

        time.set_timestep(Duration::from_millis(500));
        assert_eq!(time.timestep(), Duration::from_millis(500));

        time.set_timestep_seconds(0.25);
        assert_eq!(time.timestep(), Duration::from_millis(250));

        time.set_timestep_hz(8.0);
        assert_eq!(time.timestep(), Duration::from_millis(125));
    }

    #[test]
    fn test_expend() {
        let mut time = Time::<Fixed>::from_seconds(2.0);

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);

        time.accumulate(Duration::from_secs(1));

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);
        assert_eq!(time.overstep(), Duration::from_secs(1));
        assert_eq!(time.overstep_fraction(), 0.5);
        assert_eq!(time.overstep_fraction_f64(), 0.5);

        assert!(!time.expend()); // false

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);
        assert_eq!(time.overstep(), Duration::from_secs(1));
        assert_eq!(time.overstep_fraction(), 0.5);
        assert_eq!(time.overstep_fraction_f64(), 0.5);

        time.accumulate(Duration::from_secs(1));

        assert_eq!(time.delta(), Duration::ZERO);
        assert_eq!(time.elapsed(), Duration::ZERO);
        assert_eq!(time.overstep(), Duration::from_secs(2));
        assert_eq!(time.overstep_fraction(), 1.0);
        assert_eq!(time.overstep_fraction_f64(), 1.0);

        assert!(time.expend()); // true

        assert_eq!(time.delta(), Duration::from_secs(2));
        assert_eq!(time.elapsed(), Duration::from_secs(2));
        assert_eq!(time.overstep(), Duration::ZERO);
        assert_eq!(time.overstep_fraction(), 0.0);
        assert_eq!(time.overstep_fraction_f64(), 0.0);

        assert!(!time.expend()); // false

        assert_eq!(time.delta(), Duration::from_secs(2));
        assert_eq!(time.elapsed(), Duration::from_secs(2));
        assert_eq!(time.overstep(), Duration::ZERO);
        assert_eq!(time.overstep_fraction(), 0.0);
        assert_eq!(time.overstep_fraction_f64(), 0.0);

        time.accumulate(Duration::from_secs(1));

        assert_eq!(time.delta(), Duration::from_secs(2));
        assert_eq!(time.elapsed(), Duration::from_secs(2));
        assert_eq!(time.overstep(), Duration::from_secs(1));
        assert_eq!(time.overstep_fraction(), 0.5);
        assert_eq!(time.overstep_fraction_f64(), 0.5);

        assert!(!time.expend()); // false

        assert_eq!(time.delta(), Duration::from_secs(2));
        assert_eq!(time.elapsed(), Duration::from_secs(2));
        assert_eq!(time.overstep(), Duration::from_secs(1));
        assert_eq!(time.overstep_fraction(), 0.5);
        assert_eq!(time.overstep_fraction_f64(), 0.5);
    }

    #[test]
    fn test_expend_multiple() {
        let mut time = Time::<Fixed>::from_seconds(2.0);

        time.accumulate(Duration::from_secs(7));
        assert_eq!(time.overstep(), Duration::from_secs(7));

        assert!(time.expend()); // true
        assert_eq!(time.elapsed(), Duration::from_secs(2));
        assert_eq!(time.overstep(), Duration::from_secs(5));

        assert!(time.expend()); // true
        assert_eq!(time.elapsed(), Duration::from_secs(4));
        assert_eq!(time.overstep(), Duration::from_secs(3));

        assert!(time.expend()); // true
        assert_eq!(time.elapsed(), Duration::from_secs(6));
        assert_eq!(time.overstep(), Duration::from_secs(1));

        assert!(!time.expend()); // false
        assert_eq!(time.elapsed(), Duration::from_secs(6));
        assert_eq!(time.overstep(), Duration::from_secs(1));
    }
}
