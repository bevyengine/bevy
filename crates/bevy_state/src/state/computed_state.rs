use bevy_ecs::schedule::Schedule;

use super::{StateSet, States};

pub trait ComputedStates: States {}

/// This function sets up systems that compute the state whenever one of the [`SourceStates`](Self::SourceStates)
/// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
/// used.
pub fn register_computed_state_systems<T: ComputedStates, SourceStates: StateSet>(
    schedule: &mut Schedule,
    f: impl Fn(SourceStates) -> Option<T> + Send + Sync + 'static,
) {
    SourceStates::register_computed_state_systems_in_schedule(schedule, f);
}
