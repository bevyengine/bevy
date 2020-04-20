use crate::{
    resource::{ResourceTypeId, Resources},
    system::SystemId,
};
use bit_set::BitSet;
use legion_core::{
    borrow::RefMut,
    command::CommandBuffer,
    storage::ComponentTypeId,
    world::{World, WorldId},
};
use std::cell::UnsafeCell;

#[cfg(feature = "par-schedule")]
use tracing::{span, trace, Level};

#[cfg(feature = "par-schedule")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "par-schedule")]
use fxhash::{FxHashMap, FxHashSet};

#[cfg(feature = "par-schedule")]
use rayon::prelude::*;

#[cfg(feature = "par-schedule")]
use itertools::izip;

#[cfg(feature = "par-schedule")]
use std::iter::repeat;

/// Empty trait which defines a `System` as schedulable by the dispatcher - this requires that the
/// type is both `Send` and `Sync`.
///
/// This is automatically implemented for all types that implement `Runnable` which meet the requirements.
pub trait Schedulable: Runnable + Send + Sync {}
impl<T> Schedulable for T where T: Runnable + Send + Sync {}

/// Describes which archetypes a system declares access to.
pub enum ArchetypeAccess {
    /// All archetypes.
    All,
    /// Some archetypes.
    Some(BitSet),
}

impl ArchetypeAccess {
    pub fn is_disjoint(&self, other: &ArchetypeAccess) -> bool {
        match self {
            Self::All => false,
            Self::Some(mine) => match other {
                Self::All => false,
                Self::Some(theirs) => mine.is_disjoint(theirs),
            },
        }
    }
}

/// Trait describing a schedulable type. This is implemented by `System`
pub trait Runnable {
    /// Gets the name of the system.
    fn name(&self) -> &SystemId;

    /// Gets the resources and component types read by the system.
    fn reads(&self) -> (&[ResourceTypeId], &[ComponentTypeId]);

    /// Gets the resources and component types written by the system.
    fn writes(&self) -> (&[ResourceTypeId], &[ComponentTypeId]);

    /// Prepares the system for execution against a world.
    fn prepare(&mut self, world: &World);

    /// Gets the set of archetypes the system will access when run,
    /// as determined when the system was last prepared.
    fn accesses_archetypes(&self) -> &ArchetypeAccess;

    /// Runs the system.
    ///
    /// # Safety
    ///
    /// The shared references to world and resources may result in
    /// unsound mutable aliasing if other code is accessing the same components or
    /// resources as this system. Prefer to use `run` when possible.
    unsafe fn run_unsafe(&mut self, world: &World, resources: &Resources);

    /// Gets the system's command buffer.
    fn command_buffer_mut(&self, world: WorldId) -> Option<RefMut<CommandBuffer>>;

    /// Runs the system.
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        unsafe { self.run_unsafe(world, resources) };
    }
}

/// Executes a sequence of systems, potentially in parallel, and then commits their command buffers.
///
/// Systems are provided in execution order. When the `par-schedule` feature is enabled, the `Executor`
/// may run some systems in parallel. The order in which side-effects (e.g. writes to resources
/// or entities) are observed is maintained.
pub struct Executor {
    systems: Vec<SystemBox>,
    #[cfg(feature = "par-schedule")]
    static_dependants: Vec<Vec<usize>>,
    #[cfg(feature = "par-schedule")]
    dynamic_dependants: Vec<Vec<usize>>,
    #[cfg(feature = "par-schedule")]
    static_dependency_counts: Vec<AtomicUsize>,
    #[cfg(feature = "par-schedule")]
    awaiting: Vec<AtomicUsize>,
}

struct SystemBox(UnsafeCell<Box<dyn Schedulable>>);

// NOT SAFE:
// This type is only safe to use as Send and Sync within
// the constraints of how it is used inside Executor
unsafe impl Send for SystemBox {}
unsafe impl Sync for SystemBox {}

