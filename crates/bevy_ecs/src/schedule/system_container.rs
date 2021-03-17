use crate::{
    component::ComponentId,
    query::Access,
    schedule::{
        BoxedAmbiguitySetLabel, BoxedRunCriterionLabel, BoxedSystemLabel,
        ExclusiveSystemDescriptor, ParallelSystemDescriptor,
    },
    system::{ExclusiveSystem, System},
};
use std::{borrow::Cow, ptr::NonNull};

/// System metadata like its name, labels, order requirements and component access.
pub trait SystemContainer {
    fn name(&self) -> Cow<'static, str>;
    #[doc(hidden)]
    fn dependencies(&self) -> &[usize];
    #[doc(hidden)]
    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>);
    #[doc(hidden)]
    fn run_criterion(&self) -> Option<usize>;
    #[doc(hidden)]
    fn set_run_criterion(&mut self, criterion: usize);
    fn run_criterion_label(&self) -> Option<&BoxedRunCriterionLabel>;
    fn labels(&self) -> &[BoxedSystemLabel];
    fn before(&self) -> &[BoxedSystemLabel];
    fn after(&self) -> &[BoxedSystemLabel];
    fn ambiguity_sets(&self) -> &[BoxedAmbiguitySetLabel];
    fn component_access(&self) -> Option<&Access<ComponentId>>;
}

pub(super) struct ExclusiveSystemContainer {
    system: Box<dyn ExclusiveSystem>,
    run_criterion: Option<usize>,
    run_criterion_label: Option<BoxedRunCriterionLabel>,
    dependencies: Vec<usize>,
    labels: Vec<BoxedSystemLabel>,
    before: Vec<BoxedSystemLabel>,
    after: Vec<BoxedSystemLabel>,
    ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

impl ExclusiveSystemContainer {
    pub fn from_descriptor(
        descriptor: ExclusiveSystemDescriptor,
        run_criterion_label: Option<BoxedRunCriterionLabel>,
    ) -> Self {
        ExclusiveSystemContainer {
            system: descriptor.system,
            run_criterion: None,
            run_criterion_label,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
            ambiguity_sets: descriptor.ambiguity_sets,
        }
    }

    pub fn system_mut(&mut self) -> &mut Box<dyn ExclusiveSystem> {
        &mut self.system
    }
}

impl SystemContainer for ExclusiveSystemContainer {
    fn name(&self) -> Cow<'static, str> {
        self.system.name()
    }

    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn run_criterion(&self) -> Option<usize> {
        self.run_criterion
    }

    fn set_run_criterion(&mut self, criterion: usize) {
        self.run_criterion = Some(criterion);
    }

    fn run_criterion_label(&self) -> Option<&BoxedRunCriterionLabel> {
        self.run_criterion_label.as_ref()
    }

    fn labels(&self) -> &[BoxedSystemLabel] {
        &self.labels
    }

    fn before(&self) -> &[BoxedSystemLabel] {
        &self.before
    }

    fn after(&self) -> &[BoxedSystemLabel] {
        &self.after
    }

    fn ambiguity_sets(&self) -> &[BoxedAmbiguitySetLabel] {
        &self.ambiguity_sets
    }

    fn component_access(&self) -> Option<&Access<ComponentId>> {
        None
    }
}

pub struct ParallelSystemContainer {
    system: NonNull<dyn System<In = (), Out = ()>>,
    run_criterion: Option<usize>,
    run_criterion_label: Option<BoxedRunCriterionLabel>,
    pub(crate) should_run: bool,
    dependencies: Vec<usize>,
    labels: Vec<BoxedSystemLabel>,
    before: Vec<BoxedSystemLabel>,
    after: Vec<BoxedSystemLabel>,
    ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

unsafe impl Send for ParallelSystemContainer {}
unsafe impl Sync for ParallelSystemContainer {}

impl ParallelSystemContainer {
    pub(crate) fn from_descriptor(
        descriptor: ParallelSystemDescriptor,
        run_criterion_label: Option<BoxedRunCriterionLabel>,
    ) -> Self {
        ParallelSystemContainer {
            system: unsafe { NonNull::new_unchecked(Box::into_raw(descriptor.system)) },
            should_run: false,
            run_criterion: None,
            run_criterion_label,
            dependencies: Vec::new(),
            labels: descriptor.labels,
            before: descriptor.before,
            after: descriptor.after,
            ambiguity_sets: descriptor.ambiguity_sets,
        }
    }

    pub fn name(&self) -> Cow<'static, str> {
        SystemContainer::name(self)
    }

    pub fn system(&self) -> &dyn System<In = (), Out = ()> {
        // SAFE: statically enforced shared access.
        unsafe { self.system.as_ref() }
    }

    pub fn system_mut(&mut self) -> &mut dyn System<In = (), Out = ()> {
        // SAFE: statically enforced exclusive access.
        unsafe { self.system.as_mut() }
    }

    /// # Safety
    /// Ensure no other borrows exist along with this one.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn system_mut_unsafe(&self) -> &mut dyn System<In = (), Out = ()> {
        &mut *self.system.as_ptr()
    }

    pub fn should_run(&self) -> bool {
        self.should_run
    }

    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }
}

impl SystemContainer for ParallelSystemContainer {
    fn name(&self) -> Cow<'static, str> {
        self.system().name()
    }

    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn run_criterion(&self) -> Option<usize> {
        self.run_criterion
    }

    fn set_run_criterion(&mut self, criterion: usize) {
        self.run_criterion = Some(criterion);
    }

    fn run_criterion_label(&self) -> Option<&BoxedRunCriterionLabel> {
        self.run_criterion_label.as_ref()
    }

    fn labels(&self) -> &[BoxedSystemLabel] {
        &self.labels
    }

    fn before(&self) -> &[BoxedSystemLabel] {
        &self.before
    }

    fn after(&self) -> &[BoxedSystemLabel] {
        &self.after
    }

    fn ambiguity_sets(&self) -> &[BoxedAmbiguitySetLabel] {
        &self.ambiguity_sets
    }

    fn component_access(&self) -> Option<&Access<ComponentId>> {
        Some(self.system().component_access())
    }
}
