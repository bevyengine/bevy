use crate::{
    schedule::{IntoRunCriteria, RunCriteriaDescriptorOrLabel, SystemLabel, SystemLabelId},
    system::{AsSystemLabel, BoxedSystem, IntoSystem},
};

/// Configures ambiguity detection for a single system.
#[derive(Debug, Default)]
pub(crate) enum AmbiguityDetection {
    #[default]
    Check,
    IgnoreAll,
    /// Ignore systems with any of these labels.
    IgnoreWithLabel(Vec<SystemLabelId>),
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
///     .with_system(do_something_else.at_end());
/// ```
#[derive(Debug)]
pub struct SystemDescriptor {
    pub(crate) system: BoxedSystem<(), ()>,
    pub(crate) exclusive_insertion_point: Option<ExclusiveInsertionPoint>,
    pub(crate) run_criteria: Option<RunCriteriaDescriptorOrLabel>,
    pub(crate) labels: Vec<SystemLabelId>,
    pub(crate) before: Vec<SystemLabelId>,
    pub(crate) after: Vec<SystemLabelId>,
    pub(crate) ambiguity_detection: AmbiguityDetection,
}

impl SystemDescriptor {
    fn new(system: BoxedSystem<(), ()>) -> SystemDescriptor {
        SystemDescriptor {
            labels: system.default_labels(),
            exclusive_insertion_point: if system.is_exclusive() {
                Some(ExclusiveInsertionPoint::AtStart)
            } else {
                None
            },
            system,
            run_criteria: None,
            before: Vec::new(),
            after: Vec::new(),
            ambiguity_detection: Default::default(),
        }
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

    /// Marks this system as ambiguous with any system with the specified label.
    /// This means that execution order between these systems does not matter,
    /// which allows [some warnings](crate::schedule::ReportExecutionOrderAmbiguities) to be silenced.
    fn ambiguous_with<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor;

    /// Specifies that this system should opt out of
    /// [execution order ambiguity detection](crate::schedule::ReportExecutionOrderAmbiguities).
    fn ignore_all_ambiguities(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the start of stage.
    fn at_start(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems after the parallel
    /// systems and before command buffer application.
    fn before_commands(self) -> SystemDescriptor;

    /// Specifies that the system should run with other exclusive systems at the end of stage.
    fn at_end(self) -> SystemDescriptor;
}

impl IntoSystemDescriptor<()> for SystemDescriptor {
    fn with_run_criteria<Marker>(
        mut self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        self.run_criteria = Some(run_criteria.into());
        self
    }

    fn label(mut self, label: impl SystemLabel) -> SystemDescriptor {
        self.labels.push(label.as_label());
        self
    }

    fn before<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        self.before.push(label.as_system_label().as_label());
        self
    }

    fn after<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        self.after.push(label.as_system_label().as_label());
        self
    }

    fn ambiguous_with<Marker>(mut self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        match &mut self.ambiguity_detection {
            detection @ AmbiguityDetection::Check => {
                *detection =
                    AmbiguityDetection::IgnoreWithLabel(vec![label.as_system_label().as_label()]);
            }
            AmbiguityDetection::IgnoreWithLabel(labels) => {
                labels.push(label.as_system_label().as_label());
            }
            // This descriptor is already ambiguous with everything.
            AmbiguityDetection::IgnoreAll => {}
        }
        self
    }

    fn ignore_all_ambiguities(mut self) -> SystemDescriptor {
        self.ambiguity_detection = AmbiguityDetection::IgnoreAll;
        self
    }

    fn at_start(mut self) -> SystemDescriptor {
        self.exclusive_insertion_point = Some(ExclusiveInsertionPoint::AtStart);
        self
    }

    fn before_commands(mut self) -> SystemDescriptor {
        self.exclusive_insertion_point = Some(ExclusiveInsertionPoint::BeforeCommands);
        self
    }

    fn at_end(mut self) -> SystemDescriptor {
        self.exclusive_insertion_point = Some(ExclusiveInsertionPoint::AtEnd);
        self
    }

    fn into_descriptor(self) -> SystemDescriptor {
        self
    }
}

impl<S, Params> IntoSystemDescriptor<Params> for S
where
    S: IntoSystem<(), (), Params>,
{
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self)))
            .with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).label(label)
    }

    fn before<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).before(label)
    }

    fn after<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).after(label)
    }

    fn ambiguous_with<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).ambiguous_with(label)
    }

    fn ignore_all_ambiguities(self) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).ignore_all_ambiguities()
    }

    fn at_start(self) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self))).at_end()
    }

    fn into_descriptor(self) -> SystemDescriptor {
        SystemDescriptor::new(Box::new(IntoSystem::into_system(self)))
    }
}

impl IntoSystemDescriptor<()> for BoxedSystem<(), ()> {
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> SystemDescriptor {
        SystemDescriptor::new(self).with_run_criteria(run_criteria)
    }

    fn label(self, label: impl SystemLabel) -> SystemDescriptor {
        SystemDescriptor::new(self).label(label)
    }

    fn before<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        SystemDescriptor::new(self).before(label)
    }

    fn after<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        SystemDescriptor::new(self).after(label)
    }

    fn ambiguous_with<Marker>(self, label: impl AsSystemLabel<Marker>) -> SystemDescriptor {
        SystemDescriptor::new(self).ambiguous_with(label)
    }

    fn ignore_all_ambiguities(self) -> SystemDescriptor {
        SystemDescriptor::new(self).ignore_all_ambiguities()
    }

    fn at_start(self) -> SystemDescriptor {
        SystemDescriptor::new(self).at_start()
    }

    fn before_commands(self) -> SystemDescriptor {
        SystemDescriptor::new(self).before_commands()
    }

    fn at_end(self) -> SystemDescriptor {
        SystemDescriptor::new(self).at_end()
    }

    fn into_descriptor(self) -> SystemDescriptor {
        SystemDescriptor::new(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExclusiveInsertionPoint {
    AtStart,
    BeforeCommands,
    AtEnd,
}
