use bevy_ecs::prelude::Schedule;
use bevy_ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};
use bevy_ecs::system::IntoSystem;

use super::states::States;
use super::transitions::*;

/// This trait allows a state to be mutated directly using the [`NextState<S>`](crate::state::NextState) resource.
///
/// While ordinary states are freely mutable (and implement this trait as part of their derive macro),
/// computed states are not: instead, they can *only* change when the states that drive them do.
#[diagnostic::on_unimplemented(note = "consider annotating `{Self}` with `#[derive(States)]`")]
pub trait FreelyMutableState: States {
    /// This function registers all the necessary systems to apply state changes and run transition schedules
    fn register_state(schedule: &mut Schedule) {
        schedule
            .add_systems(
                apply_state_transition::<Self>.in_set(ApplyStateTransition::<Self>::apply()),
            )
            .add_systems(
                should_run_transition::<Self, OnEnter<Self>>
                    .pipe(run_enter::<Self>)
                    .in_set(StateTransitionSteps::EnterSchedules),
            )
            .add_systems(
                should_run_transition::<Self, OnExit<Self>>
                    .pipe(run_exit::<Self>)
                    .in_set(StateTransitionSteps::ExitSchedules),
            )
            .add_systems(
                should_run_transition::<Self, OnTransition<Self>>
                    .pipe(run_transition::<Self>)
                    .in_set(StateTransitionSteps::TransitionSchedules),
            )
            .configure_sets(
                ApplyStateTransition::<Self>::apply()
                    .in_set(StateTransitionSteps::ManualTransitions),
            );
    }
}
