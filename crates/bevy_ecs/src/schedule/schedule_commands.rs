use bevy_ecs::{
    prelude::*,
    schedule::{InternedScheduleLabel, ScheduleLabel},
    world::CommandQueue,
};
use bevy_platform::collections::HashMap;
use core::mem;
use log::error;

/// A wrapper over [`Commands`] that stores [`CommandQueue`]s to be applied with given delays.
///
/// When dropped, the queue is moved into the [`ScheduleCommandQueues`] resource,
/// and a unique system is added to the specified [`ScheduleLabel`] to run the `CommandQueue`s.
pub struct ScheduleCommands<'w, 's> {
    /// Used to own queues and deduplicate them by their `ScheduleLabel`.
    queues: HashMap<InternedScheduleLabel, CommandQueue>,

    /// The wrapped `Commands` - used to provision out new `Commands`
    /// and move the queues into the `ScheduleCommandQueues` resource when the struct is dropped.
    commands: Commands<'w, 's>,
}

impl<'w, 's> ScheduleCommands<'w, 's> {
    /// Return a [`Commands`] whose commands will be delayed by `ScheduleLabel`.
    #[must_use = "The returned Commands must be used to submit commands with this ScheduleLabel."]
    pub fn label(&mut self, label: impl ScheduleLabel) -> Commands<'w, '_> {
        // Fetch a queue with the given duration or create one
        let queue = self.queues.entry(label.intern()).or_default();
        // Return a new `Commands` to write commands to the queue
        self.commands.rebound_to(queue)
    }

    /// Move the queues into [`ScheduleCommandQueues`] resource.
    fn submit(&mut self) {
        let queues = mem::take(&mut self.queues);

        self.commands.queue(move |world: &mut World| {
            for (label, mut queue) in queues {
                let mut schedules = world.resource_mut::<Schedules>();

                // The use of bypass_change_detection is to determine
                // whether a system needs to be added to the corresponding schedule stage.
                // The resource is only set_changed when such an addition occurs.
                let Some(schedule) = schedules.bypass_change_detection().get_mut(label) else {
                    error!(
                        "ScheduleCommands get error, The schedule:{:?} not exist",
                        label
                    );
                    continue;
                };

                if !schedule.graph().system_sets.contains(ScheduleCommandSet) {
                    // Using the pipe system, pass the corresponding ScheduleLabel.
                    schedule.add_systems(
                        (move || label)
                            .pipe(apply_schedule_command)
                            .in_set(ScheduleCommandSet),
                    );

                    schedules.set_changed();
                }

                let mut resource = world.get_resource_or_init::<ScheduleCommandQueues>();

                resource
                    .0
                    .entry(label)
                    .and_modify(|command| {
                        command.append(&mut queue);
                    })
                    .or_insert(queue);
            }
        });
    }
}

/// Used to ensure that the system processing [`ScheduleCommandQueues`] is unique for each [`Schedule`] stage.
#[derive(Debug, SystemSet, PartialEq, Eq, Hash, Clone)]
pub struct ScheduleCommandSet;

/// Temporarily stores the command queue for each [`Schedule`] stage.
/// The queue is cleared after the corresponding `Schedule` stage has run.
///
/// You should not manually add a `CommandQueue` to this resource,
/// as doing so may result in the `Schedule` lacking a system to execute that queue.
///
/// Instead, use [`ScheduleCommands`], which automatically adds a system to the `Schedule`
/// to execute the corresponding queue, provided that the `Schedule` is included in the run.
#[derive(Debug, Resource, Default)]
pub struct ScheduleCommandQueues(pub HashMap<InternedScheduleLabel, CommandQueue>);

/// Run [`ScheduleCommandQueues`]
fn apply_schedule_command(
    In(label): In<InternedScheduleLabel>,
    mut commands: Commands,
    mut resource: ResMut<ScheduleCommandQueues>,
) {
    if let Some(queue) = resource.0.get_mut(&label) {
        commands.append(queue);
    }
}
/// Extension trait for [`Commands`] that provides delayed command functionality.
pub trait ScheduleCommandsExt<'w> {
    /// Returns a [`ScheduleCommands`] instance that can be used to queue
    /// commands to be submitted at a later point in time.
    ///
    /// When dropped, the `ScheduleCommands` submits move commands that will
    /// move into [`ScheduleCommandQueues`] resource.
    /// Queues are submitted when the corresponding [`ScheduleLabel`] is run.
    ///
    /// # Usage
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::schedule::schedule_commands::ScheduleCommandsExt;
    /// # use bevy_ecs::schedule::ScheduleLabel;
    /// #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
    /// pub struct Update;
    ///
    /// fn my_system(mut commands: Commands) {
    ///     // Spawn an entity after Update Schedule
    ///     commands.scheduled().label(Update).spawn_empty();
    /// }
    /// # bevy_ecs::system::assert_is_system(my_system);
    /// ```
    /// # Timing
    ///
    /// `ScheduleCommands` cannot add queues to the current `ScheduleLabel`.
    /// If you need to add queues to the current `ScheduleLabel` for the next frame,
    /// consider using [`Local`]. `ScheduleCommands` only adds commands for the
    /// corresponding `ScheduleLabel` stage, and only when that `ScheduleLabel` is included in the run list.
    fn scheduled(&mut self) -> ScheduleCommands<'w, '_>;
}

