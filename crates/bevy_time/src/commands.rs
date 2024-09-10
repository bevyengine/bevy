use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    time::Duration,
};

use bevy_ecs::{
    system::{Commands, Deferred, Res, ResMut, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::{CommandQueue, World},
};
use bevy_utils::TypeIdMap;

use crate::{Domain, Fixed, Generic, Time, Timer, TimerMode, Virtual};

/// A [`SystemParam`] that allows you to queue commands to be
/// executed after a certain duration has elapsed,
/// rather than immediately at the next sync point.
///
/// This type also derefs to [`Commands`], so you can still use it
/// to queue commands to run at the next sync point.
///
/// # Deferred
///
/// - Commands queued during the [`Update`] schedules
///   will be applied (if their timers have elapsed) during the [`First`] schedule.
/// - Commands queued during the [`FixedUpdate`] schedules
///   will be applied (if their timers have elapsed) during the [`FixedFirst`] schedule.
///
/// # Usage
///
/// Add `mut commands: TimedCommands` as a function argument to your system,
/// and call [`TimedCommands::after`] to get a [`Commands`] that will queue its commands
/// to run after the specified duration has elapsed.
///
/// ```
/// # use bevy_time::prelude::*;
/// # use std::time::Duration;
/// #
/// fn my_system(mut commands: TimedCommands) {
///     commands.after(Duration::from_secs(5))
///         .spawn_empty();
///
///     commands.spawn_empty();
/// }
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// [`Update`]: bevy_app::Update
/// [`First`]: bevy_app::First
/// [`FixedUpdate`]: bevy_app::FixedUpdate
/// [`FixedFirst`]: bevy_app::FixedFirst
#[derive(SystemParam)]
pub struct TimedCommands<'w, 's, T: Domain = Generic>
where
    TimedCommandQueues<T>: SystemBuffer,
{
    commands: Commands<'w, 's>,
    queues: Deferred<'s, TimedCommandQueues<T>>,
}

impl<'w, 's, T: Domain> TimedCommands<'w, 's, T>
where
    TimedCommandQueues<T>: SystemBuffer,
{
    /// Creates a new [`Commands`] instance that will have its commands queued
    /// after the specified duration has elapsed.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_time::prelude::*;
    /// # use std::time::Duration;
    /// #
    /// fn my_system(mut commands: TimedCommands) {
    ///     commands.after(Duration::from_secs(5))
    ///         .spawn_empty();
    /// }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    #[must_use = "no commands are queued by this method itself, only through the returned Commands"]
    pub fn after(&mut self, duration: Duration) -> Commands<'w, '_> {
        let timer = Timer::new(duration, TimerMode::Once);
        let queue = CommandQueue::default();
        self.queues.inner.push((timer, queue));
        let (_, queue) = self
            .queues
            .inner
            .last_mut()
            .unwrap_or_else(|| unreachable!("we just pushed a queue"));
        self.commands.with_queue(queue)
    }
}

impl<'w, 's, T: Domain> Deref for TimedCommands<'w, 's, T>
where
    TimedCommandQueues<T>: SystemBuffer,
{
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl<'w, 's, T: Domain> DerefMut for TimedCommands<'w, 's, T>
where
    TimedCommandQueues<T>: SystemBuffer,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

/// A [`SystemBuffer`] and [`Resource`] that holds a list of pairs of [`Timer`]s and [`CommandQueue`]s.
///
/// Used by [`TimedCommands`] to hold queued commands.
pub struct TimedCommandQueues<T: Domain> {
    inner: Vec<(Timer, CommandQueue)>,
    _domain: std::marker::PhantomData<T>,
}

/// The [`Generic`] [`Time`] [`Domain`] is a special case that contextually changes
/// based on what schedule is currently running:
/// - When operating in the normal [`Update`] schedule, the actual domain is [`Virtual`].
/// - When operating in the [`FixedUpdate`] schedule, the actual domain is [`Fixed`].
///
/// We use the "provenance" of the [`Generic`] [`Time`] to determine which domain to use,
/// and append to the global queues accordingly.
/// If the provenance is not set, we append to the [`Virtual`] domain.
impl SystemBuffer for TimedCommandQueues<Generic> {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let provenance = world
            .get_resource::<Time>()
            .and_then(|t| t.context().provenance());

        let Some(mut global) = world.get_resource_mut::<GlobalTimedCommandQueues>() else {
            return;
        };

        if let Some(provenance) = provenance {
            global.append_to_dyn(provenance, self);
        } else {
            global.append_to::<_, Virtual>(self);
        }
    }
}

impl SystemBuffer for TimedCommandQueues<Virtual> {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let Some(mut global) = world.get_resource_mut::<GlobalTimedCommandQueues>() else {
            return;
        };

        global.append(self);
    }
}

