use crate::{BoxedSystem, ExclusiveSystem, ExclusiveSystemCoerced, ExclusiveSystemFn, System};

type Label = &'static str; // TODO

pub enum SystemDescriptor {
    Parallel(ParallelSystemDescriptor),
    Exclusive(ExclusiveSystemDescriptor),
}

impl From<ParallelSystemDescriptor> for SystemDescriptor {
    fn from(descriptor: ParallelSystemDescriptor) -> Self {
        SystemDescriptor::Parallel(descriptor)
    }
}

impl<S> From<S> for SystemDescriptor
where
    S: System<In = (), Out = ()>,
{
    fn from(system: S) -> Self {
        new_parallel_descriptor(Box::new(system)).into()
    }
}

impl From<BoxedSystem<(), ()>> for SystemDescriptor {
    fn from(system: BoxedSystem<(), ()>) -> Self {
        new_parallel_descriptor(system).into()
    }
}

impl From<ExclusiveSystemDescriptor> for SystemDescriptor {
    fn from(descriptor: ExclusiveSystemDescriptor) -> Self {
        SystemDescriptor::Exclusive(descriptor)
    }
}

impl From<ExclusiveSystemFn> for SystemDescriptor {
    fn from(system: ExclusiveSystemFn) -> Self {
        new_exclusive_descriptor(Box::new(system)).into()
    }
}

impl From<ExclusiveSystemCoerced> for SystemDescriptor {
    fn from(system: ExclusiveSystemCoerced) -> Self {
        new_exclusive_descriptor(Box::new(system)).into()
    }
}

pub struct ParallelSystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) label: Option<Label>,
    pub(crate) before: Vec<Label>,
    pub(crate) after: Vec<Label>,
}

fn new_parallel_descriptor(system: BoxedSystem<(), ()>) -> ParallelSystemDescriptor {
    ParallelSystemDescriptor {
        system,
        label: None,
        before: Vec::new(),
        after: Vec::new(),
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
        self.before.push(label);
        self
    }

    fn after(mut self, label: Label) -> ParallelSystemDescriptor {
        self.after.push(label);
        self
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum InsertionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}

pub struct ExclusiveSystemDescriptor {
    pub(crate) system: Box<dyn ExclusiveSystem>,
    pub(crate) label: Option<Label>,
    pub(crate) before: Vec<Label>,
    pub(crate) after: Vec<Label>,
    pub(crate) insertion_point: InsertionPoint,
}

fn new_exclusive_descriptor(system: Box<dyn ExclusiveSystem>) -> ExclusiveSystemDescriptor {
    ExclusiveSystemDescriptor {
        system,
        label: None,
        before: Vec::new(),
        after: Vec::new(),
        insertion_point: InsertionPoint::AtStart,
    }
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
        self.before.push(label);
        self
    }

    fn after(mut self, label: Label) -> ExclusiveSystemDescriptor {
        self.after.push(label);
        self
    }

    fn at_start(mut self) -> ExclusiveSystemDescriptor {
        self.insertion_point = InsertionPoint::AtStart;
        self
    }

    fn before_commands(mut self) -> ExclusiveSystemDescriptor {
        self.insertion_point = InsertionPoint::BeforeCommands;
        self
    }

    fn at_end(mut self) -> ExclusiveSystemDescriptor {
        self.insertion_point = InsertionPoint::AtEnd;
        self
    }
}

impl<T> ExclusiveSystemDescriptorCoercion for T
where
    T: ExclusiveSystem + 'static,
{
    fn label(self, label: Label) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).label(label)
    }

    fn before(self, label: Label) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).before(label)
    }

    fn after(self, label: Label) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).after(label)
    }

    fn at_start(self) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).at_start()
    }

    fn before_commands(self) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).before_commands()
    }

    fn at_end(self) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).at_end()
    }
}
