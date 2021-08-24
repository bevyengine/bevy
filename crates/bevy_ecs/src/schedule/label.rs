pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};

use bevy_utils::define_label;

define_label!(StageLabel);
pub(crate) type BoxedStageLabel = Box<dyn StageLabel>;

define_label!(SystemLabel);
pub(crate) type BoxedSystemLabel = Box<dyn SystemLabel>;

define_label!(AmbiguitySetLabel);
pub(crate) type BoxedAmbiguitySetLabel = Box<dyn AmbiguitySetLabel>;

define_label!(RunCriteriaLabel);
pub(crate) type BoxedRunCriteriaLabel = Box<dyn RunCriteriaLabel>;
