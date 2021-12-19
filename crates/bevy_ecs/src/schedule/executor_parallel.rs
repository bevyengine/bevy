use crate::{
    archetype::{ArchetypeComponentId, ArchetypeGeneration},
    query::Access,
    schedule::{ParallelSystemContainer, ParallelSystemExecutor},
    world::World,
};
use async_channel::{Receiver, Sender};
use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
use fixedbitset::FixedBitSet;

#[cfg(test)]
use SchedulingEvent::*;

struct SystemSchedulingMetadata {
    /// Used to signal the system's task to start the system.
    start_sender: Sender<()>,
    /// Receives the signal to start the system.
    start_receiver: Receiver<()>,
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
    /// Last archetypes generation observed by parallel systems.
    archetype_generation: ArchetypeGeneration,
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
        let (finish_sender, finish_receiver) = async_channel::unbounded();
        Self {
            archetype_generation: ArchetypeGeneration::initial(),
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
    fn rebuild_cached_data(&mut self, systems: &[ParallelSystemContainer]) {
        self.system_metadata.clear();
        self.queued.grow(systems.len());
        self.running.grow(systems.len());
        self.should_run.grow(systems.len());

        // Construct scheduling data for systems.
        for container in systems.iter() {
            let dependencies_total = container.dependencies().len();
            let system = container.system();
            let (start_sender, start_receiver) = async_channel::bounded(1);
            self.system_metadata.push(SystemSchedulingMetadata {
                start_sender,
                start_receiver,
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

    fn run_systems(&mut self, systems: &mut [ParallelSystemContainer], world: &mut World) {
        #[cfg(test)]
        if self.events_sender.is_none() {
            let (sender, receiver) = async_channel::unbounded::<SchedulingEvent>();
            world.insert_resource(receiver);
            self.events_sender = Some(sender);
        }

        self.update_archetypes(systems, world);

        let compute_pool = world
            .get_resource_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
            .clone();
        compute_pool.scope(|scope| {
            self.prepare_systems(scope, systems, world);
            scope.spawn(async {
                // All systems have been ran if there are no queued or running systems.
                while 0 != self.queued.count_ones(..) + self.running.count_ones(..) {
                    self.process_queued_systems().await;
                    // Avoid deadlocking if no systems were actually started.
                    if self.running.count_ones(..) != 0 {
                        // Wait until at least one system has finished.
                        let index = self
                            .finish_receiver
                            .recv()
                            .await
                            .unwrap_or_else(|error| unreachable!(error));
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
            });
        });
    }
}

impl ParallelExecutor {
    /// Calls system.new_archetype() for each archetype added since the last call to
    /// [update_archetypes] and updates cached archetype_component_access.
    fn update_archetypes(&mut self, systems: &mut [ParallelSystemContainer], world: &World) {
        #[cfg(feature = "trace")]
        let span = bevy_utils::tracing::info_span!("update_archetypes");
        #[cfg(feature = "trace")]
        let _guard = span.enter();
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype in archetypes.archetypes[archetype_index_range].iter() {
            for (index, container) in systems.iter_mut().enumerate() {
                let meta = &mut self.system_metadata[index];
                let system = container.system_mut();
                system.new_archetype(archetype);
                meta.archetype_component_access
                    .extend(system.archetype_component_access());
            }
        }
    }

    /// Populates `should_run` bitset, spawns tasks for systems that should run this iteration,
    /// queues systems with no dependencies to run (or skip) at next opportunity.
    fn prepare_systems<'scope>(
        &mut self,
        scope: &mut Scope<'scope, ()>,
        systems: &'scope mut [ParallelSystemContainer],
        world: &'scope World,
    ) {
        #[cfg(feature = "trace")]
        let span = bevy_utils::tracing::info_span!("prepare_systems");
        #[cfg(feature = "trace")]
        let _guard = span.enter();
        self.should_run.clear();
        for (index, (system_data, system)) in
            self.system_metadata.iter_mut().zip(systems).enumerate()
        {
            // Spawn the system task.
            if system.should_run() {
                self.should_run.set(index, true);
                let start_receiver = system_data.start_receiver.clone();
                let finish_sender = self.finish_sender.clone();
                let system = system.system_mut();
                #[cfg(feature = "trace")] // NB: outside the task to get the TLS current span
                let system_span = bevy_utils::tracing::info_span!("system", name = &*system.name());
                let task = async move {
                    start_receiver
                        .recv()
                        .await
                        .unwrap_or_else(|error| unreachable!(error));
                    #[cfg(feature = "trace")]
                    let system_guard = system_span.enter();
                    unsafe { system.run_unsafe((), world) };
                    #[cfg(feature = "trace")]
                    drop(system_guard);
                    finish_sender
                        .send(index)
                        .await
                        .unwrap_or_else(|error| unreachable!(error));
                };
                if system_data.is_send {
                    scope.spawn(task);
                } else {
                    scope.spawn_local(task);
                }
            }
            // Queue the system if it has no dependencies, otherwise reset its dependency counter.
            if system_data.dependencies_total == 0 {
                self.queued.insert(index);
            } else {
                system_data.dependencies_now = system_data.dependencies_total;
            }
        }
    }

    /// Determines if the system with given index has no conflicts with already running systems.
    fn can_start_now(&self, index: usize) -> bool {
        let system_data = &self.system_metadata[index];
        // Non-send systems are considered conflicting with each other.
        (!self.non_send_running || system_data.is_send)
            && system_data
                .archetype_component_access
                .is_compatible(&self.active_archetype_component_access)
    }

    /// Starts all non-conflicting queued systems, moves them from `queued` to `running`,
    /// adds their access information to active access information;
    /// processes queued systems that shouldn't run this iteration as completed immediately.
    async fn process_queued_systems(&mut self) {
        #[cfg(test)]
        let mut started_systems = 0;
        for index in self.queued.ones() {
            // If the system shouldn't actually run this iteration, process it as completed
            // immediately; otherwise, check for conflicts and signal its task to start.
            let system_metadata = &self.system_metadata[index];
            if !self.should_run[index] {
                self.dependants_scratch.extend(&system_metadata.dependants);
            } else if self.can_start_now(index) {
                #[cfg(test)]
                {
                    started_systems += 1;
                }
                system_metadata
                    .start_sender
                    .send(())
                    .await
                    .unwrap_or_else(|error| unreachable!(error));
                self.running.set(index, true);
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
            self.emit_event(StartedSystems(started_systems));
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
#[derive(Debug, PartialEq, Eq)]
enum SchedulingEvent {
    StartedSystems(usize),
}

#[cfg(test)]
mod tests {
    use super::SchedulingEvent::{self, *};
    use crate::{
        schedule::{SingleThreadedExecutor, Stage, SystemStage},
        system::{NonSend, Query, Res, ResMut},
        world::World,
    };
    use async_channel::Receiver;

    use crate as bevy_ecs;
    use crate::component::Component;
    #[derive(Component)]
    struct W<T>(T);

    fn receive_events(world: &World) -> Vec<SchedulingEvent> {
        let mut events = Vec::new();
        while let Ok(event) = world
            .get_resource::<Receiver<SchedulingEvent>>()
            .unwrap()
            .try_recv()
        {
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
        )
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        world.insert_resource(0usize);
        fn wants_mut(_: ResMut<usize>) {}
        fn wants_ref(_: Res<usize>) {}
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
        world.spawn().insert(W(0usize));
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
        world.spawn().insert_bundle((W(0usize), W(0u32), W(0f32)));
        fn wants_mut_usize(_: Query<(&mut W<usize>, &W<f32>)>) {}
        fn wants_mut_u32(_: Query<(&mut W<u32>, &W<f32>)>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut_usize)
            .with_system(wants_mut_u32);
        stage.run(&mut world);
        assert_eq!(receive_events(&world), vec![StartedSystems(2),]);
    }

    #[test]
    fn non_send_resource() {
        use std::thread;
        let mut world = World::new();
        world.insert_non_send(thread::current().id());
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