impl SystemBuffer for TimedCommandQueues<Fixed> {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let Some(mut global) = world.get_resource_mut::<GlobalTimedCommandQueues>() else {
            return;
        };

        global.append(self);
    }
}

impl<T: Domain> Default for TimedCommandQueues<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            _domain: Default::default(),
        }
    }
}

/// [`Resource`] that holds separate queues of [`TimedCommandQueues`] for each [`Domain`].
#[derive(Resource, Default)]
pub struct GlobalTimedCommandQueues {
    domains_to_queues: TypeIdMap<Vec<(Timer, CommandQueue)>>,
}

impl GlobalTimedCommandQueues {
    /// Appends the queues from the given [`TimedCommandQueues`].
    pub fn append<Current: Domain>(&mut self, queues: &mut TimedCommandQueues<Current>) {
        self.append_to::<Current, Current>(queues);
    }

    /// Appends the queues from the given [`TimedCommandQueues`] to the target `Target` [`Domain`]
    /// in the [`GlobalTimedCommandQueues`].
    ///
    /// Queuing to a different [`Domain`] than the current one is required to support
    /// the special-case behavior of the [`Generic`] [`Time`] domain.
    pub fn append_to<Current: Domain, Target: Domain>(
        &mut self,
        queues: &mut TimedCommandQueues<Current>,
    ) {
        self.append_to_dyn(TypeId::of::<Target>(), queues);
    }

    /// Appends the queues from the given [`TimedCommandQueues`] to the target [`Domain`] type
    /// in the [`GlobalTimedCommandQueues`].
    ///
    /// Queuing to a different [`Domain`] than the current one is required to support
    /// the special-case behavior of the [`Generic`] [`Time`] domain.
    pub fn append_to_dyn<Current: Domain>(
        &mut self,
        target_domain: TypeId,
        queues: &mut TimedCommandQueues<Current>,
    ) {
        let domain = self.domains_to_queues.entry(target_domain).or_default();
        domain.append(&mut queues.inner);
    }
}

/// A system that ticks commands queued via [`TimedCommands`],
/// which will be applied by default in the [`First`] and [`FixedFirst`] schedules
/// for time domains [`Virtual`] and [`Fixed`], respectively.
///
/// [`First`]: bevy_app::First
/// [`FixedFirst`]: bevy_app::FixedFirst
/// [`Fixed`]: crate::Fixed
pub fn queue_timed_commands<T: Domain>(
    mut commands: Commands,
    mut global: ResMut<GlobalTimedCommandQueues>,
    time: Res<Time<T>>,
) {
    let Some(queues) = global.domains_to_queues.get_mut(&TypeId::of::<T>()) else {
        return;
    };

    queues.retain_mut(|(timer, queue)| {
        let finished = timer.tick(time.delta()).just_finished();
        if finished {
            commands.append(queue);
        }
        !finished
    });
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use bevy_app::{App, MainSchedulePlugin, Startup, Update};
    use bevy_ecs::{system::Resource, world::World};

    use crate::TimePlugin;

    use super::*;

    #[test]
    fn test() {
        #[derive(Resource, PartialEq, Eq, Debug)]
        struct Flag(bool);

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.insert_resource(Flag(false));
        app.add_systems(Startup, |mut commands: TimedCommands| {
            commands
                .after(Duration::from_secs(1000))
                .add(|world: &mut World| {
                    *world.resource_mut::<Flag>() = Flag(true);
                });
        });

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(false)));

        app.update();

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(false)));

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(999));

        app.update();

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(false)));

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));

        app.update();

        assert_eq!(app.world().get_resource::<Flag>(), Some(&Flag(true)));
    }
}
