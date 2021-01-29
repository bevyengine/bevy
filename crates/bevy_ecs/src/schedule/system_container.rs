use std::{borrow::Cow, ptr::NonNull};

use crate::{ExclusiveSystem, System};

pub(super) trait SystemContainer {
    fn display_name(&self) -> Cow<'static, str>;
    fn set(&self) -> usize;
    fn label(&self) -> &Option<Cow<'static, str>>;
    fn before(&self) -> &[Cow<'static, str>];
    fn after(&self) -> &[Cow<'static, str>];
}

pub(super) struct ExclusiveSystemContainer {
    pub system: Box<dyn ExclusiveSystem>,
    pub set: usize,
    pub label: Option<Cow<'static, str>>,
    pub before: Vec<Cow<'static, str>>,
    pub after: Vec<Cow<'static, str>>,
}

impl SystemContainer for ExclusiveSystemContainer {
    fn display_name(&self) -> Cow<'static, str> {
        self.label
            .as_ref()
            .cloned()
            .map(|label| label.into())
            .unwrap_or_else(|| self.system.name())
    }

    fn set(&self) -> usize {
        self.set
    }

    fn label(&self) -> &Option<Cow<'static, str>> {
        &self.label
    }

    fn before(&self) -> &[Cow<'static, str>] {
        &self.before
    }

    fn after(&self) -> &[Cow<'static, str>] {
        &self.after
    }
}

pub struct ParallelSystemContainer {
    pub(super) system: NonNull<dyn System<In = (), Out = ()>>,
    pub(super) should_run: bool,
    pub(super) dependencies: Vec<usize>,
    pub(super) set: usize,
    pub(super) label: Option<Cow<'static, str>>,
    pub(super) before: Vec<Cow<'static, str>>,
    pub(super) after: Vec<Cow<'static, str>>,
}

impl SystemContainer for ParallelSystemContainer {
    fn display_name(&self) -> Cow<'static, str> {
        self.label
            .as_ref()
            .cloned()
            .map(|label| label.into())
            .unwrap_or_else(|| self.system().name())
    }

    fn set(&self) -> usize {
        self.set
    }

    fn label(&self) -> &Option<Cow<'static, str>> {
        &self.label
    }

    fn before(&self) -> &[Cow<'static, str>] {
        &self.before
    }

    fn after(&self) -> &[Cow<'static, str>] {
        &self.after
    }
}

unsafe impl Send for ParallelSystemContainer {}
unsafe impl Sync for ParallelSystemContainer {}

impl ParallelSystemContainer {
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
