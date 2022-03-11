mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

pub use fixed_timestep::*;
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{Time, Timer};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemLabel)]
/// Updates the elapsed time. Any system that interacts with [Time] component should run after
/// this.
pub struct TimeSystem;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<FixedTimesteps>()
            .register_type::<Timer>()
            // time system is added as an "exclusive system" to ensure it runs before other systems
            // in CoreStage::First
            .add_system_to_stage(
                CoreStage::First,
                time_system.exclusive_system().at_start().label(TimeSystem),
            );
    }
}

fn time_system(mut time: ResMut<Time>) {
    time.update();
}
