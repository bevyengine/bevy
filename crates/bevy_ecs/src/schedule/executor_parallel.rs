use async_channel::{Receiver, Sender};
use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
use bevy_utils::HashSet;
use fixedbitset::FixedBitSet;

use crate::{
    ArchetypesGeneration, CondensedTypeAccess, ParallelSystemContainer, ParallelSystemExecutor,
    Resources, System, World,
};

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
    /// Archetype-component access information condensed into executor-specific bitsets.
    archetype_component_access: CondensedTypeAccess,
    /// Resource access information condensed into executor-specific bitsets.
    resource_access: CondensedTypeAccess,
}

pub struct ParallelExecutor {
    /// Last archetypes generation observed by parallel systems.
    last_archetypes_generation: ArchetypesGeneration,
    /// Cached metadata of every system.
    system_metadata: Vec<SystemSchedulingMetadata>,
    /// Used by systems to notify the executor that they have finished.
    finish_sender: Sender<usize>,
    /// Receives finish events from systems.
    finish_receiver: Receiver<usize>,
    /// Systems that must run on the main thread.
    non_send: FixedBitSet,
    /// Systems that should be started at next opportunity.
    queued: FixedBitSet,
    /// Systems that are currently running.
    running: FixedBitSet,
    /// Whether a non-send system is currently running.
    non_send_running: bool,
    /// Systems that should run this iteration.
    should_run: FixedBitSet,
    /// Compound archetype-component access information of currently running systems.
    active_archetype_component_access: CondensedTypeAccess,
    /// Compound resource access information of currently running systems.
    active_resource_access: CondensedTypeAccess,
    /// Scratch space to avoid reallocating a vector when updating dependency counters.
    dependants_scratch: Vec<usize>,
    #[cfg(test)]
    events_sender: Option<Sender<SchedulingEvent>>,
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        let (finish_sender, finish_receiver) = async_channel::unbounded();
        Self {
            // MAX ensures access information will be initialized on first run.
            last_archetypes_generation: ArchetypesGeneration(u64::MAX),
            system_metadata: Default::default(),
            finish_sender,
            finish_receiver,
            non_send: Default::default(),
            queued: Default::default(),
            running: Default::default(),
            non_send_running: false,
            should_run: Default::default(),
            active_archetype_component_access: Default::default(),
            active_resource_access: Default::default(),
            dependants_scratch: Default::default(),
            #[cfg(test)]
            events_sender: None,
        }
    }
}

impl ParallelSystemExecutor for ParallelExecutor {
    fn rebuild_cached_data(&mut self, systems: &mut [ParallelSystemContainer], world: &World) {
        self.system_metadata.clear();
        self.non_send.clear();
        self.non_send.grow(systems.len());
        self.queued.grow(systems.len());
        self.running.grow(systems.len());
        self.should_run.grow(systems.len());
        // Collect all distinct types accessed by systems in order to condense their
        // access sets into bitsets.
        let mut all_archetype_components = HashSet::default();
        let mut all_resource_types = HashSet::default();
        let mut gather_distinct_access_types = |system: &dyn System<In = (), Out = ()>| {
            if let Some(archetype_components) =
                system.archetype_component_access().all_distinct_types()
            {
                all_archetype_components.extend(archetype_components);
            }
            if let Some(resources) = system.resource_access().all_distinct_types() {
                all_resource_types.extend(resources);
            }
        };
        // If the archetypes were changed too, system access should be updated
        // before gathering the types.
        if self.last_archetypes_generation != world.archetypes_generation() {
            for container in systems.iter_mut() {
                let system = container.system_mut();
                system.update_access(world);
                gather_distinct_access_types(system);
            }
            self.last_archetypes_generation = world.archetypes_generation();
        } else {
            for container in systems.iter() {
                gather_distinct_access_types(container.system());
            }
        }
        let all_archetype_components = all_archetype_components.drain().collect::<Vec<_>>();
        let all_resource_types = all_resource_types.drain().collect::<Vec<_>>();
        // Construct scheduling data for systems.
        for container in systems.iter() {
            let dependencies_total = container.dependencies().len();
            let system = container.system();
            if system.is_non_send() {
                self.non_send.insert(self.system_metadata.len());
            }
            let (start_sender, start_receiver) = async_channel::bounded(1);
            self.system_metadata.push(SystemSchedulingMetadata {
                start_sender,
                start_receiver,
                dependants: vec![],
                dependencies_total,
                dependencies_now: 0,
                archetype_component_access: system
                    .archetype_component_access()
                    .condense(&all_archetype_components),
                resource_access: system.resource_access().condense(&all_resource_types),
            });
        }
        // Populate the dependants lists in the scheduling metadata.
        for (dependant, container) in systems.iter().enumerate() {
            for dependency in container.dependencies() {
                self.system_metadata[*dependency].dependants.push(dependant);
            }
        }
    }

