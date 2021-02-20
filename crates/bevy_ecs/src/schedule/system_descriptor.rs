use crate::{
    AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystem, BoxedSystemLabel, ExclusiveSystem,
    ExclusiveSystemCoerced, ExclusiveSystemFn, System, SystemLabel,
};

/// Encapsulates a system and information on when it run in a `SystemStage`.
///
/// Systems can be inserted into 4 different groups within the stage:
/// * Parallel, accepts non-exclusive systems.
/// * At start, accepts exclusive systems; runs before parallel systems.
/// * Before commands, accepts exclusive systems; runs after parallel systems, but before their
/// command buffers are applied.
/// * At end, accepts exclusive systems; runs after parallel systems' command buffers have
/// been applied.
///
/// All systems can have a label attached to them; other systems in the same group can then specify
/// that they have to run before or after the system with that label using the `before` and `after` methods.
///
/// # Example
/// ```
/// # use bevy_ecs::prelude::*;
/// # fn do_something() {}
/// # fn do_the_other_thing() {}
/// # fn do_something_else() {}
/// #[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
/// struct Something;
///
/// SystemStage::parallel()
///     .with_system(do_something.system().label(Something))
///     .with_system(do_the_other_thing.system().after(Something))
///     .with_system(do_something_else.exclusive_system().at_end());
/// ```
pub enum SystemDescriptor {
    Parallel(ParallelSystemDescriptor),
    Exclusive(ExclusiveSystemDescriptor),
}

pub struct SystemLabelMarker;

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

/// Encapsulates a parallel system and information on when it run in a `SystemStage`.
pub struct ParallelSystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) label: Option<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

fn new_parallel_descriptor(system: BoxedSystem<(), ()>) -> ParallelSystemDescriptor {
    ParallelSystemDescriptor {
        system,
        label: None,
        before: Vec::new(),
        after: Vec::new(),
        ambiguity_sets: Vec::new(),
    }
}

pub trait ParallelSystemDescriptorCoercion {
    /// Assigns a label to the system.
    fn label(self, label: impl SystemLabel) -> ParallelSystemDescriptor;

    /// Specifies that the system should run before the system with given label.
    fn before(self, label: impl SystemLabel) -> ParallelSystemDescriptor;

    /// Specifies that the system should run after the system with given label.
    fn after(self, label: impl SystemLabel) -> ParallelSystemDescriptor;

    /// Specifies that the system is exempt from execution order ambiguity detection
    /// with other systems in this set.
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ParallelSystemDescriptor;
}

impl ParallelSystemDescriptorCoercion for ParallelSystemDescriptor {
    fn label(mut self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        self.label = Some(Box::new(label));
        self
    }

    fn before(mut self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        self.before.push(Box::new(label));
        self
    }

    fn after(mut self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        self.after.push(Box::new(label));
        self
    }

    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> ParallelSystemDescriptor {
        self.ambiguity_sets.push(Box::new(set));
        self
    }
}

impl<S> ParallelSystemDescriptorCoercion for S
where
    S: System<In = (), Out = ()>,
{
    fn label(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).label(label)
    }

    fn before(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).before(label)
    }

    fn after(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self)).in_ambiguity_set(set)
    }
}

impl ParallelSystemDescriptorCoercion for BoxedSystem<(), ()> {
    fn label(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).label(label)
    }

    fn before(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).before(label)
    }

    fn after(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).in_ambiguity_set(set)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum InsertionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}

/// Encapsulates an exclusive system and information on when it run in a `SystemStage`.
pub struct ExclusiveSystemDescriptor {
    pub(crate) system: Box<dyn ExclusiveSystem>,
    pub(crate) label: Option<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    pub(crate) insertion_point: InsertionPoint,
}

fn new_exclusive_descriptor(system: Box<dyn ExclusiveSystem>) -> ExclusiveSystemDescriptor {
    ExclusiveSystemDescriptor {
        system,
        label: None,
        before: Vec::new(),
        after: Vec::new(),
        ambiguity_sets: Vec::new(),
        insertion_point: InsertionPoint::AtStart,
    }
}

pub trait ExclusiveSystemDescriptorCoercion {
    /// Assigns a label to the system.
    fn label(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run before the system with given label.
    fn before(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run after the system with given label.
    fn after(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor;

    /// Specifies that the system is exempt from execution order ambiguity detection
    /// with other systems in this set.
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the start of stage.
    fn at_start(self) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run with other exclusive systems after the parallel
    /// systems and before command buffer application.
    fn before_commands(self) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the end of stage.
    fn at_end(self) -> ExclusiveSystemDescriptor;
}

impl ExclusiveSystemDescriptorCoercion for ExclusiveSystemDescriptor {
    fn label(mut self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        self.label = Some(Box::new(label));
        self
    }

    fn before(mut self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        self.before.push(Box::new(label));
        self
    }

    fn after(mut self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        self.after.push(Box::new(label));
        self
    }

    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> ExclusiveSystemDescriptor {
        self.ambiguity_sets.push(Box::new(set));
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
    fn label(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).label(label)
    }

    fn before(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).before(label)
    }

    fn after(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).in_ambiguity_set(set)
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
