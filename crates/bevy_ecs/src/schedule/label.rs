pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(StageLabel, BoxedStageLabel);
define_label!(SystemLabel, BoxedSystemLabel);
define_label!(AmbiguitySetLabel, BoxedAmbiguitySetLabel);
define_label!(RunCriteriaLabel, BoxedRunCriteriaLabel);
