use alloc::vec::Vec;
use bevy_ecs_macros::Event;
use bevy_platform::sync::{atomic::AtomicU64, Mutex};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::{
    cell::UnsafeCell,
    fmt::{self, Debug, Formatter},
    panic::Location,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::change_detection::{MaybeLocation, CHECK_TICK_THRESHOLD, MAX_CHANGE_AGE};

/// Ticks older than this are compacted by [`Tick::check_tick`].
pub(super) const COMPACT_AGE: u32 = MAX_CHANGE_AGE - 3 * CHECK_TICK_THRESHOLD;

/// The age of the oldest compacted tick. Compacted ticks can't age past [`Tick::MAX`]
/// before the next scan, so systems that have never run stay older than every tick.
const COMPACT_START: u32 = MAX_CHANGE_AGE - 2 * CHECK_TICK_THRESHOLD;

/// A value that tracks when a system ran relative to other systems.
/// This is used to power change detection.
///
/// *Note* that a system that hasn't been run yet has a `Tick` of 0.
#[derive(Copy, Clone, Default, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq, Clone)
)]
pub struct Tick {
    tick: u32,
}

impl Tick {
    /// The maximum relative age for a change tick.
    /// The value of this is equal to [`MAX_CHANGE_AGE`].
    ///
    /// Ticks are periodically scanned to ensure their relative values are below this.
    pub const MAX: Self = Self::new(MAX_CHANGE_AGE);

    /// Creates a new [`Tick`] wrapping the given value.
    #[inline]
    pub const fn new(tick: u32) -> Self {
        Self { tick }
    }

    /// Gets the value of this change tick.
    #[inline]
    pub const fn get(self) -> u32 {
        self.tick
    }

    /// Sets the value of this change tick.
    #[inline]
    pub fn set(&mut self, tick: u32) {
        self.tick = tick;
    }

    /// Returns `true` if this `Tick` occurred since the system's `last_run`.
    ///
    /// `this_run` is the current tick of the system, used as a reference to help deal with wraparound.
    #[inline]
    pub fn is_newer_than(self, last_run: Tick, this_run: Tick) -> bool {
        // This works even with wraparound because the world tick (`this_run`) is always "newer" than
        // `last_run` and `self.tick`, and we scan periodically to compact `ComponentTicks` values
        // so they never get older than `u32::MAX` (the difference would overflow).
        //
        // The clamp here ensures determinism (since scans could differ between app runs).
        let ticks_since_insert = this_run.relative_to(self).tick.min(MAX_CHANGE_AGE);
        let ticks_since_system = this_run.relative_to(last_run).tick.min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_insert
    }

    /// Returns a change tick representing the relationship between `self` and `other`.
    #[inline]
    pub(crate) fn relative_to(self, other: Self) -> Self {
        let tick = self.tick.wrapping_sub(other.tick);
        Self { tick }
    }

    /// Compacts this tick if it is old, keeping its order relative to every tick
    /// recorded by the [`CheckChangeTicks`] pass. Call it at most once per tick in each pass,
    /// as compacting a tick twice breaks its order.
    ///
    /// Returns `true` if the tick was compacted. Otherwise, returns `false`.
    #[inline]
    pub fn check_tick(&mut self, check: &CheckChangeTicks) -> bool {
        let age = check.present.relative_to(*self).get();
        if age <= check.cutoff {
            return false;
        }
        if let CheckPhase::Compact { last, .. } = &check.phase {
            // Neighboring ticks are often equal, so reuse the last result.
            let cached = last.load(Ordering::Relaxed);
            if (cached >> 32) as u32 == self.tick {
                self.tick = cached as u32;
                return true;
            }
        }
        self.check_old_tick(check, age)
    }

