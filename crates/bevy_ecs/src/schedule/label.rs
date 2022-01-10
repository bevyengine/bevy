pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};
use bevy_utils::define_label;

define_label!(StageLabel);
define_label!(SystemLabel);
define_label!(AmbiguitySetLabel);
define_label!(RunCriteriaLabel);

pub(crate) type BoxedStageLabel = Box<dyn StageLabel>;
pub(crate) type BoxedSystemLabel = Box<dyn SystemLabel>;
pub(crate) type BoxedAmbiguitySetLabel = Box<dyn AmbiguitySetLabel>;
pub(crate) type BoxedRunCriteriaLabel = Box<dyn RunCriteriaLabel>;
