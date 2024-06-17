use bevy_ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};
use bevy_ecs::system::IntoSystem;
use bevy_ecs::{
    event::EventWriter,
    prelude::Schedule,
    system::{Commands, ResMut},
};

use super::{states::States, NextState, State};
use super::{take_next_state, transitions::*};

/// This trait allows a state to be mutated directly using the [`NextState<S>`](crate::state::NextState) resource.
///
/// While ordinary states are freely mutable (and implement this trait as part of their derive macro),
/// computed states are not: instead, they can *only* change when the states that drive them do.
#[diagnostic::on_unimplemented(note = "consider annotating `{Self}` with `#[derive(States)]`")]
pub trait FreelyMutableState: States {
    /// This function registers all the necessary systems to apply state changes and run transition schedules
    fn register_state(schedule: &mut Schedule) {
        schedule.configure_sets((
            ApplyStateTransition::<Self>::default()
                .in_set(StateTransitionSteps::DependentTransitions),
            ExitSchedules::<Self>::default().in_set(StateTransitionSteps::ExitSchedules),
            TransitionSchedules::<Self>::default()
                .in_set(StateTransitionSteps::TransitionSchedules),
            EnterSchedules::<Self>::default().in_set(StateTransitionSteps::EnterSchedules),
        ));

        schedule
            .add_systems(
                apply_state_transition::<Self>.in_set(ApplyStateTransition::<Self>::default()),
            )
            .add_systems(
                last_transition::<Self>
                    .pipe(run_exit::<Self>)
                    .in_set(ExitSchedules::<Self>::default()),
            )
            .add_systems(
                last_transition::<Self>
                    .pipe(run_transition::<Self>)
                    .in_set(TransitionSchedules::<Self>::default()),
            )
            .add_systems(
                last_transition::<Self>
                    .pipe(run_enter::<Self>)
                    .in_set(EnterSchedules::<Self>::default()),
            );
    }
}

fn apply_state_transition<S: FreelyMutableState>(
    event: EventWriter<StateTransitionEvent<S>>,
    commands: Commands,
    current_state: Option<ResMut<State<S>>>,
    next_state: Option<ResMut<NextState<S>>>,
) {
    let Some(next_state) = take_next_state(next_state) else {
        return;
    };
    let Some(current_state) = current_state else {
        return;
    };
    internal_apply_state_transition(event, commands, Some(current_state), Some(next_state));
}