    #[cold]
    #[inline(never)]
    fn check_old_tick(&mut self, check: &CheckChangeTicks, age: u32) -> bool {
        match &check.phase {
            CheckPhase::Collect(ticks) => {
                ticks.lock().unwrap().push(*self);
                false
            }
            CheckPhase::Compact { ages, last } => {
                // Each recorded age gets its own slot and each gap between them shares one,
                // so comparisons against recorded ticks are unchanged.
                let older = ages.partition_point(|&a| a > age);
                let slot = 2 * older as u32 + u32::from(ages.get(older) == Some(&age));
                let compacted = check.present.relative_to(Tick::new(COMPACT_START - slot));
                last.store(
                    (u64::from(self.tick) << 32) | u64::from(compacted.tick),
                    Ordering::Relaxed,
                );
                *self = compacted;
                true
            }
        }
    }
}

/// A tick that can be updated from multiple threads.
///
/// This has exactly the same semantics as [`Tick`] but can be updated
/// atomically. It's used for summary ticks.
#[derive(Default)]
pub struct AtomicTick {
    /// The atomic tick value.
    tick: AtomicU32,
}

impl AtomicTick {
    /// Returns the current value of the tick.
    pub fn get(&self) -> Tick {
        Tick {
            tick: self.tick.load(Ordering::Relaxed),
        }
    }

    #[expect(
        clippy::doc_markdown,
        reason = "The word 'ARMv6' does not require backticks"
    )]
    /// Sets a new value for the tick.
    ///
    /// Note that this method takes `&self` and can therefore be called from
    /// multiple threads. The tick is updated using relaxed ordering and is
    /// therefore cheap to update on common architectures (x86-64, ARMv6 and
    /// newer). However, be warned that, because it uses relaxed ordering, a
    /// full mutex lock or similar barrier is required if you need to coordinate
    /// the synchronization of this value with other memory locations.
    pub fn set(&self, new_tick: Tick) {
        // Do an unsynchronized read first.
        //
        // This is important on x86-64 (both Intel and AMD) to avoid a
        // performance cliff. It has much smaller effects on AArch64, but it's
        // still worth doing on e.g. Apple M2 in parallel mode, so I'm leaving
        // it in.
        //
        // See benchmarks:
        // https://github.com/bevyengine/bevy/pull/25157#issuecomment-5242287382
        if self.tick.load(Ordering::Relaxed) != new_tick.get() {
            self.tick.store(new_tick.get(), Ordering::Relaxed);
        }
    }
}

impl Debug for AtomicTick {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AtomicTick")
            .field("tick", &self.get().get())
            .finish()
    }
}

/// An [`Event`] that can be used to maintain [`Tick`]s in custom data structures, enabling to make
/// use of bevy's periodic checks that compact old ticks, preventing overflows while keeping
/// methods like [`Tick::is_newer_than`] reliable.
///
/// It is triggered twice per check: first to record old ticks, then to compact them.
/// Observers should pass it to [`Tick::check_tick`] both times.
///
/// # Example
///
/// Here a schedule is stored in a custom resource. This way the systems in it would not have their change
/// ticks automatically updated via [`World::check_change_ticks`](crate::world::World::check_change_ticks),
/// possibly causing `Tick`-related bugs on long-running apps.
///
/// To fix that, add an observer for this event that calls the schedule's
/// [`Schedule::check_change_ticks`](bevy_ecs::schedule::Schedule::check_change_ticks).
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::change_detection::CheckChangeTicks;
///
/// #[derive(Resource)]
/// struct CustomSchedule(Schedule);
///
/// # let mut world = World::new();
/// world.add_observer(|check: On<CheckChangeTicks>, mut schedule: ResMut<CustomSchedule>| {
///     schedule.0.check_change_ticks(&check);
/// });
/// ```
#[derive(Debug, Event)]
pub struct CheckChangeTicks {
    present: Tick,
    /// Ticks older than this are checked.
    cutoff: u32,
    phase: CheckPhase,
}

#[derive(Debug)]
enum CheckPhase {
    /// Recording old ticks that are compared against, like a system's `last_run`.
    Collect(Mutex<Vec<Tick>>),
    /// Compacting old ticks around the recorded ages, sorted oldest first.
    Compact {
        ages: Vec<u32>,
        /// The last compacted tick, packed as `(old << 32) | new`.
        last: AtomicU64,
    },
}

