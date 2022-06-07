pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(StageLabelId, StageLabel);
define_label!(SystemLabelId, SystemLabel);
define_label!(AmbiguitySetLabelId, AmbiguitySetLabel);
define_label!(RunCriteriaLabelId, RunCriteriaLabel);
