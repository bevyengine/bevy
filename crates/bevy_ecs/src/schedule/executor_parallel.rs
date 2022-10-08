use crate::{
    archetype::ArchetypeComponentId,
    query::Access,
    schedule::{ParallelSystemExecutor, SystemContainer},
    world::World,
};
use async_channel::{Receiver, Sender};
use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
#[cfg(feature = "trace")]
use bevy_utils::tracing::Instrument;
use event_listener::Event;
use fixedbitset::FixedBitSet;

#[cfg(test)]
use scheduling_event::*;

struct SystemSchedulingMetadata {
    /// Used to signal the system's task to start the system.
    start: Event,
    /// Indices of systems that depend on this one, used to decrement their
    /// dependency counters when this system finishes.
    dependants: Vec<usize>,
    /// Total amount of dependencies this system has.
    dependencies_total: usize,
    /// Amount of unsatisfied dependencies, when it reaches 0 the system is queued to be started.
    dependencies_now: usize,
    /// Archetype-component access information.
    archetype_component_access: Access<ArchetypeComponentId>,
    /// Whether or not this system is send-able
    is_send: bool,
}

pub struct ParallelExecutor {
    /// Cached metadata of every system.
    system_metadata: Vec<SystemSchedulingMetadata>,
    /// Used by systems to notify the executor that they have finished.
    finish_sender: Sender<usize>,
    /// Receives finish events from systems.
    finish_receiver: Receiver<usize>,
    /// Systems that should be started at next opportunity.
    queued: FixedBitSet,
    /// Systems that are currently running.
    running: FixedBitSet,
    /// Whether a non-send system is currently running.
    non_send_running: bool,
    /// Systems that should run this iteration.
    should_run: FixedBitSet,
    /// Compound archetype-component access information of currently running systems.
    active_archetype_component_access: Access<ArchetypeComponentId>,
    /// Scratch space to avoid reallocating a vector when updating dependency counters.
    dependants_scratch: Vec<usize>,
    #[cfg(test)]
    events_sender: Option<Sender<SchedulingEvent>>,
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        // Using a bounded channel here as it avoids allocations when signaling
        // and generally remains hotter in memory. It'll take 128 systems completing
        // before the parallel executor runs before this overflows. If it overflows
        // all systems will just suspend until the parallel executor runs.
        let (finish_sender, finish_receiver) = async_channel::bounded(128);
        Self {
            system_metadata: Default::default(),
            finish_sender,
            finish_receiver,
            queued: Default::default(),
            running: Default::default(),
            non_send_running: false,
            should_run: Default::default(),
            active_archetype_component_access: Default::default(),
            dependants_scratch: Default::default(),
            #[cfg(test)]
            events_sender: None,
        }
    }
}

impl ParallelSystemExecutor for ParallelExecutor {
    fn rebuild_cached_data(&mut self, systems: &[SystemContainer]) {
        self.system_metadata.clear();
        self.queued.grow(systems.len());
        self.running.grow(systems.len());
        self.should_run.grow(systems.len());

        // Construct scheduling data for systems.
        for container in systems {
            let dependencies_total = container.dependencies().len();
            let system = container.system();
            self.system_metadata.push(SystemSchedulingMetadata {
                start: Event::new(),
                dependants: vec![],
                dependencies_total,
                dependencies_now: 0,
                is_send: system.is_send(),
                archetype_component_access: Default::default(),
            });
        }
        // Populate the dependants lists in the scheduling metadata.
        for (dependant, container) in systems.iter().enumerate() {
            for dependency in container.dependencies() {
                self.system_metadata[*dependency].dependants.push(dependant);
            }
        }
    }

