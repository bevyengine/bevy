use std::time::Duration;

use bevy_ecs::{
    entity::Entities,
    system::{Commands, Deferred, Res, ResMut, Resource, SystemBuffer, SystemMeta, SystemParam},
    world::{CommandQueue, World},
};

use crate::{Domain, Time, Timer, TimerMode};

/// A [`SystemParam`] that allows you to queue commands to be
/// executed after a certain duration has elapsed,
/// rather than immediately at the next sync point.
#[derive(SystemParam)]
pub struct TimedCommands<'w, 's, T: Domain = ()> {
    entities: &'w Entities,
    queues: Deferred<'s, TimedCommandQueues<T>>,
}

impl<'w, 's, T: Domain> TimedCommands<'w, 's, T> {
    /// Creates a new [`Commands`] instance that will have its commands queued
    /// after the specified duration has elapsed.
    #[must_use]
    pub fn after(&mut self, duration: Duration) -> Commands<'w, '_> {
        let timer = Timer::new(duration, TimerMode::Once);
        let queue = CommandQueue::default();
        self.queues.inner.push((timer, queue));
        let (_, queue) = self
            .queues
            .inner
            .last_mut()
            .unwrap_or_else(|| unreachable!("we just pushed a queue"));
        Commands::new_from_entities(queue, self.entities)
    }
}

/// A [`SystemBuffer`] and [`Resource`] that holds a list of pairs of [`Timer`]s and [`CommandQueue`]s.
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

/// A system that ticks delayed commands, queuing them for the next sync point if their timers have finished.
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