impl SystemBox {
    #[cfg(feature = "par-schedule")]
    unsafe fn get(&self) -> &dyn Schedulable { std::ops::Deref::deref(&*self.0.get()) }

    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut(&self) -> &mut dyn Schedulable {
        std::ops::DerefMut::deref_mut(&mut *self.0.get())
    }
}

impl Executor {
    /// Constructs a new executor for all systems to be run in a single stage.
    ///
    /// Systems are provided in the order in which side-effects (e.g. writes to resources or entities)
    /// are to be observed.
    #[cfg(not(feature = "par-schedule"))]
    pub fn new(systems: Vec<Box<dyn Schedulable>>) -> Self {
        Self {
            systems: systems
                .into_iter()
                .map(|s| SystemBox(UnsafeCell::new(s)))
                .collect(),
        }
    }

    /// Constructs a new executor for all systems to be run in a single stage.
    ///
    /// Systems are provided in the order in which side-effects (e.g. writes to resources or entities)
    /// are to be observed.
    #[cfg(feature = "par-schedule")]
    #[allow(clippy::cognitive_complexity)]
    // TODO: we should break this up
    pub fn new(systems: Vec<Box<dyn Schedulable>>) -> Self {
        if systems.len() > 1 {
            let mut static_dependency_counts = Vec::with_capacity(systems.len());

            let mut static_dependants: Vec<Vec<_>> =
                repeat(Vec::with_capacity(64)).take(systems.len()).collect();
            let mut dynamic_dependants: Vec<Vec<_>> =
                repeat(Vec::with_capacity(64)).take(systems.len()).collect();

            let mut resource_last_mutated =
                FxHashMap::<ResourceTypeId, usize>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );
            let mut resource_last_read =
                FxHashMap::<ResourceTypeId, usize>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );
            let mut component_last_mutated =
                FxHashMap::<ComponentTypeId, usize>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );
            let mut component_last_read =
                FxHashMap::<ComponentTypeId, usize>::with_capacity_and_hasher(
                    64,
                    Default::default(),
                );

            for (i, system) in systems.iter().enumerate() {
                let span = span!(
                    Level::TRACE,
                    "Building system dependencies",
                    system = %system.name(),
                    index = i,
                );
                let _guard = span.enter();

                let (read_res, read_comp) = system.reads();
                let (write_res, write_comp) = system.writes();

                // find resource access dependencies
                let mut dependencies = FxHashSet::with_capacity_and_hasher(64, Default::default());
                for res in read_res {
                    trace!(resource = ?res, "Read resource");
                    if let Some(n) = resource_last_mutated.get(res) {
                        trace!(system_index = n, "Added write dependency");
                        dependencies.insert(*n);
                    }
                    resource_last_read.insert(*res, i);
                }
                for res in write_res {
                    trace!(resource = ?res, "Write resource");
                    // Writes have to be exclusive, so we are dependent on reads too
                    if let Some(n) = resource_last_read.get(res) {
                        trace!(system_index = n, "Added read dependency");
                        dependencies.insert(*n);
                    }

                    if let Some(n) = resource_last_mutated.get(res) {
                        trace!(system_index = n, "Added write dependency");
                        dependencies.insert(*n);
                    }

                    resource_last_mutated.insert(*res, i);
                }

                static_dependency_counts.push(AtomicUsize::from(dependencies.len()));
                trace!(dependants = ?dependencies, "Computed static dependants");
                for dep in dependencies {
                    static_dependants[dep].push(i);
                }

                // find component access dependencies
                let mut comp_dependencies = FxHashSet::default();
                for comp in write_comp {
                    // Writes have to be exclusive, so we are dependent on reads too
                    trace!(component = ?comp, "Write component");
                    if let Some(n) = component_last_read.get(comp) {
                        trace!(system_index = n, "Added read dependency");
                        comp_dependencies.insert(*n);
                    }
                    if let Some(n) = component_last_mutated.get(comp) {
                        trace!(system_index = n, "Added write dependency");
                        comp_dependencies.insert(*n);
                    }
                    component_last_mutated.insert(*comp, i);
                }

                // Do reads after writes to ensure we don't overwrite last_read
                for comp in read_comp {
                    trace!(component = ?comp, "Read component");
                    if let Some(n) = component_last_mutated.get(comp) {
                        trace!(system_index = n, "Added write dependency");
                        comp_dependencies.insert(*n);
                    }
                    component_last_read.insert(*comp, i);
                }

                trace!(depentants = ?comp_dependencies, "Computed dynamic dependants");
                for dep in comp_dependencies {
                    if dep != i {
                        // dont be dependent on ourselves
                        dynamic_dependants[dep].push(i);
                    }
                }
            }

            trace!(
                ?static_dependants,
                ?dynamic_dependants,
                "Computed system dependencies"
            );

            let mut awaiting = Vec::with_capacity(systems.len());
            systems
                .iter()
                .for_each(|_| awaiting.push(AtomicUsize::new(0)));

            Executor {
                awaiting,
                static_dependants,
                dynamic_dependants,
                static_dependency_counts,
                systems: systems
                    .into_iter()
                    .map(|s| SystemBox(UnsafeCell::new(s)))
                    .collect(),
            }
        } else {
            Executor {
                awaiting: Vec::with_capacity(0),
                static_dependants: Vec::with_capacity(0),
                dynamic_dependants: Vec::with_capacity(0),
                static_dependency_counts: Vec::with_capacity(0),
                systems: systems
                    .into_iter()
                    .map(|s| SystemBox(UnsafeCell::new(s)))
                    .collect(),
            }
        }
    }

    /// Converts this executor into a vector of its component systems.
    pub fn into_vec(self) -> Vec<Box<dyn Schedulable>> {
        self.systems.into_iter().map(|s| s.0.into_inner()).collect()
    }

    /// Executes all systems and then flushes their command buffers.
    pub fn execute(&mut self, world: &mut World, resources: &mut Resources) {
        self.run_systems(world, resources);
        self.flush_command_buffers(world);
    }

    /// Executes all systems sequentially.
    ///
    /// Only enabled with par-schedule is disabled
    #[cfg(not(feature = "par-schedule"))]
    pub fn run_systems(&mut self, world: &mut World, resources: &mut Resources) {
        self.systems.iter_mut().for_each(|system| {
            let system = unsafe { system.get_mut() };
            system.run(world, resources);
        });
    }

    /// Executes all systems, potentially in parallel.
    ///
    /// Ordering is retained in so far as the order of observed resource and component
    /// accesses is maintained.
    ///
    /// Call from within `rayon::ThreadPool::install()` to execute within a specific thread pool.
    #[cfg(feature = "par-schedule")]
    pub fn run_systems(&mut self, world: &mut World, resources: &mut Resources) {
        rayon::join(
            || {},
            || {
                match self.systems.len() {
                    1 => {
                        // safety: we have exlusive access to all systems, world and resources here
                        unsafe { self.systems[0].get_mut().run(world, resources) };
                    }
                    _ => {
                        let systems = &mut self.systems;
                        let static_dependency_counts = &self.static_dependency_counts;
                        let awaiting = &mut self.awaiting;

                        // prepare all systems - archetype filters are pre-executed here
                        systems
                            .par_iter_mut()
                            .for_each(|sys| unsafe { sys.get_mut() }.prepare(world));

                        // determine dynamic dependencies
                        izip!(
                            systems.iter(),
                            self.static_dependants.iter_mut(),
                            self.dynamic_dependants.iter_mut()
                        )
                        .par_bridge()
                        .for_each(|(sys, static_dep, dyn_dep)| {
                            // safety: systems is held exclusively, and we are only reading each system
                            let archetypes = unsafe { sys.get() }.accesses_archetypes();
                            for i in (0..dyn_dep.len()).rev() {
                                let dep = dyn_dep[i];
                                let other = unsafe { systems[dep].get() };

                                // if the archetype sets intersect,
                                // then we can move the dynamic dependant into the static dependants set
                                if !other.accesses_archetypes().is_disjoint(archetypes) {
                                    static_dep.push(dep);
                                    dyn_dep.swap_remove(i);
                                    static_dependency_counts[dep].fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        });

                        // initialize dependency tracking
                        for (i, count) in static_dependency_counts.iter().enumerate() {
                            awaiting[i].store(count.load(Ordering::Relaxed), Ordering::Relaxed);
                        }

                        let awaiting = &self.awaiting;

                        // execute all systems with no outstanding dependencies
                        (0..systems.len())
                            .filter(|i| awaiting[*i].load(Ordering::SeqCst) == 0)
                            .for_each(|i| {
                                // safety: we are at the root of the execution tree, so we know each
                                // index is exclusive here
                                unsafe { self.run_recursive(i, world, resources) };
                            });
                    }
                }
            },
        );
    }

    /// Flushes the recorded command buffers for all systems.
    pub fn flush_command_buffers(&mut self, world: &mut World) {
        self.systems.iter().for_each(|system| {
            // safety: systems are exlcusive due to &mut self
            let system = unsafe { system.get_mut() };
            if let Some(mut cmd) = system.command_buffer_mut(world.id()) {
                cmd.write(world);
            }
        });
    }

    /// Recursively execute through the generated depedency cascade and exhaust it.
    ///
    /// # Safety
    ///
    /// Ensure the system indexed by `i` is only accessed once.
    #[cfg(feature = "par-schedule")]
    unsafe fn run_recursive(&self, i: usize, world: &World, resources: &Resources) {
        // safety: the caller ensures nothing else is accessing systems[i]
        self.systems[i].get_mut().run_unsafe(world, resources);

        self.static_dependants[i].par_iter().for_each(|dep| {
            match self.awaiting[*dep].compare_exchange(
                1,
                std::usize::MAX,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // safety: each dependency is unique, so run_recursive is safe to call
                    self.run_recursive(*dep, world, resources);
                }
                Err(_) => {
                    self.awaiting[*dep].fetch_sub(1, Ordering::Relaxed);
                }
            }
        });
    }
}