    fn run_systems(
        &mut self,
        systems: &mut [ParallelSystemContainer],
        world: &mut World,
        resources: &mut Resources,
    ) {
        #[cfg(test)]
        if self.events_sender.is_none() {
            let (sender, receiver) = async_channel::unbounded::<SchedulingEvent>();
            resources.insert(receiver);
            self.events_sender = Some(sender);
        }
        if self.last_archetypes_generation != world.archetypes_generation() {
            self.update_access(systems, world);
            self.last_archetypes_generation = world.archetypes_generation();
        }
        let compute_pool = resources
            .get_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
            .clone();
        compute_pool.scope(|scope| {
            self.prepare_systems(scope, systems, world, resources);
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
    /// Updates access and recondenses the archetype component bitsets of systems.
    fn update_access(&mut self, systems: &mut [ParallelSystemContainer], world: &mut World) {
        let mut all_archetype_components = HashSet::default();
        for container in systems.iter_mut() {
            let system = container.system_mut();
            system.update_access(world);
            if let Some(archetype_components) =
                system.archetype_component_access().all_distinct_types()
            {
                all_archetype_components.extend(archetype_components);
            }
        }
        let all_archetype_components = all_archetype_components.drain().collect::<Vec<_>>();
        for (index, container) in systems.iter().enumerate() {
            let system = container.system();
            if !system.archetype_component_access().reads_all() {
                self.system_metadata[index].archetype_component_access = system
                    .archetype_component_access()
                    .condense(&all_archetype_components);
            }
        }
    }

    /// Populates `should_run` bitset, spawns tasks for systems that should run this iteration,
    /// queues systems with no dependencies to run (or skip) at next opportunity.
    fn prepare_systems<'scope>(
        &mut self,
        scope: &mut Scope<'scope, ()>,
        systems: &'scope [ParallelSystemContainer],
        world: &'scope World,
        resources: &'scope Resources,
    ) {
        self.should_run.clear();
        for (index, system_data) in self.system_metadata.iter_mut().enumerate() {
            // Spawn the system task.
            if systems[index].should_run() {
                self.should_run.set(index, true);
                let start_receiver = system_data.start_receiver.clone();
                let finish_sender = self.finish_sender.clone();
                let system = unsafe { systems[index].system_mut_unsafe() };
                let task = async move {
                    start_receiver
                        .recv()
                        .await
                        .unwrap_or_else(|error| unreachable!(error));
                    unsafe { system.run_unsafe((), world, resources) };
                    finish_sender
                        .send(index)
                        .await
                        .unwrap_or_else(|error| unreachable!(error));
                };
                if self.non_send[index] {
                    scope.spawn_local(task);
                } else {
                    scope.spawn(task);
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
        !(self.non_send[index] && self.non_send_running)
            && system_data
                .resource_access
                .is_compatible(&self.active_resource_access)
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
            if !self.should_run[index] {
                self.dependants_scratch
                    .extend(&self.system_metadata[index].dependants);
            } else if self.can_start_now(index) {
                #[cfg(test)]
                {
                    started_systems += 1;
                }
                let system_data = &self.system_metadata[index];
                system_data
                    .start_sender
                    .send(())
                    .await
                    .unwrap_or_else(|error| unreachable!(error));
                self.running.set(index, true);
                if self.non_send[index] {
                    self.non_send_running = true;
                }
                // Add this system's access information to the active access information.
                self.active_archetype_component_access
                    .extend(&system_data.archetype_component_access);
                self.active_resource_access
                    .extend(&system_data.resource_access);
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
        if self.non_send[index] {
            self.non_send_running = false;
        }
        self.running.set(index, false);
        self.dependants_scratch
            .extend(&self.system_metadata[index].dependants);
    }

    /// Discards active access information and builds it again using currently
    /// running systems' access information.
    fn rebuild_active_access(&mut self) {
        self.active_archetype_component_access.clear();
        self.active_resource_access.clear();
        for index in self.running.ones() {
            self.active_archetype_component_access
                .extend(&self.system_metadata[index].archetype_component_access);
            self.active_resource_access
                .extend(&self.system_metadata[index].resource_access);
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
        self.events_sender
            .as_ref()
            .unwrap()
            .try_send(event)
            .unwrap();
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
    use crate::{prelude::*, SingleThreadedExecutor};
    use async_channel::Receiver;
    use std::thread::{self, ThreadId};

    fn receive_events(resources: &Resources) -> Vec<SchedulingEvent> {
        let mut events = Vec::new();
        while let Ok(event) = resources
            .get::<Receiver<SchedulingEvent>>()
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
        let mut resources = Resources::default();
        fn wants_for_nothing() {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_for_nothing.system())
            .with_system(wants_for_nothing.system())
            .with_system(wants_for_nothing.system());
        stage.run(&mut world, &mut resources);
        stage.run(&mut world, &mut resources);
        assert_eq!(
            receive_events(&resources),
            vec![StartedSystems(3), StartedSystems(3),]
        )
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(0usize);
        fn wants_mut(_: ResMut<usize>) {}
        fn wants_ref(_: Res<usize>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut.system())
            .with_system(wants_mut.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(
            receive_events(&resources),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut.system())
            .with_system(wants_ref.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(
            receive_events(&resources),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_ref.system())
            .with_system(wants_ref.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(receive_events(&resources), vec![StartedSystems(2),]);
    }

    #[test]
    fn queries() {
        let mut world = World::new();
        let mut resources = Resources::default();
        world.spawn((0usize,));
        fn wants_mut(_: Query<&mut usize>) {}
        fn wants_ref(_: Query<&usize>) {}
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut.system())
            .with_system(wants_mut.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(
            receive_events(&resources),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_mut.system())
            .with_system(wants_ref.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(
            receive_events(&resources),
            vec![StartedSystems(1), StartedSystems(1),]
        );
        let mut stage = SystemStage::parallel()
            .with_system(wants_ref.system())
            .with_system(wants_ref.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(receive_events(&resources), vec![StartedSystems(2),]);
    }

    #[test]
    fn non_send_resource() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert_non_send(thread::current().id());
        fn non_send(thread_id: NonSend<ThreadId>) {
            assert_eq!(thread::current().id(), *thread_id);
        }
        fn empty() {}
        let mut stage = SystemStage::parallel()
            .with_system(non_send.system())
            .with_system(non_send.system())
            .with_system(empty.system())
            .with_system(empty.system())
            .with_system(non_send.system())
            .with_system(non_send.system());
        stage.run(&mut world, &mut resources);
        assert_eq!(
            receive_events(&resources),
            vec![
                StartedSystems(3),
                StartedSystems(1),
                StartedSystems(1),
                StartedSystems(1),
            ]
        );
        stage.set_executor(Box::new(SingleThreadedExecutor::default()));
        stage.run(&mut world, &mut resources);
    }
}