impl<'w, 's> ScheduleCommandsExt<'w> for Commands<'w, 's> {
    fn scheduled(&mut self) -> ScheduleCommands<'w, '_> {
        ScheduleCommands {
            commands: self.reborrow(),
            queues: HashMap::default(),
        }
    }
}

impl<'w, 's> Drop for ScheduleCommands<'w, 's> {
    fn drop(&mut self) {
        self.submit();
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use bevy_ecs_macros::{Component, Resource};

    use crate::schedule::schedule_commands::ScheduleCommandsExt;
    use crate::schedule::set::ScheduleLabel;
    use crate::schedule::ScheduleCommandQueues;
    use crate::{
        prelude::Schedules, schedule::InternedScheduleLabel, system::Commands, world::World,
    };

    #[derive(Resource, Default)]
    pub struct ScheduleOrder(pub Vec<i32>);

    /// The schedule that runs before [`Startup`].
    ///
    /// See the [`Main`] schedule for some details about how schedules are run.
    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
    pub struct PreStartup;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
    pub struct Startup;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
    pub struct PostStartup;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
    pub struct Update;

    fn init_world() -> World {
        let mut world = World::new();
        world.init_resource::<ScheduleOrder>();

        let mut schedules = Schedules::new();
        schedules.entry(PreStartup);
        schedules.entry(Startup);
        schedules.entry(PostStartup);
        schedules.entry(Update);

        world.insert_resource(schedules);

        world
    }

    fn runner<'a>(world: &'a mut World, labels: &'a [InternedScheduleLabel]) -> &'a mut World {
        for label in labels {
            world.schedule_scope(*label, |world, schedule| {
                schedule.run(world);
            });
        }
        world
    }

    fn schedule_labels() -> [InternedScheduleLabel; 4] {
        [
            PreStartup.intern(),
            Startup.intern(),
            PostStartup.intern(),
            Update.intern(),
        ]
    }

    #[test]
    fn delayed_queues_should_run() {
        fn queue_commands(mut commands: Commands) {
            for (label, order) in schedule_labels().into_iter().zip([-1, 0, 1, 2]) {
                commands
                    .scheduled()
                    .label(label)
                    .queue(move |world: &mut World| {
                        world.resource_mut::<ScheduleOrder>().0.push(order);
                    });
            }
        }

        let mut world = init_world();

        world
            .resource_mut::<Schedules>()
            .add_systems(Startup, queue_commands);

        world.flush();

        runner(&mut world, &schedule_labels());

        world.flush();
        let order = &world.resource::<ScheduleOrder>().0;
        assert_eq!(&[1, 2].to_vec(), order);

        // second run
        runner(&mut world, &schedule_labels());

        world.flush();
        let order = &world.resource::<ScheduleOrder>().0;
        assert_eq!(&[1, 2, -1, 1, 2].to_vec(), order);
    }

    #[derive(Component)]
    pub struct TestComponent;

    #[test]
    fn apply_queue_system_unique() {
        let mut world = init_world();

        world
            .resource_mut::<Schedules>()
            .add_systems(Startup, |mut commands: Commands| {
                commands.scheduled().label(Update).spawn(TestComponent);
            });

        let schedules = world.resource_mut::<Schedules>();
        let schedule = schedules.get(Update).unwrap();

        assert_eq!(0, schedule.systems_len());

        runner(&mut world, &schedule_labels());
        world.flush();

        // run again
        runner(&mut world, &schedule_labels());
        world.flush();

        let schedules = world.resource_mut::<Schedules>();
        let schedule = schedules.get(Update).unwrap();

        assert_eq!(1, schedule.systems_len());

        let mut binding = world.query::<&TestComponent>();

        let entitis = binding.iter(&world);

        assert_eq!(2, entitis.count());

        let resource = world.resource::<ScheduleCommandQueues>();

        assert!(resource.0.get(&Update.intern()).unwrap().is_empty());
    }
}
