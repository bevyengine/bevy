use std::ptr::NonNull;

use crate::{BoxedSystem, System};

type Label = &'static str; // TODO

pub enum SystemDescriptor {
    Parallel(ParallelSystemDescriptor),
    Sequential(SequentialSystemDescriptor),
}

impl From<BoxedSystem<(), ()>> for SystemDescriptor {
    fn from(system: BoxedSystem<(), ()>) -> Self {
        if system.archetype_component_access().writes_all() || system.resource_access().writes_all()
        {
            return SystemDescriptor::Sequential(new_sequential_descriptor(system));
        }
        SystemDescriptor::Parallel(new_parallel_descriptor(system))
    }
}

impl<S> From<S> for SystemDescriptor
where
    S: System<In = (), Out = ()>,
{
    fn from(system: S) -> Self {
        <SystemDescriptor as From<BoxedSystem<(), ()>>>::from(Box::new(system))
    }
}

impl From<UnspecifiedSystemDescriptor> for SystemDescriptor {
    fn from(descriptor: UnspecifiedSystemDescriptor) -> Self {
        if descriptor.system.archetype_component_access().writes_all()
            || descriptor.system.resource_access().writes_all()
        {
            return SystemDescriptor::Sequential(
                new_sequential_descriptor(descriptor.system).label(descriptor.label),
            );
        }
        SystemDescriptor::Parallel(
            new_parallel_descriptor(descriptor.system).label(descriptor.label),
        )
    }
}

impl From<ParallelSystemDescriptor> for SystemDescriptor {
    fn from(descriptor: ParallelSystemDescriptor) -> Self {
        SystemDescriptor::Parallel(descriptor)
    }
}

impl From<SequentialSystemDescriptor> for SystemDescriptor {
    fn from(descriptor: SequentialSystemDescriptor) -> Self {
        SystemDescriptor::Sequential(descriptor)
    }
}

pub struct UnspecifiedSystemDescriptor {
    system: BoxedSystem<(), ()>,
    label: Label,
}

pub trait UnspecifiedSystemDescriptorCoercion {
    fn label(self, label: Label) -> UnspecifiedSystemDescriptor;
}

impl UnspecifiedSystemDescriptorCoercion for UnspecifiedSystemDescriptor {
    fn label(mut self, label: Label) -> UnspecifiedSystemDescriptor {
        self.label = label;
        self
    }
}

impl UnspecifiedSystemDescriptorCoercion for BoxedSystem<(), ()> {
    fn label(self, label: Label) -> UnspecifiedSystemDescriptor {
        UnspecifiedSystemDescriptor {
            system: self,
            label,
        }
    }
}

impl<S> UnspecifiedSystemDescriptorCoercion for S
where
    S: System<In = (), Out = ()>,
{
    fn label(self, label: Label) -> UnspecifiedSystemDescriptor {
        UnspecifiedSystemDescriptor {
            system: Box::new(self),
            label,
        }
    }
}

pub struct ParallelSystemDescriptor {
    system: NonNull<dyn System<In = (), Out = ()>>,
    pub(crate) label: Option<Label>,
    // TODO consider Vec<Option<Label>> or something to support optional dependencies?
    pub(crate) dependencies: Vec<Label>,
}

unsafe impl Send for ParallelSystemDescriptor {}
unsafe impl Sync for ParallelSystemDescriptor {}

impl ParallelSystemDescriptor {
    pub(crate) fn system(&self) -> &dyn System<In = (), Out = ()> {
        // SAFE: statically enforced shared access.
        unsafe { self.system.as_ref() }
    }

    pub(crate) fn system_mut(&mut self) -> &mut dyn System<In = (), Out = ()> {
        // SAFE: statically enforced exclusive access.
        unsafe { self.system.as_mut() }
    }

    /// # Safety
    /// Ensure no other borrows exist along with this one.
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn system_mut_unsafe(&self) -> &mut dyn System<In = (), Out = ()> {
        &mut *self.system.as_ptr()
    }

    pub fn label(mut self, label: Label) -> ParallelSystemDescriptor {
        self.label = Some(label);
        self
    }
}

pub trait ParallelSystemDescriptorCoercion {
    fn with_dependency(self, dependency: Label) -> ParallelSystemDescriptor;
}

impl ParallelSystemDescriptorCoercion for ParallelSystemDescriptor {
    fn with_dependency(mut self, dependency: Label) -> ParallelSystemDescriptor {
        self.dependencies.push(dependency);
        self
    }
}