    fn run_systems(&mut self, systems: &mut [SystemContainer], world: &mut World) {
        #[cfg(test)]
        if self.events_sender.is_none() {
            let (sender, receiver) = async_channel::unbounded::<SchedulingEvent>();
            world.insert_resource(SchedulingEvents(receiver));
            self.events_sender = Some(sender);
        }

        {
            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("update_archetypes").entered();
            for (index, container) in systems.iter_mut().enumerate() {
                let meta = &mut self.system_metadata[index];
                let system = container.system_mut();
                system.update_archetype_component_access(world);
                meta.archetype_component_access
                    .extend(system.archetype_component_access());
            }
        }

        ComputeTaskPool::init(TaskPool::default).scope(|scope| {
            self.prepare_systems(scope, systems, world);
            if self.should_run.count_ones(..) == 0 {
                return;
            }
            let parallel_executor = async {
                // All systems have been ran if there are no queued or running systems.
                while 0 != self.queued.count_ones(..) + self.running.count_ones(..) {
                    self.process_queued_systems();
                    // Avoid deadlocking if no systems were actually started.
                    if self.running.count_ones(..) != 0 {
                        // Wait until at least one system has finished.
                        let index = self
                            .finish_receiver
                            .recv()
                            .await
                            .unwrap_or_else(|error| unreachable!("{}", error));
                        self.process_finished_system(index);
                        // Gather other systems than may have finished.
                        while let Ok(index) = self.finish_receiver.try_recv() {
                            self.process_finished_system(index);
                        }
                        // At least one system has finished, so active access is outdated.
                        self.rebuild_active_access();
                    }
                    self.update_counters_and_queue_systems();
                }
            };
            #[cfg(feature = "trace")]
            let span = bevy_utils::tracing::info_span!("parallel executor");
            #[cfg(feature = "trace")]
            let parallel_executor = parallel_executor.instrument(span);
            scope.spawn(parallel_executor);
        });
    }
}