/// A factory for `Schedule`.
pub struct Builder {
    steps: Vec<Step>,
    accumulator: Vec<Box<dyn Schedulable>>,
}

impl Builder {
    /// Adds a system to the schedule.
    pub fn add_system<T: Into<Box<dyn Schedulable>>>(mut self, system: T) -> Self {
        self.accumulator.push(system.into());
        self
    }

    /// Waits for executing systems to complete, and the flushes all outstanding system
    /// command buffers.
    pub fn flush(mut self) -> Self {
        self.finalize_executor();
        self.steps.push(Step::FlushCmdBuffers);
        self
    }

    fn finalize_executor(&mut self) {
        if !self.accumulator.is_empty() {
            let mut systems = Vec::new();
            std::mem::swap(&mut self.accumulator, &mut systems);
            let executor = Executor::new(systems);
            self.steps.push(Step::Systems(executor));
        }
    }

    /// Adds a thread local function to the schedule. This function will be executed on the main thread.
    pub fn add_thread_local_fn<F: FnMut(&mut World, &mut Resources) + 'static>(
        mut self,
        f: F,
    ) -> Self {
        self.finalize_executor();
        self.steps.push(Step::ThreadLocalFn(
            Box::new(f) as Box<dyn FnMut(&mut World, &mut Resources)>
        ));
        self
    }

    /// Adds a thread local system to the schedule. This system will be executed on the main thread.
    pub fn add_thread_local<S: Into<Box<dyn Runnable>>>(self, system: S) -> Self {
        let mut system = system.into();
        self.add_thread_local_fn(move |world, resources| system.run(world, resources))
    }

    /// Finalizes the builder into a `Schedule`.
    pub fn build(self) -> Schedule { self.into() }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            accumulator: Vec::new(),
        }
    }
}

