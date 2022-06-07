pub use bevy_ecs_macros::{
    IntoAmbiguitySetLabel, IntoRunCriteriaLabel, IntoStageLabel, IntoSystemLabel,
};
use bevy_utils::define_label;

define_label!(StageLabel, IntoStageLabel);
define_label!(SystemLabel, IntoSystemLabel);
define_label!(AmbiguitySetLabel, IntoAmbiguitySetLabel);
define_label!(RunCriteriaLabel, IntoRunCriteriaLabel);
