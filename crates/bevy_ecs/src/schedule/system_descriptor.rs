use std::ptr::NonNull;

use crate::{BoxedSystem, ExclusiveSystemFn, System};

type Label = &'static str; // TODO

pub struct ParallelSystemDescriptor {
    system: NonNull<dyn System<In = (), Out = ()>>,
    pub(crate) label: Option<Label>,
    // TODO consider Vec<Option<Label>> or something to support optional dependencies?
    pub(crate) dependencies: Vec<Label>,
    pub(crate) dependants: Vec<Label>,
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
}

pub trait ParallelSystemDescriptorCoercion {
    fn label(self, label: Label) -> ParallelSystemDescriptor;

    fn before(self, label: Label) -> ParallelSystemDescriptor;

    fn after(self, label: Label) -> ParallelSystemDescriptor;
}

impl ParallelSystemDescriptorCoercion for ParallelSystemDescriptor {
    fn label(mut self, label: Label) -> ParallelSystemDescriptor {
        self.label = Some(label);
        self
    }

    fn before(mut self, label: Label) -> ParallelSystemDescriptor {
        self.dependants.push(label);
        self
    }

    fn after(mut self, label: Label) -> ParallelSystemDescriptor {
        self.dependencies.push(label);
        self
    }
}

fn new_parallel_descriptor(system: BoxedSystem<(), ()>) -> ParallelSystemDescriptor {
    ParallelSystemDescriptor {
        system: unsafe { NonNull::new_unchecked(Box::into_raw(system)) },
        label: None,
        dependencies: Vec::new(),
        dependants: Vec::new(),
    }
}

impl<S> ParallelSystemDescriptorCoercion for S
where
    S: System<In = (), Out = ()>,
{
    fn label(self, label: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).label(label)
    }

    fn before(self, label: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).before(label)
    }

    fn after(self, label: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).after(label)
    }
}

impl ParallelSystemDescriptorCoercion for BoxedSystem<(), ()> {
    fn label(self, label: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).label(label)
    }

    fn before(self, label: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).before(label)
    }

    fn after(self, label: Label) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).after(label)
    }
}

impl<S> From<S> for ParallelSystemDescriptor
where
    S: System<In = (), Out = ()>,
{
    fn from(system: S) -> Self {
        new_parallel_descriptor(Box::new(system))
    }
}

impl From<BoxedSystem<(), ()>> for ParallelSystemDescriptor {
    fn from(system: BoxedSystem<(), ()>) -> Self {
        new_parallel_descriptor(system)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Ordering {
    None,
    Before(Label),
    After(Label),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum InjectionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}

pub struct ExclusiveSystemDescriptor {
    pub(crate) system: ExclusiveSystemFn,
    pub(crate) label: Option<Label>,
    pub(crate) ordering: Ordering,
    pub(crate) injection_point: InjectionPoint,
}

pub trait ExclusiveSystemDescriptorCoercion {
    fn label(self, label: Label) -> ExclusiveSystemDescriptor;

    fn before(self, label: Label) -> ExclusiveSystemDescriptor;

    fn after(self, label: Label) -> ExclusiveSystemDescriptor;

    fn at_start(self) -> ExclusiveSystemDescriptor;

    fn before_commands(self) -> ExclusiveSystemDescriptor;

    fn at_end(self) -> ExclusiveSystemDescriptor;
}

impl ExclusiveSystemDescriptorCoercion for ExclusiveSystemDescriptor {
    fn label(mut self, label: Label) -> ExclusiveSystemDescriptor {
        self.label = Some(label);
        self
    }

    fn before(mut self, label: Label) -> ExclusiveSystemDescriptor {
        self.ordering = Ordering::Before(label);
        self
    }

    fn after(mut self, label: Label) -> ExclusiveSystemDescriptor {
        self.ordering = Ordering::After(label);
        self
    }

    fn at_start(mut self) -> ExclusiveSystemDescriptor {
        self.injection_point = InjectionPoint::AtStart;
        self
    }

    fn before_commands(mut self) -> ExclusiveSystemDescriptor {
        self.injection_point = InjectionPoint::BeforeCommands;
        self
    }

    fn at_end(mut self) -> ExclusiveSystemDescriptor {
        self.injection_point = InjectionPoint::AtEnd;
        self
    }
}

fn new_exclusive_descriptor(system: ExclusiveSystemFn) -> ExclusiveSystemDescriptor {
    ExclusiveSystemDescriptor {
        system,
        label: None,
        ordering: Ordering::None,
        injection_point: InjectionPoint::AtStart,
    }
}

impl<T> ExclusiveSystemDescriptorCoercion for T
where
    T: Into<ExclusiveSystemFn>,
{
    fn label(self, label: Label) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(self.into()).label(label)
    }

    fn before(self, label: Label) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(self.into()).before(label)
    }

    fn after(self, label: Label) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(self.into()).after(label)
    }

    fn at_start(self) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(self.into()).at_start()
    }

    fn before_commands(self) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(self.into()).before_commands()
    }

    fn at_end(self) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(self.into()).at_end()
    }
}

impl From<ExclusiveSystemFn> for ExclusiveSystemDescriptor {
    fn from(system: ExclusiveSystemFn) -> Self {
        new_exclusive_descriptor(system)
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, Stage};

    fn make_exclusive(tag: usize) -> impl FnMut(&mut Resources) {
        move |resources| resources.get_mut::<Vec<usize>>().unwrap().push(tag)
    }

    // This is silly. https://github.com/bevyengine/bevy/issues/1029
    macro_rules! make_parallel {
        ($tag:expr) => {{
            fn parallel(mut resource: ResMut<Vec<usize>>) {
                resource.push($tag)
            }
            parallel
        }};
    }

    #[test]
    fn basic_order() {
        let mut world = World::new();
        let mut resources = Resources::default();

        resources.insert(Vec::<usize>::new());
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(0).system().at_start())
            .with_system(make_parallel!(1).system())
            .with_exclusive_system(make_exclusive(2).system().before_commands())
            .with_exclusive_system(make_exclusive(3).system().at_end());
        stage.run(&mut world, &mut resources);
        assert_eq!(*resources.get::<Vec<usize>>().unwrap(), vec![0, 1, 2, 3]);

        resources.get_mut::<Vec<usize>>().unwrap().clear();
        let mut stage = SystemStage::parallel()
            .with_exclusive_system(make_exclusive(2).system().before_commands())
            .with_exclusive_system(make_exclusive(3).system().at_end())
            .with_system(make_parallel!(1).system())
            .with_exclusive_system(make_exclusive(0).system().at_start());
        stage.run(&mut world, &mut resources);
        assert_eq!(*resources.get::<Vec<usize>>().unwrap(), vec![0, 1, 2, 3]);
    }
}
