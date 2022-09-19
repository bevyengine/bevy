use crate::{
    component::ComponentId,
    query::Access,
    schedule::{
        ExclusiveSystemDescriptor, GraphNode, ParallelSystemDescriptor, RunCriteriaLabelId,
        SystemLabelId,
    },
    system::{ExclusiveSystem, System},
};
use std::borrow::Cow;

/// System metadata like its name, labels, order requirements and component access.
pub trait SystemContainer: GraphNode<Label = SystemLabelId> {
    #[doc(hidden)]
    fn dependencies(&self) -> &[usize];
    #[doc(hidden)]
    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>);
    #[doc(hidden)]
    fn run_criteria(&self) -> Option<usize>;
    #[doc(hidden)]
    fn set_run_criteria(&mut self, index: usize);
    fn run_criteria_label(&self) -> Option<&RunCriteriaLabelId>;
    fn component_access(&self) -> Option<&Access<ComponentId>>;
}

pub(super) struct ExclusiveSystemContainer {
    system: Box<dyn ExclusiveSystem>,
    pub(super) run_criteria_index: Option<usize>,
    pub(super) run_criteria_label: Option<RunCriteriaLabelId>,
    dependencies: Vec<usize>,
    labels: Vec<SystemLabelId>,
    before: Vec<SystemLabelId>,
    after: Vec<SystemLabelId>,
}

impl ExclusiveSystemContainer {
    pub(super) fn from_descriptor(descriptor: ExclusiveSystemDescriptor) -> Self {
        ExclusiveSystemContainer {
            system: descriptor.system,
            run_criteria_index: None,
            run_criteria_label: None,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
        }
    }

    pub(super) fn system_mut(&mut self) -> &mut Box<dyn ExclusiveSystem> {
        &mut self.system
    }
}

impl GraphNode for ExclusiveSystemContainer {
    type Label = SystemLabelId;

    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn labels(&self) -> &[SystemLabelId] {
        &self.labels
    }

    fn before(&self) -> &[SystemLabelId] {
        &self.before
    }

    fn after(&self) -> &[SystemLabelId] {
        &self.after
    }
}

impl SystemContainer for ExclusiveSystemContainer {
    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn run_criteria(&self) -> Option<usize> {
        self.run_criteria_index
    }

    fn set_run_criteria(&mut self, index: usize) {
        self.run_criteria_index = Some(index);
    }

    fn run_criteria_label(&self) -> Option<&RunCriteriaLabelId> {
        self.run_criteria_label.as_ref()
    }

    fn component_access(&self) -> Option<&Access<ComponentId>> {
        None
    }
}

pub struct ParallelSystemContainer {
    system: Box<dyn System<In = (), Out = ()>>,
    pub(crate) run_criteria_index: Option<usize>,
    pub(crate) run_criteria_label: Option<RunCriteriaLabelId>,
    pub(crate) should_run: bool,
    dependencies: Vec<usize>,
    labels: Vec<SystemLabelId>,
    before: Vec<SystemLabelId>,
    after: Vec<SystemLabelId>,
}

impl ParallelSystemContainer {
    pub(crate) fn from_descriptor(descriptor: ParallelSystemDescriptor) -> Self {
        ParallelSystemContainer {
            system: descriptor.system,
            should_run: false,
            run_criteria_index: None,
            run_criteria_label: None,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
        }
    }

    pub fn name(&self) -> Cow<'static, str> {
        GraphNode::name(self)
    }

    pub fn system(&self) -> &dyn System<In = (), Out = ()> {
        &*self.system
    }

    pub fn system_mut(&mut self) -> &mut dyn System<In = (), Out = ()> {
        &mut *self.system
    }

    pub fn should_run(&self) -> bool {
        self.should_run
    }

    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }
}

impl GraphNode for ParallelSystemContainer {
    type Label = SystemLabelId;

    fn name(&self) -> Cow<'static, str> {
        self.system().name()
    }

    fn labels(&self) -> &[SystemLabelId] {
        &self.labels
    }

    fn before(&self) -> &[SystemLabelId] {
        &self.before
    }

    fn after(&self) -> &[SystemLabelId] {
        &self.after
    }
}

impl SystemContainer for ParallelSystemContainer {
    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn run_criteria(&self) -> Option<usize> {
        self.run_criteria_index
    }

    fn set_run_criteria(&mut self, index: usize) {
        self.run_criteria_index = Some(index);
    }

    fn run_criteria_label(&self) -> Option<&RunCriteriaLabelId> {
        self.run_criteria_label.as_ref()
    }

    fn component_access(&self) -> Option<&Access<ComponentId>> {
        Some(self.system().component_access())
    }
}
