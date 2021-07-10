use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    schedule::{BoxedRunCriteriaLabel, GraphNode, RunCriteriaLabel},
    system::{BoxedSystem, IntoSystem, System, SystemId},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use std::borrow::Cow;

/// Determines whether a system should be executed or not, and how many times it should be ran each
/// time the stage is executed.
///
/// A stage will loop over its run criteria and systems until no more systems need to be executed
/// and no more run criteria need to be checked.
/// - Any systems with run criteria that returns [`Yes`] will be ran exactly one more time during
///   the stage's execution that tick.
/// - Any systems with run criteria that returns [`No`] are not ran for the rest of the stage's
///   execution that tick.
/// - Any systems with run criteria that returns [`YesAndCheckAgain`] will be ran during this
///   iteration of the loop. After all the systems that need to run are ran, that criteria will be
///   checked again.
/// - Any systems with run criteria that returns [`NoAndCheckAgain`] will not be ran during this
///   iteration of the loop. After all the systems that need to run are ran, that criteria will be
///   checked again.
///
/// [`Yes`]: ShouldRun::Yes
/// [`No`]: ShouldRun::No
/// [`YesAndCheckAgain`]: ShouldRun::YesAndCheckAgain
/// [`NoAndCheckAgain`]: ShouldRun::NoAndCheckAgain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldRun {
    /// Yes, the system should run one more time this tick.
    Yes,
    /// No, the system should not run for the rest of this tick.
    No,
    /// Yes, the system should run, and after all systems in this stage have run, the criteria
    /// should be checked again. This will cause the stage to loop over the remaining systems and
    /// criteria this tick until they no longer need to be checked.
    YesAndCheckAgain,
    /// No, the system should not run right now, but after all systems in this stage have run, the
    /// criteria should be checked again. This will cause the stage to loop over the remaining
    /// systems and criteria this tick until they no longer need to be checked.
    NoAndCheckAgain,
}

pub(crate) struct BoxedRunCriteria {
    criteria_system: Option<BoxedSystem<(), ShouldRun>>,
    initialized: bool,
}

impl Default for BoxedRunCriteria {
    fn default() -> Self {
        Self {
            criteria_system: None,
            initialized: false,
        }
    }
}

impl BoxedRunCriteria {
    pub fn set(&mut self, criteria_system: BoxedSystem<(), ShouldRun>) {
        self.criteria_system = Some(criteria_system);
        self.initialized = false;
    }

    pub fn should_run(&mut self, world: &mut World) -> ShouldRun {
        if let Some(ref mut run_criteria) = self.criteria_system {
            if !self.initialized {
                run_criteria.initialize(world);
                self.initialized = true;
            }
            let should_run = run_criteria.run((), world);
            run_criteria.apply_buffers(world);
            should_run
        } else {
            ShouldRun::Yes
        }
    }
}

pub(crate) struct RunCriteriaContainer {
    pub should_run: ShouldRun,
    pub inner: Box<dyn RunCriteriaTrait>,
    pub label: Option<BoxedRunCriteriaLabel>,
    pub before: Vec<BoxedRunCriteriaLabel>,
    pub after: Vec<BoxedRunCriteriaLabel>,
}

pub(crate) struct RunCriteriaInner<In> {
    parents: Vec<usize>,
    system: BoxedSystem<In, ShouldRun>,
}

pub(crate) trait RunCriteriaTrait: Send + Sync {
    fn evaluate_criteria(
        &mut self,
        world: &mut World,
        parents: &[RunCriteriaContainer],
    ) -> ShouldRun;

    fn name(&self) -> Cow<'static, str>;

    fn initialize(&mut self, world: &mut World);

    fn set_parents(&mut self, indices: Vec<usize>);
}

impl RunCriteriaTrait for RunCriteriaInner<ShouldRun> {
    fn evaluate_criteria(
        &mut self,
        world: &mut World,
        parents: &[RunCriteriaContainer],
    ) -> ShouldRun {
        self.system.run(parents[self.parents[0]].should_run, world)
    }

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn initialize(&mut self, world: &mut World) {
        self.system.initialize(world)
    }

    fn set_parents(&mut self, indices: Vec<usize>) {
        self.parents = indices;
    }
}

macro_rules! into_should_run {
    ($input: ident) => {
        ShouldRun
    };
}

