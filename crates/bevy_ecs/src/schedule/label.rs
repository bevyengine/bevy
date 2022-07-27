pub use bevy_ecs_macros::{
    AmbiguitySetLabel, RunCriteriaLabel, ScheduleLabel, StageLabel, SystemLabel,
};
use bevy_utils::define_label;

define_label!(
    /// A strongly-typed class of labels used to identify [`Stage`](crate::schedule::Stage)s.
    StageLabel,
    /// Strongly-typed identifier for a [`StageLabel`].
    StageLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [`System`](crate::system::System)s.
    SystemLabel,
    /// Strongly-typed identifier for a [`SystemLabel`].
    SystemLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify sets of systems with intentionally ambiguous execution order.
    AmbiguitySetLabel,
    /// Strongly-typed identifier for an [`AmbiguitySetLabel`].
    AmbiguitySetLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [run criteria](crate::schedule::RunCriteria).
    RunCriteriaLabel,
    /// Strongly-typed identifier for a [`RunCriteriaLabel`].
    RunCriteriaLabelId,
);

// note: this type won't be necessary come stageless, but we need it for now.
define_label!(
    /// A strongly-typed class of labels used to identify a [`Schedule`](crate::schedule::Schedule).
    ScheduleLabel,
    /// Strongly-typed identifier for a [`ScheduleLabel`].
    ScheduleLabelId,
);
