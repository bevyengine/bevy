use alloc::vec::Vec;
use bevy_ecs::{
    component::Component, reflect::ReflectComponent, system::Commands, world::CommandQueue,
};
use bevy_platform::collections::HashMap;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use core::time::Duration;

use crate::{Timer, TimerMode};

/// A wrapper over [`Commands`] that stores [`CommandQueue`]s to be applied with given delays.
///
/// When dropped, the queues are spawned into the world as new entities with
/// [`DelayedCommandQueue`] components, and then ticked by the [`delayed_queues_system`].
///
/// [`delayed_queues_system`]: crate::delayed_queues_system
pub struct DelayedCommands<'w, 's> {
    commands: Commands<'w, 's>,
    queues: HashMap<Duration, CommandQueue>,
}

impl<'w, 's, 'q> DelayedCommands<'w, 's> {
    /// Return a [`Commands`] whose commands will be delayed by `secs` seconds.
    pub fn secs(&mut self, secs: f32) -> Commands<'w, '_> {
        let queue = self
            .queues
            .entry(Duration::from_secs_f32(secs))
            .or_default();
        self.commands.rebind(queue)
    }

    /// Return a [`Commands`] whose commands will be delayed by `duration`.
    pub fn duration(&mut self, duration: Duration) -> Commands<'w, '_> {
        let queue = self.queues.entry(duration).or_default();
        self.commands.rebind(queue)
    }
}

/// Extension trait for `Commands` that provides delayed command functionality.
pub trait DelayedCommandsExt {
    /// Returns a [`DelayedCommands`] instance that can be used to queue
    /// commands to be submitted at a later point in time.
    ///
    /// When dropped, the [`DelayedCommands`] submits spawn commands
    /// that will spawn [`DelayedCommandQueue`] entities. The entities are ticked
    /// and their queues submitted automatically by the [`delayed_queues_system`]
    /// after the specified delays.
    ///
    /// [`delayed_queues_system`]: crate::delayed_queues_system
    fn delayed(&mut self) -> DelayedCommands<'_, '_>;
}

impl<'w> DelayedCommandsExt for Commands<'w, '_> {
    fn delayed(&mut self) -> DelayedCommands<'w, '_> {
        DelayedCommands {
            commands: self.reborrow(),
            queues: HashMap::default(),
        }
    }
}

impl<'w, 's> Drop for DelayedCommands<'w, 's> {
    fn drop(&mut self) {
        let queues = self
            .queues
            .drain()
            .map(|(dur, queue)| DelayedCommandQueue {
                timer: Timer::new(dur, TimerMode::Once),
                queue,
            })
            .collect::<Vec<_>>();
        self.commands.spawn_batch(queues);
    }
}

/// A component with a [`Timer`] and a [`CommandQueue`] to be submitted later.
///
/// Timers in these components are ticked automatically by the [`delayed_queues_system`]
/// added by [`TimePlugin`].
///
/// [`delayed_queues_system`]: crate::delayed_queues_system
/// [`TimePlugin`]: crate::TimePlugin
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DelayedCommandQueue {
    /// The timer that determines when the queue is submitted.
    pub timer: Timer,

    /// The queue to be submitted when the timer finishes.
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub queue: CommandQueue,
}