macro_rules! into_iter_next {
    ($iterator: ident, $input: ident) => {
        $iterator.next().unwrap()
    };
}

macro_rules! impl_criteria_running {
    ($($input: ident), *) => {
        impl RunCriteriaTrait for RunCriteriaInner<($(into_should_run!($input),) *)> {
            #[allow(unused_variables, unused_mut)]
            fn evaluate_criteria(
                &mut self,
                world: &mut World,
                parents: &[RunCriteriaContainer],
            ) -> ShouldRun {
                let mut parent_iter = self.parents.iter().cloned();
                let input = ($(parents[into_iter_next!(parent_iter, $input)].should_run,) *);
                self.system.run(input, world)
            }

            fn name(&self) -> Cow<'static, str> {
                self.system.name()
            }

            fn initialize(&mut self, world: &mut World) {
                self.system.initialize(world)
            }

            fn set_parents(&mut self, indices: Vec<usize>) {
                self.parents = indices;
            }
        }
    }
}

all_tuples!(impl_criteria_running, 0, 16, I);

impl RunCriteriaContainer {
    pub fn from_descriptor(descriptor: RunCriteriaDescriptor) -> Self {
        Self {
            should_run: ShouldRun::Yes,
            inner: descriptor.system,
            label: descriptor.label,
            before: descriptor.before,
            after: descriptor.after,
        }
    }
}

impl GraphNode for RunCriteriaContainer {
    type Label = BoxedRunCriteriaLabel;

    fn name(&self) -> Cow<'static, str> {
        self.inner.name()
    }

    fn labels(&self) -> &[BoxedRunCriteriaLabel] {
        if let Some(ref label) = self.label {
            std::slice::from_ref(label)
        } else {
            &[]
        }
    }

    fn before(&self) -> &[BoxedRunCriteriaLabel] {
        &self.before
    }

    fn after(&self) -> &[BoxedRunCriteriaLabel] {
        &self.after
    }
}

pub enum RunCriteriaDescriptorOrLabel {
    Descriptor(RunCriteriaDescriptor),
    Label(BoxedRunCriteriaLabel),
}

#[derive(Clone, Copy)]
pub(crate) enum DuplicateLabelStrategy {
    Panic,
    Discard,
}

pub struct RunCriteriaDescriptor {
    pub(crate) system: Box<dyn RunCriteriaTrait>,
    pub(crate) label: Option<BoxedRunCriteriaLabel>,
    pub(crate) duplicate_label_strategy: DuplicateLabelStrategy,
    pub(crate) before: Vec<BoxedRunCriteriaLabel>,
    pub(crate) after: Vec<BoxedRunCriteriaLabel>,
}

pub trait IntoRunCriteria<Marker> {
    fn into(self) -> RunCriteriaDescriptorOrLabel;
}

impl IntoRunCriteria<RunCriteriaDescriptor> for RunCriteriaDescriptorOrLabel {
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        self
    }
}

impl IntoRunCriteria<RunCriteriaDescriptorOrLabel> for RunCriteriaDescriptor {
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Descriptor(self)
    }
}

impl IntoRunCriteria<BoxedSystem<(), ShouldRun>> for BoxedSystem<(), ShouldRun> {
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Descriptor(new_run_criteria_descriptor(self))
    }
}

impl<S, Param> IntoRunCriteria<(BoxedSystem<(), ShouldRun>, Param)> for S
where
    S: IntoSystem<(), ShouldRun, Param>,
{
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Descriptor(new_run_criteria_descriptor(Box::new(
            self.system(),
        )))
    }
}

impl IntoRunCriteria<BoxedRunCriteriaLabel> for BoxedRunCriteriaLabel {
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Label(self)
    }
}

impl<L> IntoRunCriteria<BoxedRunCriteriaLabel> for L
where
    L: RunCriteriaLabel,
{
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Label(Box::new(self))
    }
}

impl IntoRunCriteria<RunCriteria> for RunCriteria {
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Label(self.label)
    }
}

