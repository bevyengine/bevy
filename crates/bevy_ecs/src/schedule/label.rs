pub use bevy_ecs_macros::{RunCriteriaLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(
    /// A strongly-typed class of labels used to identify [`System`](crate::system::System)s.
    SystemLabel,
    /// Strongly-typed identifier for a [`SystemLabel`].
    SystemLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [run criteria](crate::schedule::RunCriteria).
    RunCriteriaLabel,
    /// Strongly-typed identifier for a [`RunCriteriaLabel`].
    RunCriteriaLabelId,
);
