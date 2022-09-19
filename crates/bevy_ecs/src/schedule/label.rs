pub use bevy_ecs_macros::{RunCriteriaLabel, StageLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(
    /// A strongly-typed class of labels used to identify [`Stage`](crate::schedule::Stage)s.
    StageLabel,
    /// Types that can be converted into [`StageLabelId`], except for `StageLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`StageLabel`] due to a blanket implementation.
    IntoStageLabel,
    /// Strongly-typed identifier for a [`StageLabel`].
    StageLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [`System`](crate::system::System)s.
    SystemLabel,
    /// Types that can be converted into [`SystemLabelId`], except for `SystemLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`SystemLabel`] due to a blanket implementation.
    IntoSystemLabel,
    /// Strongly-typed identifier for a [`SystemLabel`].
    SystemLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [run criteria](crate::schedule::RunCriteria).
    RunCriteriaLabel,
    /// Types that can be converted into [`RunCriteriaLabelId`], except for `RunCriteriaLabelId` itself.
    ///
    /// Implementing this trait automatically implements [`RunCriteriaLabel`] due to a blanket implementation.
    IntoRunCriteriaLabel,
    /// Strongly-typed identifier for a [`RunCriteriaLabel`].
    RunCriteriaLabelId,
);