fn new_parallel_descriptor(system: BoxedSystem<(), ()>) -> ParallelSystemDescriptor {
    if system.archetype_component_access().writes_all() || system.resource_access().writes_all() {
        todo!("some error message that makes sense");
    }
    ParallelSystemDescriptor {
        system: unsafe { NonNull::new_unchecked(Box::into_raw(system)) },
        label: None,
        dependencies: Vec::new(),
    }
}

impl ParallelSystemDescriptorCoercion for UnspecifiedSystemDescriptor {
    fn with_dependency(self, dependency: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self.system)
            .label(self.label)
            .with_dependency(dependency)
    }
}

impl<S> ParallelSystemDescriptorCoercion for S
where
    S: System<In = (), Out = ()>,
{
    fn with_dependency(self, dependency: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).with_dependency(dependency)
    }
}

impl ParallelSystemDescriptorCoercion for BoxedSystem<(), ()> {
    fn with_dependency(self, dependency: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).with_dependency(dependency)
    }
}

pub(crate) enum Ordering {
    None,
    Before(Label),
    After(Label),
}

pub(crate) enum InjectionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}

pub struct SequentialSystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) label: Option<Label>,
    pub(crate) ordering: Ordering,
    pub(crate) injection_point: InjectionPoint,
}

impl SequentialSystemDescriptor {
    pub fn label(mut self, label: Label) -> SequentialSystemDescriptor {
        self.label = Some(label);
        self
    }
}

pub trait SequentialSystemDescriptorCoercion {
    fn before(self, label: Label) -> SequentialSystemDescriptor;

    fn after(self, label: Label) -> SequentialSystemDescriptor;

    fn at_start(self) -> SequentialSystemDescriptor;

    fn before_commands(self) -> SequentialSystemDescriptor;

    fn at_end(self) -> SequentialSystemDescriptor;
}

impl SequentialSystemDescriptorCoercion for SequentialSystemDescriptor {
    fn before(mut self, label: Label) -> SequentialSystemDescriptor {
        self.ordering = Ordering::Before(label);
        self
    }

    fn after(mut self, label: Label) -> SequentialSystemDescriptor {
        self.ordering = Ordering::After(label);
        self
    }

    fn at_start(mut self) -> SequentialSystemDescriptor {
        self.injection_point = InjectionPoint::AtStart;
        self
    }

    fn before_commands(mut self) -> SequentialSystemDescriptor {
        self.injection_point = InjectionPoint::BeforeCommands;
        self
    }

    fn at_end(mut self) -> SequentialSystemDescriptor {
        self.injection_point = InjectionPoint::AtEnd;
        self
    }
}

fn new_sequential_descriptor(system: BoxedSystem<(), ()>) -> SequentialSystemDescriptor {
    SequentialSystemDescriptor {
        system,
        label: None,
        ordering: Ordering::None,
        injection_point: InjectionPoint::AtStart,
    }
}

impl SequentialSystemDescriptorCoercion for UnspecifiedSystemDescriptor {
    fn before(self, label: Label) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self.system)
            .label(self.label)
            .before(label)
    }

    fn after(self, label: Label) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self.system)
            .label(self.label)
            .after(label)
    }

    fn at_start(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self.system)
            .label(self.label)
            .at_start()
    }

    fn before_commands(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self.system)
            .label(self.label)
            .before_commands()
    }

    fn at_end(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self.system)
            .label(self.label)
            .at_end()
    }
}

impl<S> SequentialSystemDescriptorCoercion for S
where
    S: System<In = (), Out = ()>,
{
    fn before(self, label: Label) -> SequentialSystemDescriptor {
        new_sequential_descriptor(Box::new(self)).before(label)
    }

    fn after(self, label: Label) -> SequentialSystemDescriptor {
        new_sequential_descriptor(Box::new(self)).after(label)
    }

    fn at_start(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(Box::new(self)).at_start()
    }

    fn before_commands(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(Box::new(self)).before_commands()
    }

    fn at_end(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(Box::new(self)).at_end()
    }
}

impl SequentialSystemDescriptorCoercion for BoxedSystem<(), ()> {
    fn before(self, label: Label) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self).before(label)
    }

    fn after(self, label: Label) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self).after(label)
    }

    fn at_start(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self).at_start()
    }

    fn before_commands(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self).before_commands()
    }

    fn at_end(self) -> SequentialSystemDescriptor {
        new_sequential_descriptor(self).at_end()
    }
}
