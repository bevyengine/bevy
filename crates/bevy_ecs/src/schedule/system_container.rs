use std::{borrow::Cow, ptr::NonNull};

use crate::{
    BoxedSystemLabel, ExclusiveSystem, ExclusiveSystemDescriptor, ParallelSystemDescriptor, System,
};

pub(super) trait SystemContainer {
    fn display_name(&self) -> Cow<'static, str>;
    fn dependencies(&self) -> &[usize];
    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>);
    fn system_set(&self) -> usize;
    fn label(&self) -> &Option<BoxedSystemLabel>;
    fn before(&self) -> &[BoxedSystemLabel];
    fn after(&self) -> &[BoxedSystemLabel];
    fn is_compatible(&self, other: &Self) -> bool;
}

pub(super) struct ExclusiveSystemContainer {
    system: Box<dyn ExclusiveSystem>,
    dependencies: Vec<usize>,
    set: usize,
    label: Option<BoxedSystemLabel>,
    before: Vec<BoxedSystemLabel>,
    after: Vec<BoxedSystemLabel>,
}

impl ExclusiveSystemContainer {
    pub fn from_descriptor(descriptor: ExclusiveSystemDescriptor, set: usize) -> Self {
        ExclusiveSystemContainer {
            system: descriptor.system,
            dependencies: Vec::new(),
            set,
            label: descriptor.label,
            before: descriptor.before,
            after: descriptor.after,
        }
    }

    pub fn system_mut(&mut self) -> &mut Box<dyn ExclusiveSystem> {
        &mut self.system
    }
}

impl SystemContainer for ExclusiveSystemContainer {
    fn display_name(&self) -> Cow<'static, str> {
        self.label
            .as_ref()
            .map(|l| Cow::Owned(format!("{:?}", l)))
            .unwrap_or_else(|| self.system.name())
    }

    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn system_set(&self) -> usize {
        self.set
    }

    fn label(&self) -> &Option<BoxedSystemLabel> {
        &self.label
    }

    fn before(&self) -> &[BoxedSystemLabel] {
        &self.before
    }

    fn after(&self) -> &[BoxedSystemLabel] {
        &self.after
    }

    fn is_compatible(&self, _: &Self) -> bool {
        false
    }
}

pub struct ParallelSystemContainer {
    system: NonNull<dyn System<In = (), Out = ()>>,
    pub(crate) should_run: bool,
    dependencies: Vec<usize>,
    set: usize,
    label: Option<BoxedSystemLabel>,
    before: Vec<BoxedSystemLabel>,
    after: Vec<BoxedSystemLabel>,
}

impl SystemContainer for ParallelSystemContainer {
    fn display_name(&self) -> Cow<'static, str> {
        self.label
            .as_ref()
            .map(|l| Cow::Owned(format!("{:?}", l)))
            .unwrap_or_else(|| self.system().name())
    }

    fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }

    fn set_dependencies(&mut self, dependencies: impl IntoIterator<Item = usize>) {
        self.dependencies.clear();
        self.dependencies.extend(dependencies);
    }

    fn system_set(&self) -> usize {
        self.set
    }

    fn label(&self) -> &Option<BoxedSystemLabel> {
        &self.label
    }

    fn before(&self) -> &[BoxedSystemLabel] {
        &self.before
    }

    fn after(&self) -> &[BoxedSystemLabel] {
        &self.after
    }

    fn is_compatible(&self, other: &Self) -> bool {
        self.system()
            .component_access()
            .is_compatible(other.system().component_access())
            && self
                .system()
                .resource_access()
                .is_compatible(other.system().resource_access())
    }
}

unsafe impl Send for ParallelSystemContainer {}
unsafe impl Sync for ParallelSystemContainer {}

impl ParallelSystemContainer {
    pub(crate) fn from_descriptor(descriptor: ParallelSystemDescriptor, set: usize) -> Self {
        ParallelSystemContainer {
            system: unsafe { NonNull::new_unchecked(Box::into_raw(descriptor.system)) },
            should_run: false,
            set,
            dependencies: Vec::new(),
            label: descriptor.label,
            before: descriptor.before,
            after: descriptor.after,
        }
    }

    pub fn display_name(&self) -> Cow<'static, str> {
        SystemContainer::display_name(self)
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
