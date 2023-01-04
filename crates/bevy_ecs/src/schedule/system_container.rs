use crate::{
    component::ComponentId,
    query::Access,
    schedule::{
        AmbiguityDetection, GraphNode, RunCriteriaLabelId, SystemDescriptor, SystemLabelId,
    },
    system::System,
};
use core::fmt::Debug;
use std::borrow::Cow;

pub struct SystemContainer {
    system: Box<dyn System<In = (), Out = ()>>,
    pub(crate) run_criteria_index: Option<usize>,
    pub(crate) run_criteria_label: Option<RunCriteriaLabelId>,
    pub(crate) should_run: bool,
    is_exclusive: bool,
    dependencies: Vec<usize>,
    labels: Vec<SystemLabelId>,
    before: Vec<SystemLabelId>,
    after: Vec<SystemLabelId>,
    pub(crate) ambiguity_detection: AmbiguityDetection,
}

impl SystemContainer {
    pub(crate) fn from_descriptor(descriptor: SystemDescriptor) -> Self {
        SystemContainer {
            system: descriptor.system,
            should_run: false,
            run_criteria_index: None,
            run_criteria_label: None,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
            ambiguity_detection: descriptor.ambiguity_detection,
            is_exclusive: descriptor.exclusive_insertion_point.is_some(),
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

    pub fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    pub fn run_criteria(&self) -> Option<usize> {
        self.run_criteria_index
    }

    pub fn set_run_criteria(&mut self, index: usize) {
        self.run_criteria_index = Some(index);
    }

    pub fn run_criteria_label(&self) -> Option<&RunCriteriaLabelId> {
        self.run_criteria_label.as_ref()
    }

    pub fn component_access(&self) -> &Access<ComponentId> {
        self.system().component_access()
    }

    pub fn is_exclusive(&self) -> bool {
        self.is_exclusive
    }
}

impl Debug for SystemContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{:?}}}", &self.system())
    }
}

impl GraphNode for SystemContainer {
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