impl ParallelExecutor {
    /// Populates `should_run` bitset, spawns tasks for systems that should run this iteration,
    /// queues systems with no dependencies to run (or skip) at next opportunity.
    fn prepare_systems<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        systems: &'scope mut [SystemContainer],
        world: &'scope World,
    ) {
        // These are used as a part of a unit test.
        #[cfg(test)]
        let mut started_systems = 0;
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("prepare_systems").entered();
        self.should_run.clear();
        for (index, (system_data, system)) in
            self.system_metadata.iter_mut().zip(systems).enumerate()
        {
            let should_run = system.should_run();
            let can_start = should_run
                && system_data.dependencies_total == 0
                && Self::can_start_now(
                    self.non_send_running,
                    system_data,
                    &self.active_archetype_component_access,
                );

            // Queue the system if it has no dependencies, otherwise reset its dependency counter.
            if system_data.dependencies_total == 0 {
                if !can_start {
                    self.queued.insert(index);
                }
            } else {
                system_data.dependencies_now = system_data.dependencies_total;
            }

            if !should_run {
                continue;
            }

            // Spawn the system task.
            self.should_run.insert(index);
            let finish_sender = self.finish_sender.clone();
            let system = system.system_mut();
            #[cfg(feature = "trace")] // NB: outside the task to get the TLS current span
            let system_span = bevy_utils::tracing::info_span!("system", name = &*system.name());
            #[cfg(feature = "trace")]
            let overhead_span =
                bevy_utils::tracing::info_span!("system overhead", name = &*system.name());

            let mut run = move || {
                #[cfg(feature = "trace")]
                let _system_guard = system_span.enter();
                // SAFETY: the executor prevents two systems with conflicting access from running simultaneously.
                unsafe { system.run_unsafe((), world) };
            };

            if can_start {
                let task = async move {
                    run();
                    // This will never panic:
                    //  - The channel is never closed or dropped.
                    //  - Overflowing the bounded size will just suspend until
                    //    there is capacity.
                    finish_sender
                        .send(index)
                        .await
                        .unwrap_or_else(|error| unreachable!("{}", error));
                };

                #[cfg(feature = "trace")]
                let task = task.instrument(overhead_span);
                if system_data.is_send {
                    scope.spawn(task);
                } else {
                    scope.spawn_on_scope(task);
                }

                #[cfg(test)]
                {
                    started_systems += 1;
                }

                self.running.insert(index);
                if !system_data.is_send {
                    self.non_send_running = true;
                }
                // Add this system's access information to the active access information.
                self.active_archetype_component_access
                    .extend(&system_data.archetype_component_access);
            } else {
                let start_listener = system_data.start.listen();
                let task = async move {
                    start_listener.await;
                    run();
                    // This will never panic:
                    //  - The channel is never closed or dropped.
                    //  - Overflowing the bounded size will just suspend until
                    //    there is capacity.
                    finish_sender
                        .send(index)
                        .await
                        .unwrap_or_else(|error| unreachable!("{}", error));
                };

                #[cfg(feature = "trace")]
                let task = task.instrument(overhead_span);
                if system_data.is_send {
                    scope.spawn(task);
                } else {
                    scope.spawn_on_scope(task);
                }
            }
        }
        #[cfg(test)]
        if started_systems != 0 {
            self.emit_event(SchedulingEvent::StartedSystems(started_systems));
        }
    }

    /// Determines if the system with given index has no conflicts with already running systems.
    #[inline]
    fn can_start_now(
        non_send_running: bool,
        system_data: &SystemSchedulingMetadata,
        active_archetype_component_access: &Access<ArchetypeComponentId>,
    ) -> bool {
        // Non-send systems are considered conflicting with each other.
        (!non_send_running || system_data.is_send)
            && system_data
                .archetype_component_access
                .is_compatible(active_archetype_component_access)
    }

    /// Starts all non-conflicting queued systems, moves them from `queued` to `running`,
    /// adds their access information to active access information;
    /// processes queued systems that shouldn't run this iteration as completed immediately.
    fn process_queued_systems(&mut self) {
        // These are used as a part of a unit test as seen in `process_queued_systems`.
        // Removing them will cause the test to fail.
        #[cfg(test)]
        let mut started_systems = 0;
        for index in self.queued.ones() {
            // If the system shouldn't actually run this iteration, process it as completed
            // immediately; otherwise, check for conflicts and signal its task to start.
            let system_metadata = &self.system_metadata[index];
            if !self.should_run[index] {
                self.dependants_scratch.extend(&system_metadata.dependants);
            } else if Self::can_start_now(
                self.non_send_running,
                system_metadata,
                &self.active_archetype_component_access,
            ) {
                #[cfg(test)]
                {
                    started_systems += 1;
                }
                system_metadata.start.notify_additional_relaxed(1);
                self.running.insert(index);
                if !system_metadata.is_send {
                    self.non_send_running = true;
                }
                // Add this system's access information to the active access information.
                self.active_archetype_component_access
                    .extend(&system_metadata.archetype_component_access);
            }
        }
        #[cfg(test)]
        if started_systems != 0 {
            self.emit_event(SchedulingEvent::StartedSystems(started_systems));
        }
        // Remove now running systems from the queue.
        self.queued.difference_with(&self.running);
        // Remove immediately processed systems from the queue.
        self.queued.intersect_with(&self.should_run);
    }

    /// Unmarks the system give index as running, caches indices of its dependants
    /// in the `dependants_scratch`.
    fn process_finished_system(&mut self, index: usize) {
        let system_data = &self.system_metadata[index];
        if !system_data.is_send {
            self.non_send_running = false;
        }
        self.running.set(index, false);
        self.dependants_scratch.extend(&system_data.dependants);
    }

    /// Discards active access information and builds it again using currently
    /// running systems' access information.
    fn rebuild_active_access(&mut self) {
        self.active_archetype_component_access.clear();
        for index in self.running.ones() {
            self.active_archetype_component_access
                .extend(&self.system_metadata[index].archetype_component_access);
        }
    }

    /// Drains `dependants_scratch`, decrementing dependency counters and enqueueing any
    /// systems that become able to run.
    fn update_counters_and_queue_systems(&mut self) {
        for index in self.dependants_scratch.drain(..) {
            let dependant_data = &mut self.system_metadata[index];
            dependant_data.dependencies_now -= 1;
            if dependant_data.dependencies_now == 0 {
                self.queued.insert(index);
            }
        }
    }

    #[cfg(test)]
    fn emit_event(&self, event: SchedulingEvent) {
        let _ = self.events_sender.as_ref().unwrap().try_send(event);
    }
}

