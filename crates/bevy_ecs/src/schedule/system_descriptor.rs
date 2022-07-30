use crate::{
    schedule::{
        AmbiguitySetLabel, AmbiguitySetLabelId, IntoRunCriteria, RunCriteriaDescriptorOrLabel,
        SystemLabel, SystemLabelId,
    },
    system::{AsSystemLabel, BoxedSystem, IntoSystem},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum InsertionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SystemType {
    Parallel,
    Exclusive(InsertionPoint),
}

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
pub struct SystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<SystemLabelId>,
    pub(crate) before: Vec<SystemLabelId>,
    pub(crate) after: Vec<SystemLabelId>,
    pub(crate) ambiguity_sets: Vec<AmbiguitySetLabelId>,
    pub(crate) system_type: SystemType,
}

fn new_parallel_descriptor(system: BoxedSystem<(), ()>) -> SystemDescriptor {
    SystemDescriptor {
        labels: system.default_labels(),
        system,
        run_criteria: None,
        before: Vec::new(),
        after: Vec::new(),
        ambiguity_sets: Vec::new(),
        system_type: SystemType::Parallel,
    }
}

impl SystemDescriptor {
    pub fn with_run_criteria<Marker>(
        mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    pub fn label(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.labels.push(label.as_label());
        self
    }

    pub fn before<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        self.before.push(label.as_system_label().as_label());
        self
    }

    pub fn after<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        self.after.push(label.as_system_label().as_label());
        self
    }

    pub fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        self.ambiguity_sets.push(set.as_label());
        self
    }

    pub fn at_start(mut self) -> SystemDescriptor {
        self.system_type = SystemType::Exclusive(InsertionPoint::AtStart);
        self
    }

    pub fn before_commands(mut self) -> SystemDescriptor {
        self.system_type = SystemType::Exclusive(InsertionPoint::BeforeCommands);
        self
    }

    pub fn at_end(mut self) -> SystemDescriptor {
        self.system_type = SystemType::Exclusive(InsertionPoint::AtEnd);
        self
    }
}

pub trait IntoSystemDescriptor<Params> {
    fn into_descriptor(self) -> SystemDescriptor;

    /// Assigns a run criteria to the system. Can be a new descriptor or a label of a
    /// run criteria defined elsewhere.
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor;

    /// Assigns a label to the system; there can be more than one, and it doesn't have to be unique.
    fn label(self, label: impl SystemLabel) -> SystemDescriptor;

    /// Specifies that the system should run before systems with the given label.
    fn before<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor;

    /// Specifies that the system should run after systems with the given label.
    fn after<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor;

    /// Specifies that the system is exempt from execution order ambiguity detection
    /// with other systems in this set.
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor;

    /// Add a system that has a `&mut World` parameter (or coerce any system to be exclusive).
    ///
    /// Defaults to the "at start" insertion point.
    ///
    /// As of [#4166](https://github.com/bevyengine/bevy/pull/4166),
    /// this method is no longer required and will be removed in the future.
    // #[deprecated(since = "0.8.1", note = "`.exclusive_system()` is no longer needed")]
    fn exclusive_system(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the start of stage.
    fn at_start(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems after the parallel
    /// systems and before command buffer application.
    fn before_commands(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the end of stage.
    fn at_end(self) -> SystemDescriptor;
}

impl IntoSystemDescriptor<()> for SystemDescriptor {
    fn into_descriptor(self) -> SystemDescriptor {
        self
    }

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        self.with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        self.label(label)
    }

    fn before<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        self.before(label)
    }

    fn after<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        self.after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        self.in_ambiguity_set(set)
    }

    fn exclusive_system(self) -> SystemDescriptor {
        self.at_start()
    }

    fn at_start(self) -> SystemDescriptor {
        self.at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        self.before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        self.at_end()
    }
}

impl<S, Params> IntoSystemDescriptor<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn into_descriptor(self) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self)))
    }

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self)))
            .with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).label(label)
    }

    fn before<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).before(label)
    }

    fn after<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).in_ambiguity_set(set)
    }

    fn exclusive_system(self) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).at_start()
    }

    fn at_start(self) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        new_parallel_descriptor(Box::new(IntoSystem::into_system(self))).at_end()
    }
}

impl IntoSystemDescriptor<()> for BoxedSystem<(), ()> {
    fn into_descriptor(self) -> SystemDescriptor {
        new_parallel_descriptor(self)
    }

    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        new_parallel_descriptor(self).with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        new_parallel_descriptor(self).label(label)
    }

    fn before<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        new_parallel_descriptor(self).before(label)
    }

    fn after<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        new_parallel_descriptor(self).after(label)
    }

    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> SystemDescriptor {
        new_parallel_descriptor(self).in_ambiguity_set(set)
    }

    fn exclusive_system(self) -> SystemDescriptor {
        new_parallel_descriptor(self).at_start()
    }

    fn at_start(self) -> SystemDescriptor {
        new_parallel_descriptor(self).at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        new_parallel_descriptor(self).before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        new_parallel_descriptor(self).at_end()
    }
}
