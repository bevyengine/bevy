use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use bevy_ecs::{
    system::{Commands, Deferred, Res, ResMut, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::{CommandQueue, World},
};

use crate::{Domain, Time, Timer, TimerMode};

/// A [`SystemParam`] that allows you to queue commands to be
/// executed after a certain duration has elapsed,
/// rather than immediately at the next sync point.
///
/// This type also derefs to [`Commands`], so you can still use it
/// to queue commands to run at the next sync point.
///
/// **Note**: By default, commands that have elapsed will be queued
/// in the [`First`] and [`FixedFirst`] schedules for time domains
/// `()` and [`Fixed`], respectively.
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
/// [`First`]: bevy_app::First
/// [`FixedFirst`]: bevy_app::FixedFirst
/// [`Fixed`]: crate::Fixed
#[derive(SystemParam)]
pub struct TimedCommands<'w, 's, T: Domain = ()> {
    commands: Commands<'w, 's>,
    queues: Deferred<'s, TimedCommandQueues<T>>,
}

impl<'w, 's, T: Domain> TimedCommands<'w, 's, T> {
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
    #[must_use = "no commands are queued by this method itself, only by the returned Commands"]
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

impl<'w, 's, T: Domain> Deref for TimedCommands<'w, 's, T> {
    type Target = Commands<'w, 's>;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl<'w, 's, T: Domain> DerefMut for TimedCommands<'w, 's, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

/// A [`SystemBuffer`] and [`Resource`] that holds a list of pairs of [`Timer`]s and [`CommandQueue`]s.
///
/// Used by [`TimedCommands`] to hold queued commands.
#[derive(Default, Resource)]
pub struct TimedCommandQueues<T: Domain> {
    inner: Vec<(Timer, CommandQueue)>,
    _domain: std::marker::PhantomData<T>,
}

impl<T: Domain> SystemBuffer for TimedCommandQueues<T> {
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        let Some(mut queues) = world.get_resource_mut::<Self>() else {
            return;
        };

        queues.inner.append(&mut self.inner);
    }
}

/// A system that ticks commands queued via [`TimedCommands`],
/// which will be applied by default in the [`First`] and [`FixedFirst`] schedules
/// for time domains `()` and [`Fixed`], respectively.
///
/// [`First`]: bevy_app::First
/// [`FixedFirst`]: bevy_app::FixedFirst
/// [`Fixed`]: crate::Fixed
pub fn queue_delayed_commands<T: Domain>(
    mut commands: Commands,
    mut queues: ResMut<TimedCommandQueues<T>>,
    time: Res<Time<T>>,
) {
    queues.inner.retain_mut(|(timer, queue)| {
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

    use bevy_app::{App, Startup, Update};
    use bevy_ecs::{system::Resource, world::World};

    use super::*;

    #[test]
    fn test() {
        #[derive(Resource, PartialEq, Eq, Debug)]
        struct Flag(bool);

        let mut app = App::new();

        app.insert_resource(Time::new(Instant::now()).as_generic());
        app.insert_resource(Flag(false));
        app.add_systems(Update, queue_delayed_commands::<()>);
        app.init_resource::<TimedCommandQueues<()>>();
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
