pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};
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

/// Defines one or more local, unique types implementing [`StageLabel`].
#[macro_export]
macro_rules! stage_label {
    ($($name:ident),* $(,)*) => {
        $(
            /// A macro-generated local `StageLabel`.
            #[allow(non_camel_case_types)]
            struct $name;

            impl $crate::schedule::StageLabel for $name {
                fn as_str(&self) -> &'static str {
                    std::stringify!($name)
                }
            }
        )*
    }
}

/// Defines one or more local, unique types implementing [`SystemLabel`].
#[macro_export]
macro_rules! system_label {
    ($($name:ident),* $(,)*) => {
        $(
            /// A macro-generated local `SystemLabel`.
            #[allow(non_camel_case_types)]
            struct $name;

            impl $crate::schedule::SystemLabel for $name {
                fn as_str(&self) -> &'static str {
                    std::stringify!($name)
                }
            }
        )*
    }
}

/// Defines one or more local, unique types implementing [`AmbiguitySetLabel`].
#[macro_export]
macro_rules! ambiguity_set_label {
    ($($name:ident),* $(,)*) => {
        $(
            /// A macro-generated local `AmbiguitySetLabel`.
            #[allow(non_camel_case_types)]
            struct $name;

            impl $crate::schedule::AmbiguitySetLabel for $name {
                fn as_str(&self) -> &'static str {
                    std::stringify!($name)
                }
            }
        )*
    }
}

/// Defines one or more local, unique types implementing [`RunCriteriaLabel`].
#[macro_export]
macro_rules! run_criteria_label {
    ($($name:ident),* $(,)*) => {
        $(
            /// A macro-generated local `RunCriteria`.
            #[allow(non_camel_case_types)]
            struct $name;

            impl $crate::schedule::RunCriteriaLabel for $name {
                fn as_str(&self) -> &'static str {
                    std::stringify!($name)
                }
            }
        )*
    }
}
