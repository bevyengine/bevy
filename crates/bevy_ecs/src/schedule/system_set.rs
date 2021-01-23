use crate::{
    ExclusiveSystem, ExclusiveSystemDescriptor, ParallelSystemDescriptor, Resources, RunCriteria,
    ShouldRun, System, World,
};

pub struct SystemSet {
    run_criteria: RunCriteria,
    is_dirty: bool,
    pub(crate) exclusive_descriptors: Vec<ExclusiveSystemDescriptor>,
    pub(crate) parallel_descriptors: Vec<ParallelSystemDescriptor>,
    uninitialized_parallel: Vec<usize>,
    uninitialized_exclusive: Vec<usize>,
}

impl Default for SystemSet {
    fn default() -> SystemSet {
        SystemSet {
            run_criteria: Default::default(),
            is_dirty: true,
            exclusive_descriptors: vec![],
            parallel_descriptors: vec![],
            uninitialized_parallel: vec![],
            uninitialized_exclusive: vec![],
        }
    }
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub(crate) fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        for index in self.uninitialized_exclusive.drain(..) {
            self.exclusive_descriptors[index]
                .system
                .initialize(world, resources);
        }
        for index in self.uninitialized_parallel.drain(..) {
            self.parallel_descriptors[index]
                .system_mut()
                .initialize(world, resources);
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    pub(crate) fn reset_dirty(&mut self) {
        self.is_dirty = false;
    }

    pub(crate) fn should_run(&mut self, world: &mut World, resources: &mut Resources) -> ShouldRun {
        self.run_criteria.should_run(world, resources)
    }

    pub(crate) fn exclusive_system_mut(&mut self, index: usize) -> &mut impl ExclusiveSystem {
        &mut self.exclusive_descriptors[index].system
    }

    pub(crate) fn parallel_system_mut(
        &mut self,
        index: usize,
    ) -> &mut dyn System<In = (), Out = ()> {
        self.parallel_descriptors[index].system_mut()
    }

    /// # Safety
    /// Ensure no other borrows of this system exist along with this one.
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn parallel_system_mut_unsafe(
        &self,
        index: usize,
    ) -> &mut dyn System<In = (), Out = ()> {
        self.parallel_descriptors[index].system_mut_unsafe()
    }

    pub(crate) fn parallel_systems_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut dyn System<In = (), Out = ()>> {
        self.parallel_descriptors
            .iter_mut()
            .map(|descriptor| descriptor.system_mut())
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.add_run_criteria(system);
        self
    }

    pub fn add_run_criteria<S: System<In = (), Out = ShouldRun>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.run_criteria.set(Box::new(system));
        self
    }

    pub fn with_system(mut self, system: impl Into<ParallelSystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn with_exclusive_system(mut self, system: impl Into<ExclusiveSystemDescriptor>) -> Self {
        self.add_exclusive_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl Into<ParallelSystemDescriptor>) -> &mut Self {
        self.uninitialized_parallel
            .push(self.parallel_descriptors.len());
        self.parallel_descriptors.push(system.into());

        self.is_dirty = true;
        self
    }

    pub fn add_exclusive_system(
        &mut self,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> &mut Self {
        self.uninitialized_exclusive
            .push(self.exclusive_descriptors.len());
        self.exclusive_descriptors.push(system.into());
        self.is_dirty = true;
        self
    }
}