pub trait RunCriteriaDescriptorCoercion<Param> {
    /// Assigns a label to the criteria. Must be unique.
    fn label(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor;

    /// Assigns a label to the criteria. If the given label is already in use,
    /// this criteria will be discarded before initialization.
    fn label_discard_if_duplicate(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor;

    /// Specifies that this criteria must be evaluated before a criteria with the given label.
    fn before(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor;

    /// Specifies that this criteria must be evaluated after a criteria with the given label.
    fn after(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor;
}

impl RunCriteriaDescriptorCoercion<()> for RunCriteriaDescriptor {
    fn label(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.label = Some(Box::new(label));
        self.duplicate_label_strategy = DuplicateLabelStrategy::Panic;
        self
    }

    fn label_discard_if_duplicate(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.label = Some(Box::new(label));
        self.duplicate_label_strategy = DuplicateLabelStrategy::Discard;
        self
    }

    fn before(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.before.push(Box::new(label));
        self
    }

    fn after(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.after.push(Box::new(label));
        self
    }
}

fn new_run_criteria_descriptor(system: BoxedSystem<(), ShouldRun>) -> RunCriteriaDescriptor {
    RunCriteriaDescriptor {
        system: Box::new(RunCriteriaInner {
            parents: vec![],
            system,
        }),
        label: None,
        duplicate_label_strategy: DuplicateLabelStrategy::Panic,
        before: vec![],
        after: vec![],
    }
}

impl RunCriteriaDescriptorCoercion<()> for BoxedSystem<(), ShouldRun> {
    fn label(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(self).label(label)
    }

    fn label_discard_if_duplicate(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(self).label_discard_if_duplicate(label)
    }

    fn before(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(self).before(label)
    }

    fn after(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(self).after(label)
    }
}

impl<S, Param> RunCriteriaDescriptorCoercion<Param> for S
where
    S: IntoSystem<(), ShouldRun, Param>,
{
    fn label(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self.system())).label(label)
    }

    fn label_discard_if_duplicate(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self.system())).label_discard_if_duplicate(label)
    }

    fn before(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self.system())).before(label)
    }

    fn after(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self.system())).after(label)
    }
}

pub struct RunCriteria {
    label: BoxedRunCriteriaLabel,
}

impl RunCriteria {
    /// Constructs a new run criteria that will retrieve the result(s) of the criteria `labels`
    /// and pipe that as input to `system`.
    ///
    /// `labels` can be a single run criteria label, or a tuple of them.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_ecs::{prelude::*, schedule::ShouldRun};
    /// # fn system_a() {}
    /// # fn system_b() {}
    /// # fn system_c() {}
    /// # fn system_d() {}
    /// # fn some_simple_criteria() -> ShouldRun { ShouldRun::Yes }
    /// # fn another_simple_criteria() -> ShouldRun { ShouldRun::Yes }
    ///
    /// #[derive(RunCriteriaLabel, Debug, Clone, PartialEq, Eq, Hash)]
    /// enum MyCriteriaLabel {
    ///     Alpha,
    ///     Beta,
    /// }
    /// use MyCriteriaLabel::*;
    ///
    /// SystemStage::parallel()
    ///     .with_system_run_criteria(some_simple_criteria.label(Alpha))
    ///     .with_system(system_a.with_run_criteria(another_simple_criteria.label(Beta)))
    ///     .with_system(system_b.with_run_criteria(RunCriteria::pipe(
    ///         Alpha,
    ///         |In(piped): In<ShouldRun>| if piped == ShouldRun::No {
    ///             ShouldRun::Yes
    ///         } else {
    ///             ShouldRun::No
    ///         }
    ///     )))
    ///     .with_system(system_c.with_run_criteria(RunCriteria::pipe(
    ///         (Alpha, Beta),
    ///         |piped: In<(ShouldRun, ShouldRun)>| match piped {
    ///             In((ShouldRun::Yes, ShouldRun::Yes)) => ShouldRun::Yes,
    ///             _ => ShouldRun::No,
    ///         },
    ///     )))
    ///     // Alternative, short-hand syntax.
    ///     .with_system(system_d.with_run_criteria((Alpha, Beta).pipe(
    ///         |piped: In<(ShouldRun, ShouldRun)>| match piped {
    ///             In((ShouldRun::No, ShouldRun::No)) => ShouldRun::Yes,
    ///             _ => ShouldRun::No,
    ///         },
    ///     )));
    /// ```
    pub fn pipe<In, Param>(
        labels: impl RunCriteriaPiping<In>,
        system: impl IntoSystem<In, ShouldRun, Param>,
    ) -> RunCriteriaDescriptor {
        labels.pipe(system)
    }
}

