use crate::{
    component::ComponentId,
    query::Access,
    schedule::{
        AmbiguitySetLabelId, GraphNode, RunCriteriaLabelId, SystemDescriptor, SystemLabelId,
    },
    system::System,
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
    fn ambiguity_sets(&self) -> &[AmbiguitySetLabelId];
    fn component_access(&self) -> Option<&Access<ComponentId>>;
}

pub struct FunctionSystemContainer {
    system: Box<dyn System<In = (), Out = ()>>,
    pub(crate) run_criteria_index: Option<usize>,
    pub(crate) run_criteria_label: Option<RunCriteriaLabelId>,
    pub(crate) should_run: bool,
    dependencies: Vec<usize>,
    labels: Vec<SystemLabelId>,
    before: Vec<SystemLabelId>,
    after: Vec<SystemLabelId>,
    ambiguity_sets: Vec<AmbiguitySetLabelId>,
}

impl FunctionSystemContainer {
    pub(crate) fn from_descriptor(descriptor: SystemDescriptor) -> Self {
        FunctionSystemContainer {
            system: descriptor.system,
            should_run: false,
            run_criteria_index: None,
            run_criteria_label: None,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
            ambiguity_sets: descriptor.ambiguity_sets,
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

impl GraphNode for FunctionSystemContainer {
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

impl SystemContainer for FunctionSystemContainer {
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

    fn ambiguity_sets(&self) -> &[AmbiguitySetLabelId] {
        &self.ambiguity_sets
    }

    fn component_access(&self) -> Option<&Access<ComponentId>> {
        Some(self.system().component_access())
    }
}
