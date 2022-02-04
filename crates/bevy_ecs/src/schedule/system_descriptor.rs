use crate::{
    schedule::{
        AmbiguitySetLabel, BoxedAmbiguitySetLabel, BoxedSystemLabel, IntoRunCriteria,
        RunCriteriaDescriptorOrLabel, SystemLabel,
    },
    system::{BoxedSystem, ExclusiveSystem, ExclusiveSystemCoerced, ExclusiveSystemFn, IntoSystem},
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
/// Systems can have one or more labels attached to them; other systems in the same group
/// can then specify that they have to run before or after systems with that label using the
/// `before` and `after` methods.
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
///     .with_system(do_something.label(Something))
///     .with_system(do_the_other_thing.after(Something))
///     .with_system(do_something_else.exclusive_system().at_end());
/// ```
pub enum SystemDescriptor {
    Parallel(ParallelSystemDescriptor),
    Exclusive(ExclusiveSystemDescriptor),
}

pub trait IntoSystemDescriptor<Params> {
    fn into_descriptor(self) -> SystemDescriptor;
}

pub struct SystemLabelMarker;

impl IntoSystemDescriptor<()> for ParallelSystemDescriptor {
    fn into_descriptor(self) -> SystemDescriptor {
        SystemDescriptor::Parallel(self)
    }
}

impl<Params, S> IntoSystemDescriptor<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn into_descriptor(self) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(self.system())).into_descriptor()
    }
}

impl IntoSystemDescriptor<()> for SystemDescriptor {
    fn into_descriptor(self) -> SystemDescriptor {
        self
    }
}

impl IntoSystemDescriptor<()> for BoxedSystem<(), ()> {
    fn into_descriptor(self) -> SystemDescriptor {
        new_parallel_descriptor(self).into_descriptor()
    }
}

impl IntoSystemDescriptor<()> for ExclusiveSystemDescriptor {
    fn into_descriptor(self) -> SystemDescriptor {
        SystemDescriptor::Exclusive(self)
    }
}

impl<F> IntoSystemDescriptor<()> for ExclusiveSystemFn<F>
where
    F: FnMut(&mut crate::prelude::World) + Send + Sync + 'static,
{
    fn into_descriptor(self) -> SystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).into_descriptor()
    }
}

impl IntoSystemDescriptor<()> for ExclusiveSystemCoerced {
    fn into_descriptor(self) -> SystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).into_descriptor()
    }
}

/// Encapsulates a parallel system and information on when it runs in a `SystemStage`.
pub struct ParallelSystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
}

fn new_parallel_descriptor(system: BoxedSystem<(), ()>) -> ParallelSystemDescriptor {
    ParallelSystemDescriptor {
        system,
        run_criteria: None,
        labels: Vec::new(),
        before: Vec::new(),
        after: Vec::new(),
        ambiguity_sets: Vec::new(),
    }
}

pub trait ParallelSystemDescriptorCoercion<Params> {
    /// Assigns a run criteria to the system. Can be a new descriptor or a label of a
    /// run criteria defined elsewhere.
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ParallelSystemDescriptor;

    /// Assigns a label to the system; there can be more than one, and it doesn't have to be unique.
    fn label(self, label: impl SystemLabel) -> ParallelSystemDescriptor;

    /// Specifies that the system should run before systems with the given label.
    fn before(self, label: impl SystemLabel) -> ParallelSystemDescriptor;

    /// Specifies that the system should run after systems with the given label.
    fn after(self, label: impl SystemLabel) -> ParallelSystemDescriptor;

    /// Specifies that the system is exempt from execution order ambiguity detection
    /// with other systems in this set.
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ParallelSystemDescriptor;
}

impl ParallelSystemDescriptorCoercion<()> for ParallelSystemDescriptor {
    fn with_run_criteria<Marker>(
        mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ParallelSystemDescriptor {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    fn label(mut self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        self.labels.push(Box::new(label));
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

impl<S, Params> ParallelSystemDescriptorCoercion<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self.system())).with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self.system())).label(label)
    }

    fn before(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self.system())).before(label)
    }

    fn after(self, label: impl SystemLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self.system())).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ParallelSystemDescriptor {
        new_parallel_descriptor(Box::new(self.system())).in_ambiguity_set(set)
    }
}

impl ParallelSystemDescriptorCoercion<()> for BoxedSystem<(), ()> {
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ParallelSystemDescriptor {
        new_parallel_descriptor(self).with_run_criteria(run_criteria)
    }

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

/// Encapsulates an exclusive system and information on when it runs in a `SystemStage`.
pub struct ExclusiveSystemDescriptor {
    pub(crate) system: Box<dyn ExclusiveSystem>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<BoxedSystemLabel>,
    pub(crate) before: Vec<BoxedSystemLabel>,
    pub(crate) after: Vec<BoxedSystemLabel>,
    pub(crate) ambiguity_sets: Vec<BoxedAmbiguitySetLabel>,
    pub(crate) insertion_point: InsertionPoint,
}

fn new_exclusive_descriptor(system: Box<dyn ExclusiveSystem>) -> ExclusiveSystemDescriptor {
    ExclusiveSystemDescriptor {
        system,
        run_criteria: None,
        labels: Vec::new(),
        before: Vec::new(),
        after: Vec::new(),
        ambiguity_sets: Vec::new(),
        insertion_point: InsertionPoint::AtStart,
    }
}

pub trait ExclusiveSystemDescriptorCoercion {
    /// Assigns a run criteria to the system. Can be a new descriptor or a label of a
    /// run criteria defined elsewhere.
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ExclusiveSystemDescriptor;

    /// Assigns a label to the system; there can be more than one, and it doesn't have to be unique.
    fn label(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run before systems with the given label.
    fn before(self, label: impl SystemLabel) -> ExclusiveSystemDescriptor;

    /// Specifies that the system should run after systems with the given label.
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
    fn with_run_criteria<Marker>(
        mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ExclusiveSystemDescriptor {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    fn label(mut self, label: impl SystemLabel) -> ExclusiveSystemDescriptor {
        self.labels.push(Box::new(label));
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
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ExclusiveSystemDescriptor {
        new_exclusive_descriptor(Box::new(self)).with_run_criteria(run_criteria)
    }

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
