use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    schedule::{BoxedRunCriteriaLabel, GraphNode, RunCriteriaLabel},
    system::{BoxedSystem, System, SystemId},
    world::World,
};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldRun {
    /// Yes, the system should run.
    Yes,
    /// No, the system should not run.
    No,
    /// Yes, the system should run, and afterwards the criteria should be checked again.
    YesAndCheckAgain,
    /// No, the system should not run right now, but the criteria should be checked again later.
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

pub(crate) enum RunCriteriaInner {
    Single(BoxedSystem<(), ShouldRun>),
    Piped {
        input: usize,
        system: BoxedSystem<ShouldRun, ShouldRun>,
    },
}

pub(crate) struct RunCriteriaContainer {
    pub should_run: ShouldRun,
    pub inner: RunCriteriaInner,
    pub label: Option<BoxedRunCriteriaLabel>,
    pub before: Vec<BoxedRunCriteriaLabel>,
    pub after: Vec<BoxedRunCriteriaLabel>,
}

impl RunCriteriaContainer {
    pub fn from_descriptor(descriptor: RunCriteriaDescriptor) -> Self {
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

    pub fn name(&self) -> Cow<'static, str> {
        match &self.inner {
            RunCriteriaInner::Single(system) => system.name(),
            RunCriteriaInner::Piped { system, .. } => system.name(),
        }
    }

    pub fn initialize(&mut self, world: &mut World) {
        match &mut self.inner {
            RunCriteriaInner::Single(system) => system.initialize(world),
            RunCriteriaInner::Piped { system, .. } => system.initialize(world),
        }
    }
}

impl GraphNode<BoxedRunCriteriaLabel> for RunCriteriaContainer {
    fn name(&self) -> Cow<'static, str> {
        match &self.inner {
            RunCriteriaInner::Single(system) => system.name(),
            RunCriteriaInner::Piped { system, .. } => system.name(),
        }
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
    pub(crate) system: RunCriteriaSystem,
    pub(crate) label: Option<BoxedRunCriteriaLabel>,
    pub(crate) duplicate_label_strategy: DuplicateLabelStrategy,
    pub(crate) before: Vec<BoxedRunCriteriaLabel>,
    pub(crate) after: Vec<BoxedRunCriteriaLabel>,
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

impl<S> IntoRunCriteria<BoxedSystem<(), ShouldRun>> for S
where
    S: System<In = (), Out = ShouldRun>,
{
    fn into(self) -> RunCriteriaDescriptorOrLabel {
        RunCriteriaDescriptorOrLabel::Descriptor(new_run_criteria_descriptor(Box::new(self)))
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

pub trait RunCriteriaDescriptorCoercion {
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

impl RunCriteriaDescriptorCoercion for RunCriteriaDescriptor {
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
        system: RunCriteriaSystem::Single(system),
        label: None,
        duplicate_label_strategy: DuplicateLabelStrategy::Panic,
        before: vec![],
        after: vec![],
    }
}

impl RunCriteriaDescriptorCoercion for BoxedSystem<(), ShouldRun> {
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

impl<S> RunCriteriaDescriptorCoercion for S
where
    S: System<In = (), Out = ShouldRun>,
{
    fn label(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self)).label(label)
    }

    fn label_discard_if_duplicate(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self)).label_discard_if_duplicate(label)
    }

    fn before(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self)).before(label)
    }

    fn after(self, label: impl RunCriteriaLabel) -> RunCriteriaDescriptor {
        new_run_criteria_descriptor(Box::new(self)).after(label)
    }
}

pub struct RunCriteria {
    label: BoxedRunCriteriaLabel,
}

impl RunCriteria {
    /// Constructs a new run criteria that will retrieve the result of the criteria `label`
    /// and pipe it as input to `system`.
    pub fn pipe(
        label: impl RunCriteriaLabel,
        system: impl System<In = ShouldRun, Out = ShouldRun>,
    ) -> RunCriteriaDescriptor {
        label.pipe(system)
    }
}

pub trait RunCriteriaPiping {
    /// See [`RunCriteria::pipe()`].
    fn pipe(self, system: impl System<In = ShouldRun, Out = ShouldRun>) -> RunCriteriaDescriptor;
}

impl RunCriteriaPiping for BoxedRunCriteriaLabel {
    fn pipe(self, system: impl System<In = ShouldRun, Out = ShouldRun>) -> RunCriteriaDescriptor {
        RunCriteriaDescriptor {
            system: RunCriteriaSystem::Piped(Box::new(system)),
            label: None,
            duplicate_label_strategy: DuplicateLabelStrategy::Panic,
            before: vec![],
            after: vec![self],
        }
    }
}

impl<L> RunCriteriaPiping for L
where
    L: RunCriteriaLabel,
{
    fn pipe(self, system: impl System<In = ShouldRun, Out = ShouldRun>) -> RunCriteriaDescriptor {
        RunCriteriaDescriptor {
            system: RunCriteriaSystem::Piped(Box::new(system)),
            label: None,
            duplicate_label_strategy: DuplicateLabelStrategy::Panic,
            before: vec![],
            after: vec![Box::new(self)],
        }
    }
}

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

    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: &World) -> Self::Out {
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
