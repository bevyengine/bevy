pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(
    /// A strongly-typed class of labels used to identify [`Stage`](crate::schedule::Stage)s.
    StageLabel,
    /// Types that can be converted into [`StageLabelId`], except for `StageLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`StageLabel`] for you.
    IntoStageLabel,
    /// Strongly-typed identifier for a [`StageLabel`].
    StageLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [`System`](crate::system::System)s.
    SystemLabel,
    /// Types that can be converted into [`SystemLabelId`], except for `SystemLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`SystemLabel`] for you.
    IntoSystemLabel,
    /// Strongly-typed identifier for a [`SystemLabel`].
    SystemLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify sets of systems with intentionally ambiguous execution order.
    AmbiguitySetLabel,
    /// Types that can be converted into [`AmbiguitySetLabelId`], except for `AmbiguitySetLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`AmbiguitySetLabel`] for you.
    IntoAmbiguitySetLabel,
    /// Strongly-typed identifier for an [`AmbiguitySetLabel`].
    AmbiguitySetLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [run criteria](crate::schedule::RunCriteria).
    RunCriteriaLabel,
    /// Types that can be converted into [`RunCriteriaLabelId`], except for `RunCriteriaLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`RunCriteriaLabel`] for you.
    IntoRunCriteriaLabel,
    /// Strongly-typed identifier for a [`RunCriteriaLabel`].
    RunCriteriaLabelId,
);