impl CheckChangeTicks {
    pub(crate) fn collect(present: Tick) -> Self {
        Self {
            present,
            cutoff: COMPACT_AGE,
            phase: CheckPhase::Collect(Mutex::new(Vec::new())),
        }
    }

    /// `present` may be newer than the collecting pass, as observers advance the world tick.
    pub(crate) fn compact(self, present: Tick) -> Self {
        let mut ages = Vec::new();
        if let CheckPhase::Collect(ticks) = self.phase {
            let ticks = ticks.into_inner().unwrap();
            ages.extend(
                ticks
                    .into_iter()
                    .map(|tick| present.relative_to(tick).get()),
            );
        }
        ages.sort_unstable_by(|a, b| b.cmp(a));
        ages.dedup();
        // Compacted ticks must stay older than every tick that isn't compacted.
        debug_assert!(ages.len() < CHECK_TICK_THRESHOLD as usize / 2);
        Self {
            // Only compact ticks the collecting pass saw as old, so none are missed.
            cutoff: COMPACT_AGE + present.relative_to(self.present).get(),
            present,
            phase: CheckPhase::Compact {
                ages,
                // `present` is never compacted, so this never matches.
                last: AtomicU64::new(u64::from(present.tick) << 32),
            },
        }
    }

    /// Get the present `Tick` that other ticks get compared to.
    pub fn present_tick(&self) -> Tick {
        self.present
    }
}

/// Interior-mutable access to the [`Tick`]s of a single component or resource.
#[derive(Copy, Clone, Debug)]
pub struct ComponentTickCells<'a> {
    /// The tick indicating when the value was added to the world.
    pub added: &'a UnsafeCell<Tick>,
    /// The tick indicating the last time the value was modified.
    pub changed: &'a UnsafeCell<Tick>,
    /// The calling location that last modified the value.
    pub changed_by: MaybeLocation<&'a UnsafeCell<&'static Location<'static>>>,
    /// The summary tick for the column, if the component is dense and has a
    /// summary tick.
    pub summary_tick: Option<&'a AtomicTick>,
}

/// Records when a component or resource was added and when it was last mutably dereferenced (or added).
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
pub struct ComponentTicks {
    /// Tick recording the time this component or resource was added.
    pub added: Tick,

    /// Tick recording the time this component or resource was most recently changed.
    pub changed: Tick,
}

impl ComponentTicks {
    /// Returns `true` if the component or resource was added after the system last ran
    /// (or the system is running for the first time).
    #[inline]
    pub fn is_added(&self, last_run: Tick, this_run: Tick) -> bool {
        self.added.is_newer_than(last_run, this_run)
    }

    /// Returns `true` if the component or resource was added or mutably dereferenced after the system last ran
    /// (or the system is running for the first time).
    #[inline]
    pub fn is_changed(&self, last_run: Tick, this_run: Tick) -> bool {
        self.changed.is_newer_than(last_run, this_run)
    }

    /// Creates a new instance with the same change tick for `added` and `changed`.
    pub fn new(change_tick: Tick) -> Self {
        Self {
            added: change_tick,
            changed: change_tick,
        }
    }

    /// Manually sets the change tick.
    ///
    /// This is normally done automatically via the [`DerefMut`](core::ops::DerefMut) implementation
    /// on [`Mut<T>`](crate::change_detection::Mut), [`ResMut<T>`](crate::change_detection::ResMut), etc.
    /// However, components and resources that make use of interior mutability might require manual updates.
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_ecs::{world::World, change_detection::ComponentTicks};
    /// let world: World = unimplemented!();
    /// let component_ticks: ComponentTicks = unimplemented!();
    ///
    /// component_ticks.set_changed(world.read_change_tick());
    /// ```
    #[inline]
    pub fn set_changed(&mut self, change_tick: Tick) {
        self.changed = change_tick;
    }
}
