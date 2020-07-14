use crate::{system::ThreadLocalExecution, Resources, Schedule, System};
use crossbeam_channel::{Receiver, Sender};
use fixedbitset::FixedBitSet;
use hecs::{Access, Query, World};
use rayon::ScopeFifo;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Executor {
    stages: Vec<ExecutorStage>,
}

impl Executor {
    pub fn prepare(&mut self, schedule: &mut Schedule, world: &World) {
        // TODO: if world archetype generation hasnt changed, dont update here

        let mut executor_stages = vec![ExecutorStage::default(); schedule.stage_order.len()];
        for (stage_index, stage_name) in schedule.stage_order.iter().enumerate() {
            let executor_stage = &mut executor_stages[stage_index];
            // ensure finished dependencies has the required number of bits
            if let Some(systems) = schedule.stages.get(stage_name) {
                executor_stage.prepare(world, systems);
            }
        }

        self.stages = executor_stages;
    }

    pub fn run(&mut self, schedule: &mut Schedule, world: &mut World, resources: &mut Resources) {
        for (stage_name, executor_stage) in schedule.stage_order.iter().zip(self.stages.iter_mut())
        {
            if let Some(stage_systems) = schedule.stages.get_mut(stage_name) {
                executor_stage.run(world, resources, stage_systems);
            }
        }
    }
}

#[derive(Clone)]
pub struct ExecutorStage {
    /// each system's set of dependencies
    system_dependencies: Vec<FixedBitSet>,
    /// each system's dependents (the systems that can't run until this system has run)
    system_dependents: Vec<Vec<usize>>,

    /// the currently finished systems
    finished_systems: FixedBitSet,
    running_systems: FixedBitSet,

    sender: Sender<usize>,
    receiver: Receiver<usize>,
}

impl Default for ExecutorStage {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            system_dependents: Default::default(),
            system_dependencies: Default::default(),
            finished_systems: Default::default(),
            running_systems: Default::default(),
            sender,
            receiver,
        }
    }
}

impl ExecutorStage {
    pub fn prepare(&mut self, world: &World, systems: &Vec<Arc<Mutex<Box<dyn System>>>>) {
        self.system_dependencies = vec![FixedBitSet::with_capacity(systems.len()); systems.len()];
        self.system_dependents = vec![Vec::new(); systems.len()];
        self.finished_systems.grow(systems.len());
        self.running_systems.grow(systems.len());

        // update each system's archetype access to latest world archetypes
        for system in systems.iter() {
            let mut system = system.lock().unwrap();
            system.update_archetype_access(world);
        }

        let mut current_archetype_access = ArchetypeAccess::default();
        for (system_index, system) in systems.iter().enumerate() {
            let system = system.lock().unwrap();
            if let Some(archetype_access) = system.get_archetype_access() {
                // TODO: check if thread local and add full sync
                // if any system before this one conflicts, check all systems that came before for compatibility
                if current_archetype_access.is_compatible(archetype_access) == false {
                    for earlier_system_index in 0..system_index {
                        let earlier_system = systems[earlier_system_index].lock().unwrap();
                        if let Some(earlier_archetype_access) =
                            earlier_system.get_archetype_access()
                        {
                            // if earlier system is incompatible, make the current system dependent
                            if earlier_archetype_access.is_compatible(archetype_access) == false {
                                self.system_dependents[earlier_system_index].push(system_index);
                                self.system_dependencies[system_index].insert(earlier_system_index);
                            }
                        }
                    }
                }

                current_archetype_access.union(archetype_access);
            }
        }
    }

    pub fn run_ready_systems<'run>(
        &mut self,
        systems: &[Arc<Mutex<Box<dyn System>>>],
        scope: &ScopeFifo<'run>,
        world: &'run World,
        resources: &'run Resources,
    ) {
        for i in 0..systems.len() {
            Self::try_run_system(
                systems,
                i,
                &mut self.running_systems,
                &self.finished_systems,
                &self.system_dependencies,
                &self.sender,
                scope,
                world,
                resources,
            );
        }
    }

    #[inline]
    pub fn try_run_system<'run>(
        systems: &[Arc<Mutex<Box<dyn System>>>],
        system_index: usize,
        running_systems: &mut FixedBitSet,
        finished_systems: &FixedBitSet,
        system_dependencies: &[FixedBitSet],
        sender: &Sender<usize>,
        scope: &ScopeFifo<'run>,
        world: &'run World,
        resources: &'run Resources,
    ) {
        if running_systems.contains(system_index) {
            return;
        }

        // if all system dependencies are finished, queue up the system to run
        if system_dependencies[system_index].is_subset(&finished_systems) {
            let system = systems[system_index].clone();
            let sender = sender.clone();
            running_systems.insert(system_index);
            scope.spawn_fifo(move |_| {
                let mut system = system.lock().unwrap();
                system.run(world, resources);
                sender.send(system_index).unwrap();
            })
        }
    }

    pub fn run(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        systems: &[Arc<Mutex<Box<dyn System>>>],
    ) {
        self.finished_systems.clear();
        self.running_systems.clear();
        {
            let world = &*world;
            let resources = &*resources;

            rayon::scope_fifo(move |scope| {
                self.run_ready_systems(systems, scope, world, resources);
                loop {
                    if self.finished_systems.count_ones(..) == systems.len() {
                        break;
                    }

                    let finished_system = self.receiver.recv().unwrap();
                    self.finished_systems.insert(finished_system);
                    for dependent_system in self.system_dependents[finished_system].iter() {
                        Self::try_run_system(
                            systems,
                            *dependent_system,
                            &mut self.running_systems,
                            &self.finished_systems,
                            &self.system_dependencies,
                            &self.sender,
                            scope,
                            world,
                            resources,
                        );
                    }
                }
            });
        }

        // "flush"
        // NOTE: when this is made parallel a full sync is required here
        for system in systems.iter() {
            let mut system = system.lock().unwrap();
            match system.thread_local_execution() {
                ThreadLocalExecution::NextFlush => system.run_thread_local(world, resources),
                ThreadLocalExecution::Immediate => {
                    // TODO: this should happen immediately after thread local systems
                    system.run_thread_local(world, resources)
                }
            }
        }
    }
}

