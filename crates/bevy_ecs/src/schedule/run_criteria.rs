use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::ComponentId,
    query::Access,
    schedule::{BoxedRunCriterionLabel, RunCriterionLabel},
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

pub(crate) struct RunCriterion {
    criteria_system: Option<BoxedSystem<(), ShouldRun>>,
    initialized: bool,
}

impl Default for RunCriterion {
    fn default() -> Self {
        Self {
            criteria_system: None,
            initialized: false,
        }
    }
}

impl RunCriterion {
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

pub enum RunCriterionDescriptor {
    Label(BoxedRunCriterionLabel),
    System {
        system: BoxedSystem<(), ShouldRun>,
        label: Option<BoxedRunCriterionLabel>,
    },
    Chain {
        parent: BoxedRunCriterionLabel,
        system: BoxedSystem<ShouldRun, ShouldRun>,
        label: Option<BoxedRunCriterionLabel>,
    },
}

pub trait IntoRunCriterionDescriptor<Marker> {
    fn into(self) -> RunCriterionDescriptor;
}

impl IntoRunCriterionDescriptor<()> for RunCriterionDescriptor {
    fn into(self) -> RunCriterionDescriptor {
        self
    }
}

pub struct SystemMarker;
pub struct LabelMarker;

impl IntoRunCriterionDescriptor<SystemMarker> for BoxedSystem<(), ShouldRun> {
    fn into(self) -> RunCriterionDescriptor {
        RunCriterionDescriptor::System {
            system: self,
            label: None,
        }
    }
}

impl<S> IntoRunCriterionDescriptor<SystemMarker> for S
where
    S: System<In = (), Out = ShouldRun>,
{
    fn into(self) -> RunCriterionDescriptor {
        RunCriterionDescriptor::System {
            system: Box::new(self),
            label: None,
        }
    }
}

impl IntoRunCriterionDescriptor<LabelMarker> for BoxedRunCriterionLabel {
    fn into(self) -> RunCriterionDescriptor {
        RunCriterionDescriptor::Label(self)
    }
}

impl<L> IntoRunCriterionDescriptor<LabelMarker> for L
where
    L: RunCriterionLabel,
{
    fn into(self) -> RunCriterionDescriptor {
        RunCriterionDescriptor::Label(Box::new(self))
    }
}

pub trait RunCriteriaLabelling {
    fn label(self, label: impl RunCriterionLabel) -> RunCriterionDescriptor;
}

impl RunCriteriaLabelling for RunCriterionDescriptor {
    fn label(self, label: impl RunCriterionLabel) -> RunCriterionDescriptor {
        let label = Some(Box::new(label) as Box<dyn RunCriterionLabel>);
        use RunCriterionDescriptor::*;
        match self {
            Label(_) => unreachable!(),
            System { system, .. } => System { system, label },
            Chain { parent, system, .. } => Chain {
                parent,
                system,
                label,
            },
        }
    }
}

impl RunCriteriaLabelling for BoxedSystem<(), ShouldRun> {
    fn label(self, label: impl RunCriterionLabel) -> RunCriterionDescriptor {
        RunCriterionDescriptor::System {
            system: self,
            label: Some(Box::new(label)),
        }
    }
}

impl<S> RunCriteriaLabelling for S
where
    S: System<In = (), Out = ShouldRun>,
{
    fn label(self, label: impl RunCriterionLabel) -> RunCriterionDescriptor {
        RunCriterionDescriptor::System {
            system: Box::new(self),
            label: Some(Box::new(label)),
        }
    }
}

pub trait RunCriteriaChaining {
    fn chain(self, system: impl System<In = ShouldRun, Out = ShouldRun>) -> RunCriterionDescriptor;
}

impl RunCriteriaChaining for BoxedRunCriterionLabel {
    fn chain(self, system: impl System<In = ShouldRun, Out = ShouldRun>) -> RunCriterionDescriptor {
        RunCriterionDescriptor::Chain {
            parent: self,
            system: Box::new(system),
            label: None,
        }
    }
}

impl<L> RunCriteriaChaining for L
where
    L: RunCriterionLabel,
{
    fn chain(self, system: impl System<In = ShouldRun, Out = ShouldRun>) -> RunCriterionDescriptor {
        RunCriterionDescriptor::Chain {
            parent: Box::new(self),
            system: Box::new(system),
            label: None,
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
}