#[cfg(test)]
mod scheduling_event {
    use crate as bevy_ecs;
    use crate::system::Resource;
    use async_channel::Receiver;

    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum SchedulingEvent {
        StartedSystems(usize),
    }

    #[derive(Resource)]
    pub(super) struct SchedulingEvents(pub(crate) Receiver<SchedulingEvent>);
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        component::Component,
        schedule::{
            executor_parallel::scheduling_event::*, SingleThreadedExecutor, Stage, SystemStage,
        },
        system::{NonSend, Query, Res, ResMut, Resource},
        world::World,
    };

    use SchedulingEvent::StartedSystems;

    #[derive(Component)]
    struct W<T>(T);
    #[derive(Resource, Default)]
    struct Counter(usize);

    fn receive_events(world: &World) -> Vec<SchedulingEvent> {
        let mut events = Vec::new();
        while let Ok(event) = world.resource::<SchedulingEvents>().0.try_recv() {
            events.push(event);
        }
        events
    }

    #[test]
    fn trivial() {
        let mut world = World::new();
        fn wants_for_nothing() {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_for_nothing)
            .with_system(wants_for_nothing)
            .with_system(wants_for_nothing);
        stage.run(&mut world);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(3), StartedSystems(3),]
        );
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        world.init_resource::<Counter>();
        fn wants_mut(_: ResMut<Counter>) {}
        fn wants_ref(_: Res<Counter>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut)
            .with_system(wants_mut);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut)
            .with_system(wants_ref);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_ref)
            .with_system(wants_ref);
        stage.run(&mut world);
        assert_eq!(receive_events(&world), vec![StartedSystems(2),]);
    }

    #[test]
    fn queries() {
        let mut world = World::new();
        world.spawn(W(0usize));
        fn wants_mut(_: Query<&mut W<usize>>) {}
        fn wants_ref(_: Query<&W<usize>>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut)
            .with_system(wants_mut);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut)
            .with_system(wants_ref);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_ref)
            .with_system(wants_ref);
        stage.run(&mut world);
        assert_eq!(receive_events(&world), vec![StartedSystems(2),]);
        let mut world = World::new();
        world.spawn((W(0usize), W(0u32), W(0f32)));
        fn wants_mut_usize(_: Query<(&mut W<usize>, &W<f32>)>) {}
        fn wants_mut_u32(_: Query<(&mut W<u32>, &W<f32>)>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut_usize)
            .with_system(wants_mut_u32);
        stage.run(&mut world);
        assert_eq!(receive_events(&world), vec![StartedSystems(2),]);
    }

    #[test]
    fn world() {
        let mut world = World::new();
        world.spawn(W(0usize));
        fn wants_world(_: &World) {}
        fn wants_mut(_: Query<&mut W<usize>>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut)
            .with_system(wants_mut);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut)
            .with_system(wants_world);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_world)
            .with_system(wants_world);
        stage.run(&mut world);
        assert_eq!(receive_events(&world), vec![StartedSystems(2),]);
    }

    #[test]
    fn non_send_resource() {
        use std::thread;
        let mut world = World::new();
        world.insert_non_send_resource(thread::current().id());
        fn non_send(thread_id: NonSend<thread::ThreadId>) {
            assert_eq!(thread::current().id(), *thread_id);
        }
        fn empty() {}
        let mut stage = SystemStage::parallel()
            .with_system(non_send)
            .with_system(non_send)
            .with_system(empty)
            .with_system(empty)
            .with_system(non_send)
            .with_system(non_send);
        stage.run(&mut world);
        assert_eq!(
            receive_events(&world),
            vec![
                StartedSystems(3),
                StartedSystems(1),
                StartedSystems(1),
                StartedSystems(1),
            ]
        );
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world);
    }
}