// credit to Ratysz from the Yaks codebase
#[derive(Default)]
pub struct ArchetypeAccess {
    pub immutable: FixedBitSet,
    pub mutable: FixedBitSet,
}

impl ArchetypeAccess {
    pub fn is_compatible(&self, other: &ArchetypeAccess) -> bool {
        self.mutable.is_disjoint(&other.mutable)
            && self.mutable.is_disjoint(&other.immutable)
            && self.immutable.is_disjoint(&other.mutable)
    }

    pub fn union(&mut self, other: &ArchetypeAccess) {
        self.mutable.union_with(&other.mutable);
        self.immutable.union_with(&other.immutable);
    }

    pub fn set_bits_for_query<Q>(&mut self, world: &World)
    where
        Q: Query,
    {
        self.immutable.clear();
        self.mutable.clear();
        let iterator = world.archetypes();
        let bits = iterator.len();
        self.immutable.grow(bits);
        self.mutable.grow(bits);
        iterator
            .enumerate()
            .filter_map(|(index, archetype)| archetype.access::<Q>().map(|access| (index, access)))
            .for_each(|(archetype, access)| match access {
                Access::Read => self.immutable.set(archetype, true),
                Access::Write => self.mutable.set(archetype, true),
                Access::Iterate => (),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::Executor;
    use crate::{IntoQuerySystem, Query, Res, Resources, Schedule, World};
    use fixedbitset::FixedBitSet;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Counter {
        count: Arc<Mutex<usize>>,
    }

    fn read_u32(counter: Res<Counter>, _query: Query<&u32>) {
        let mut count = counter.count.lock().unwrap();
        assert!(
            *count < 2,
            "read_32 should run be one of the first two systems to run"
        );
        *count += 1;
    }

    fn write_float(counter: Res<Counter>, _query: Query<&f32>) {
        let mut count = counter.count.lock().unwrap();
        assert!(
            *count < 2,
            "write_float should run be one of the first two systems to run"
        );
        *count += 1;
    }

    fn read_u32_write_u64(counter: Res<Counter>, _query: Query<(&u32, &mut u64)>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 2,
            "read_u32_write_u64 should always be the third system to run"
        );
        *count += 1;
    }

    fn read_u64(counter: Res<Counter>, _query: Query<&u64>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 3,
            "read_u64 should always be the fourth system to run"
        );
        *count += 1;
    }

    fn write_u64(counter: Res<Counter>, _query: Query<&mut u64>) {
        let mut count = counter.count.lock().unwrap();
        assert_eq!(
            *count, 4,
            "write_u64 should always be the fifth system to run"
        );
        *count += 1;
    }

    #[test]
    fn schedule() {
        let mut world = World::new();
        let mut resources = Resources::default();
        resources.insert(Counter::default());

        world.spawn((1.0f32,));
        world.spawn((1u32, 1u64));
        world.spawn((2u32,));

        let mut schedule = Schedule::default();
        schedule.add_stage("A");
        schedule.add_stage("B");

        schedule.add_system_to_stage("A", read_u32.system());
        schedule.add_system_to_stage("A", write_float.system());
        schedule.add_system_to_stage("A", read_u32_write_u64.system());
        schedule.add_system_to_stage("A", read_u64.system());
        schedule.add_system_to_stage("B", write_u64.system());

        let mut executor = Executor::default();
        executor.prepare(&mut schedule, &world);

        assert_eq!(
            executor.stages[0].system_dependents,
            vec![vec![2], vec![], vec![3], vec![]]
        );
        assert_eq!(executor.stages[1].system_dependents, vec![vec![]]);

        let stage_0_len = executor.stages[0].system_dependencies.len();
        let mut read_u32_write_u64_deps = FixedBitSet::with_capacity(stage_0_len);
        read_u32_write_u64_deps.insert(0);
        let mut read_u64_deps = FixedBitSet::with_capacity(stage_0_len);
        read_u64_deps.insert(2);

        assert_eq!(
            executor.stages[0].system_dependencies,
            vec![
                FixedBitSet::with_capacity(stage_0_len),
                FixedBitSet::with_capacity(stage_0_len),
                read_u32_write_u64_deps,
                read_u64_deps,
            ]
        );

        assert_eq!(
            executor.stages[1].system_dependencies,
            vec![
                FixedBitSet::with_capacity(1),
            ]
        );

        executor.run(&mut schedule, &mut world, &mut resources);

        let counter = resources.get::<Counter>().unwrap();
        assert_eq!(
            *counter.count.lock().unwrap(),
            5,
            "counter should have been incremented once for each system"
        );
    }
}
