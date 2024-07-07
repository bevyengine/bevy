use std::{marker::PhantomData, mem};

use bevy_ecs::{
    event::{Event, EventReader, EventWriter},
    schedule::{IntoSystemSetConfigs, Schedule, ScheduleLabel, Schedules, SystemSet},
    system::{Commands, In, ResMut},
    world::World,
};

use super::{resources::State, states::States};

/// The label of a [`Schedule`] that **only** runs whenever [`State<S>`] enters the provided state.
///
/// This schedule ignores identity transitions.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: States>(pub S);

/// The label of a [`Schedule`] that **only** runs whenever [`State<S>`] exits the provided state.
///
/// This schedule ignores identity transitions.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: States>(pub S);

/// The label of a [`Schedule`] that **only** runs whenever [`State<S>`]
/// exits AND enters the provided `exited` and `entered` states.
///
/// Systems added to this schedule are always ran *after* [`OnExit`], and *before* [`OnEnter`].
///
/// This schedule will run on identity transitions.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnTransition<S: States> {
    /// The state being exited.
    pub exited: S,
    /// The state being entered.
    pub entered: S,
}

/// Runs [state transitions](States).
///
/// By default, it will be triggered after `PreUpdate`, but
/// you can manually trigger it at arbitrary times by creating an exclusive
/// system to run the schedule.
///
/// ```rust
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
///
/// fn run_state_transitions(world: &mut World) {
///     let _ = world.try_run_schedule(StateTransition);
/// }
/// ```
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// Event sent when any state transition of `S` happens.
/// This includes identity transitions, where `exited` and `entered` have the same value.
///
/// If you know exactly what state you want to respond to ahead of time, consider [`OnEnter`], [`OnTransition`], or [`OnExit`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Event)]
pub struct StateTransitionEvent<S: States> {
    /// The state being exited.
    pub exited: Option<S>,
    /// The state being entered.
    pub entered: Option<S>,
}

/// Applies state transitions and runs transitions schedules in order.
///
/// These system sets are run sequentially, in the order of the enum variants.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StateTransitionSteps {
    /// States apply their transitions from [`NextState`] and compute functions based on their parent states.
    DependentTransitions,
    /// Exit schedules are executed in leaf to root order
    ExitSchedules,
    /// Transition schedules are executed in arbitrary order.
    TransitionSchedules,
    /// Enter schedules are executed in root to leaf order.
    EnterSchedules,
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
/// System set that runs exit schedule(s) for state `S`.
pub struct ExitSchedules<S: States>(PhantomData<S>);

impl<S: States> Default for ExitSchedules<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
/// System set that runs transition schedule(s) for state `S`.
pub struct TransitionSchedules<S: States>(PhantomData<S>);

impl<S: States> Default for TransitionSchedules<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
/// System set that runs enter schedule(s) for state `S`.
pub struct EnterSchedules<S: States>(PhantomData<S>);

impl<S: States> Default for EnterSchedules<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// System set that applies transitions for state `S`.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ApplyStateTransition<S: States>(PhantomData<S>);

impl<S: States> Default for ApplyStateTransition<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// This function actually applies a state change, and registers the required
/// schedules for downstream computed states and transition schedules.
///
/// The `new_state` is an option to allow for removal - `None` will trigger the
/// removal of the `State<S>` resource from the [`World`].
pub(crate) fn internal_apply_state_transition<S: States>(
    mut event: EventWriter<StateTransitionEvent<S>>,
    mut commands: Commands,
    current_state: Option<ResMut<State<S>>>,
    new_state: Option<S>,
) {
    match new_state {
        Some(entered) => {
            match current_state {
                // If the [`State<S>`] resource exists, and the state is not the one we are
                // entering - we need to set the new value, compute dependant states, send transition events
                // and register transition schedules.
                Some(mut state_resource) => {
                    let exited = match *state_resource == entered {
                        true => entered.clone(),
                        false => mem::replace(&mut state_resource.0, entered.clone()),
                    };

                    // Transition events are sent even for same state transitions
                    // Although enter and exit schedules are not run by default.
                    event.send(StateTransitionEvent {
                        exited: Some(exited.clone()),
                        entered: Some(entered.clone()),
                    });
                }
                None => {
                    // If the [`State<S>`] resource does not exist, we create it, compute dependant states, send a transition event and register the `OnEnter` schedule.
                    commands.insert_resource(State(entered.clone()));

                    event.send(StateTransitionEvent {
                        exited: None,
                        entered: Some(entered.clone()),
                    });
                }
            };
        }
        None => {
            // We first remove the [`State<S>`] resource, and if one existed we compute dependant states, send a transition event and run the `OnExit` schedule.
            if let Some(resource) = current_state {
                commands.remove_resource::<State<S>>();

                event.send(StateTransitionEvent {
                    exited: Some(resource.get().clone()),
                    entered: None,
                });
            }
        }
    }
}

/// Sets up the schedules and systems for handling state transitions
/// within a [`World`].
///
/// Runs automatically when using `App` to insert states, but needs to
/// be added manually in other situations.
pub fn setup_state_transitions_in_world(world: &mut World) {
    let mut schedules = world.get_resource_or_insert_with(Schedules::default);
    if schedules.contains(StateTransition) {
        return;
    }
    let mut schedule = Schedule::new(StateTransition);
    schedule.configure_sets(
        (
            StateTransitionSteps::DependentTransitions,
            StateTransitionSteps::ExitSchedules,
            StateTransitionSteps::TransitionSchedules,
            StateTransitionSteps::EnterSchedules,
        )
            .chain(),
    );
    schedules.insert(schedule);
}

/// Returns the latest state transition event of type `S`, if any are available.
pub fn last_transition<S: States>(
    mut reader: EventReader<StateTransitionEvent<S>>,
) -> Option<StateTransitionEvent<S>> {
    reader.read().last().cloned()
}

pub(crate) fn run_enter<S: States>(
    transition: In<Option<StateTransitionEvent<S>>>,
    world: &mut World,
) {
    let Some(transition) = transition.0 else {
        return;
    };
    if transition.entered == transition.exited {
        return;
    }
    let Some(entered) = transition.entered else {
        return;
    };

    let _ = world.try_run_schedule(OnEnter(entered));
}

pub(crate) fn run_exit<S: States>(
    transition: In<Option<StateTransitionEvent<S>>>,
    world: &mut World,
) {
    let Some(transition) = transition.0 else {
        return;
    };
    if transition.entered == transition.exited {
        return;
    }
    let Some(exited) = transition.exited else {
        return;
    };

    let _ = world.try_run_schedule(OnExit(exited));
}

pub(crate) fn run_transition<S: States>(
    transition: In<Option<StateTransitionEvent<S>>>,
    world: &mut World,
) {
    let Some(transition) = transition.0 else {
        return;
    };
    let Some(exited) = transition.exited else {
        return;
    };
    let Some(entered) = transition.entered else {
        return;
    };

    let _ = world.try_run_schedule(OnTransition { exited, entered });
}