/// A step in a schedule.
pub enum Step {
    /// A batch of systems.
    Systems(Executor),
    /// Flush system command buffers.
    FlushCmdBuffers,
    /// A thread local function.
    ThreadLocalFn(Box<dyn FnMut(&mut World, &mut Resources)>),
}

/// A schedule of systems for execution.
///
/// # Examples
///
/// ```rust
/// # use legion_core::prelude::*;
/// # use legion_systems::prelude::*;
/// # let find_collisions = SystemBuilder::new("find_collisions").build(|_,_,_,_| {});
/// # let calculate_acceleration = SystemBuilder::new("calculate_acceleration").build(|_,_,_,_| {});
/// # let update_positions = SystemBuilder::new("update_positions").build(|_,_,_,_| {});
/// let mut world = World::new();
/// let mut resources = Resources::default();
/// let mut schedule = Schedule::builder()
///     .add_system(find_collisions)
///     .flush()
///     .add_system(calculate_acceleration)
///     .add_system(update_positions)
///     .build();
///
/// schedule.execute(&mut world, &mut resources);
/// ```
pub struct Schedule {
    steps: Vec<Step>,
}

impl Schedule {
    /// Creates a new schedule builder.
    pub fn builder() -> Builder { Builder::default() }

    /// Executes all of the steps in the schedule.
    pub fn execute(&mut self, world: &mut World, resources: &mut Resources) {
        let mut waiting_flush: Vec<&mut Executor> = Vec::new();
        for step in &mut self.steps {
            match step {
                Step::Systems(executor) => {
                    executor.run_systems(world, resources);
                    waiting_flush.push(executor);
                }
                Step::FlushCmdBuffers => waiting_flush
                    .drain(..)
                    .for_each(|e| e.flush_command_buffers(world)),
                Step::ThreadLocalFn(function) => function(world, resources),
            }
        }
    }