pub trait RunCriteriaPiping<In> {
    /// See [`RunCriteria::pipe()`].
    fn pipe<Param>(self, system: impl IntoSystem<In, ShouldRun, Param>) -> RunCriteriaDescriptor;
}

impl<L> RunCriteriaPiping<ShouldRun> for L
where
    L: RunCriteriaLabel,
{
    fn pipe<Param>(
        self,
        system: impl IntoSystem<ShouldRun, ShouldRun, Param>,
    ) -> RunCriteriaDescriptor {
        RunCriteriaDescriptor {
            system: Box::new(RunCriteriaInner {
                parents: vec![],
                system: Box::new(system.system()),
            }),
            label: None,
            duplicate_label_strategy: DuplicateLabelStrategy::Panic,
            before: vec![],
            after: vec![Box::new(self)],
        }
    }
}

impl RunCriteriaPiping<ShouldRun> for BoxedRunCriteriaLabel {
    fn pipe<Param>(
        self,
        system: impl IntoSystem<ShouldRun, ShouldRun, Param>,
    ) -> RunCriteriaDescriptor {
        RunCriteriaDescriptor {
            system: Box::new(RunCriteriaInner {
                parents: vec![],
                system: Box::new(system.system()),
            }),
            label: None,
            duplicate_label_strategy: DuplicateLabelStrategy::Panic,
            before: vec![],
            after: vec![self],
        }
    }
}

macro_rules! into_boxed_label {
    ($label: ident) => {
        BoxedRunCriteriaLabel
    };
}

macro_rules! impl_criteria_piping {
    ($($label: ident), *) => {
        impl<$($label: RunCriteriaLabel), *> RunCriteriaPiping<($(into_should_run!($label),) *)>
        for ($($label,) *) {
            #[allow(non_snake_case)]
            fn pipe<Param>(
                self,
                system: impl IntoSystem<($(into_should_run!($label),) *), ShouldRun, Param>,
            ) -> RunCriteriaDescriptor {
                let ($($label,) *) = self;
                RunCriteriaDescriptor {
                    system: Box::new(RunCriteriaInner {
                        parents: vec![],
                        system: Box::new(system.system()),
                    }),
                    label: None,
                    duplicate_label_strategy: DuplicateLabelStrategy::Panic,
                    before: vec![],
                    after: vec![$(Box::new($label),) *],
                }
            }
        }

        impl RunCriteriaPiping<($(into_should_run!($label),) *)>
        for ($(into_boxed_label!($label),) *) {
            #[allow(non_snake_case)]
            fn pipe<Param>(
                self,
                system: impl IntoSystem<($(into_should_run!($label),) *), ShouldRun, Param>,
            ) -> RunCriteriaDescriptor {
                let ($($label,) *) = self;
                RunCriteriaDescriptor {
                    system: Box::new(RunCriteriaInner {
                        parents: vec![],
                        system: Box::new(system.system()),
                    }),
                    label: None,
                    duplicate_label_strategy: DuplicateLabelStrategy::Panic,
                    before: vec![],
                    after: vec![$($label,) *],
                }
            }
        }
    };
}

all_tuples!(impl_criteria_piping, 1, 16, L);

pub struct RunOnce {
    ran: bool,
    system_id: SystemId,
    archetype_component_access: Access<ArchetypeComponentId>,
    component_access: Access<ComponentId>,
}

impl Default for RunOnce {
    fn default() -> Self {
        Self {
            ran: false,
            system_id: SystemId::new(),
            archetype_component_access: Default::default(),
            component_access: Default::default(),
        }
    }
}

impl System for RunOnce {
    type In = ();
    type Out = ShouldRun;

    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<RunOnce>())
    }

    fn id(&self) -> SystemId {
        self.system_id
    }

    fn new_archetype(&mut self, _archetype: &Archetype) {}

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        true
    }

    unsafe fn run_unsafe(&mut self, _input: (), _world: &World) -> ShouldRun {
        if self.ran {
            ShouldRun::No
        } else {
            self.ran = true;
            ShouldRun::Yes
        }
    }

    fn apply_buffers(&mut self, _world: &mut World) {}

    fn initialize(&mut self, _world: &mut World) {}

    fn check_change_tick(&mut self, _change_tick: u32) {}
}
