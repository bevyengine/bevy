pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(StageLabel);
define_label!(SystemLabel);
define_label!(AmbiguitySetLabel);
define_label!(RunCriteriaLabel);