    /// Converts the schedule into a vector of steps.
    pub fn into_vec(self) -> Vec<Step> { self.steps }
}

impl From<Builder> for Schedule {
    fn from(builder: Builder) -> Self {
        Self {
            steps: builder.flush().steps,
        }
    }
}

impl From<Vec<Step>> for Schedule {
    fn from(steps: Vec<Step>) -> Self { Self { steps } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use itertools::sorted;
    use legion_core::prelude::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn execute_in_order() {
        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Default)]
        struct Resource;

        let mut resources = Resources::default();
        resources.insert(Resource);

        let order = Arc::new(Mutex::new(Vec::new()));

        let order_clone = order.clone();
        let system_one = SystemBuilder::new("one")
            .write_resource::<Resource>()
            .build(move |_, _, _, _| order_clone.lock().unwrap().push(1usize));
        let order_clone = order.clone();
        let system_two = SystemBuilder::new("two")
            .write_resource::<Resource>()
            .build(move |_, _, _, _| order_clone.lock().unwrap().push(2usize));
        let order_clone = order.clone();
        let system_three = SystemBuilder::new("three")
            .write_resource::<Resource>()
            .build(move |_, _, _, _| order_clone.lock().unwrap().push(3usize));

        let mut schedule = Schedule::builder()
            .add_system(system_one)
            .add_system(system_two)
            .add_system(system_three)
            .build();

        schedule.execute(&mut world, &mut resources);

        let order = order.lock().unwrap();
        let sorted: Vec<usize> = sorted(order.clone()).collect();
        assert_eq!(*order, sorted);
    }

    #[test]
    fn flush() {
        let universe = Universe::new();
        let mut world = universe.create_world();
        let mut resources = Resources::default();

        #[derive(Clone, Copy, Debug, PartialEq)]
        struct TestComp(f32, f32, f32);

        let system_one = SystemBuilder::new("one").build(move |cmd, _, _, _| {
            cmd.insert((), vec![(TestComp(0., 0., 0.),)]);
        });
        let system_two = SystemBuilder::new("two")
            .with_query(Write::<TestComp>::query())
            .build(move |_, world, _, query| assert_eq!(0, query.iter_mut(world).count()));
        let system_three = SystemBuilder::new("three")
            .with_query(Write::<TestComp>::query())
            .build(move |_, world, _, query| assert_eq!(1, query.iter_mut(world).count()));

        let mut schedule = Schedule::builder()
            .add_system(system_one)
            .add_system(system_two)
            .flush()
            .add_system(system_three)
            .build();

        schedule.execute(&mut world, &mut resources);
    }
}
