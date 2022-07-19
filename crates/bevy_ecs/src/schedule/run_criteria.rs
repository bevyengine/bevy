use crate::{
    schedule::{GraphNode, RunCriteriaLabel, RunCriteriaLabelId},
    system::{BoxedSystem, IntoSystem, Local},
    world::World,
};
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

impl ShouldRun {
    /// A run criterion which returns [`ShouldRun::Yes`] exactly once.
    ///
    /// This leads to the systems controlled by it only being
    /// executed one time only.
    pub fn once(mut ran: Local<bool>) -> ShouldRun {
        if *ran {
            ShouldRun::No
        } else {
            *ran = true;
            ShouldRun::Yes
        }
    }
}

impl From<bool> for ShouldRun {
    fn from(value: bool) -> Self {
        if value {
            ShouldRun::Yes
        } else {
            ShouldRun::No
        }
    }
}

#[derive(Default)]
pub(crate) struct BoxedRunCriteria {
    criteria_system: Option<BoxedSystem<(), ShouldRun>>,
    initialized: bool,
}

impl BoxedRunCriteria {
    pub(crate) fn set(&mut self, criteria_system: BoxedSystem<(), ShouldRun>) {
        self.criteria_system = Some(criteria_system);
        self.initialized = false;
    }

    pub(crate) fn should_run(&mut self, world: &mut World) -> ShouldRun {
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

pub(crate) enum RunCriteriaInner {
    Single(BoxedSystem<(), ShouldRun>),
    Piped {
        input: usize,
        system: BoxedSystem<ShouldRun, ShouldRun>,
    },
}

pub(crate) struct RunCriteriaContainer {
    pub(crate) should_run: ShouldRun,
    pub(crate) inner: RunCriteriaInner,
    pub(crate) label: Option<RunCriteriaLabelId>,
    pub(crate) before: Vec<RunCriteriaLabelId>,
    pub(crate) after: Vec<RunCriteriaLabelId>,
}

impl RunCriteriaContainer {
    pub(crate) fn from_descriptor(descriptor: RunCriteriaDescriptor) -> Self {
        Self {
            should_run: ShouldRun::Yes,
            inner: match descriptor.system {
                RunCriteriaSystem::Single(system) => RunCriteriaInner::Single(system),
                RunCriteriaSystem::Piped(system) => RunCriteriaInner::Piped { input: 0, system },
            },
            label: descriptor.label,
            before: descriptor.before,
            after: descriptor.after,
        }
    }

    pub(crate) fn name(&self) -> Cow<'static, str> {
        match &self.inner {
            RunCriteriaInner::Single(system) => system.name(),
            RunCriteriaInner::Piped { system, .. } => system.name(),
        }
    }

    pub(crate) fn initialize(&mut self, world: &mut World) {
        match &mut self.inner {
            RunCriteriaInner::Single(system) => system.initialize(world),
            RunCriteriaInner::Piped { system, .. } => system.initialize(world),
        }
    }
}

impl GraphNode for RunCriteriaContainer {
    type Label = RunCriteriaLabelId;

    fn name(&self) -> Cow<'static, str> {
        match &self.inner {
            RunCriteriaInner::Single(system) => system.name(),
            RunCriteriaInner::Piped { system, .. } => system.name(),
        }
    }

    fn labels(&self) -> &[RunCriteriaLabelId] {
        if let Some(ref label) = self.label {
            std::slice::from_ref(label)
        } else {
            &[]
        }
    }

    fn before(&self) -> &[RunCriteriaLabelId] {
        &self.before
    }

    fn after(&self) -> &[RunCriteriaLabelId] {
        &self.after
    }
}

pub enum RunCriteriaDescriptorOrLabel {
    Descriptor(RunCriteriaDescriptor),
    Label(RunCriteriaLabelId),
}

#[derive(Clone, Copy)]
pub(crate) enum DuplicateLabelStrategy {
    Panic,
    Discard,
}

pub struct RunCriteriaDescriptor {
    pub(crate) system: RunCriteriaSystem,
    pub(crate) label: Option<RunCriteriaLabelId>,
    pub(crate) duplicate_label_strategy: DuplicateLabelStrategy,
    pub(crate) before: Vec<RunCriteriaLabelId>,
    pub(crate) after: Vec<RunCriteriaLabelId>,
}

pub(crate) enum RunCriteriaSystem {
    Single(BoxedSystem<(), ShouldRun>),
    Piped(BoxedSystem<ShouldRun, ShouldRun>),
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
            IntoSystem::into_system(self),
        )))
    }
}

impl<L> IntoRunCriteria<RunCriteriaLabelId> for L
where
    L: RunCriteriaLabel,
{
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Label(self.as_label())
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
        self.label = Some(label.as_label());
        self.duplicate_label_strategy = DuplicateLabelStrategy::Panic;
        self
    }

    fn label_discard_if_duplicate(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.label = Some(label.as_label());
        self.duplicate_label_strategy = DuplicateLabelStrategy::Discard;
        self
    }

    fn before(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.before.push(label.as_label());
        self
    }

    fn after(mut self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        self.after.push(label.as_label());
        self
    }
}

fn new_run_criteria_descriptor(system: BoxedSystem<(), ShouldRun>) -> RunCriteriaDescriptor {
    RunCriteriaDescriptor {
        system: RunCriteriaSystem::Single(system),
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
        new_run_criteria_descriptor(Box::new(IntoSystem::into_system(self))).label(label)
    }

    fn label_discard_if_duplicate(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(IntoSystem::into_system(self)))
            .label_discard_if_duplicate(label)
    }

    fn before(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(IntoSystem::into_system(self))).before(label)
    }

    fn after(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(IntoSystem::into_system(self))).after(label)
    }
}

pub struct RunCriteria {
    label: RunCriteriaLabelId,
}

impl RunCriteria {
    /// Constructs a new run criteria that will retrieve the result of the criteria `label`
    /// and pipe it as input to `system`.
    pub fn pipe<P>(
        label: impl RunCriteriaLabel,
        system: impl IntoSystem<ShouldRun, ShouldRun, P>,
    ) -> RunCriteriaDescriptor {
        RunCriteriaDescriptor {
            system: RunCriteriaSystem::Piped(Box::new(IntoSystem::into_system(system))),
            label: None,
            duplicate_label_strategy: DuplicateLabelStrategy::Panic,
            before: vec![],
            after: vec![label.as_label()],
        }
    }
}
