use std::{marker::PhantomData, mem, ops::DerefMut};

use bevy_ecs::{
    event::{Event, EventReader, EventWriter},
    schedule::{
        InternedScheduleLabel, IntoSystemSetConfigs, Schedule, ScheduleLabel, Schedules, SystemSet,
    },
    system::{Commands, In, Local, Res, ResMut},
    world::World,
};

use super::{
    freely_mutable_state::FreelyMutableState,
    resources::{NextState, State},
    states::States,
};

/// The label of a [`Schedule`] that runs whenever [`State<S>`]
/// enters this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: States>(pub S);

/// The label of a [`Schedule`] that runs whenever [`State<S>`]
/// exits this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: States>(pub S);

/// The label of a [`Schedule`] that **only** runs whenever [`State<S>`]
/// exits the `from` state, AND enters the `to` state.
///
/// Systems added to this schedule are always ran *after* [`OnExit`], and *before* [`OnEnter`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnTransition<S: States> {
    /// The state being exited.
    pub from: S,
    /// The state being entered.
    pub to: S,
}

/// Runs [state transitions](States).
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// Event sent when any state transition of `S` happens.
///
/// If you know exactly what state you want to respond to ahead of time, consider [`OnEnter`], [`OnTransition`], or [`OnExit`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Event)]
pub struct StateTransitionEvent<S: States> {
    /// the state we were in before
    pub before: Option<S>,
    /// the state we're in now
    pub after: Option<S>,
}

/// Applies manual state transitions using [`NextState<S>`].
///
/// These system sets are run sequentially, in the order of the enum variants.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StateTransitionSteps {
    ManualTransitions,
    DependentTransitions,
    ExitSchedules,
    TransitionSchedules,
    EnterSchedules,
}

/// Defines a system set to aid with dependent state ordering
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ApplyStateTransition<S: States>(PhantomData<S>);

impl<S: States> ApplyStateTransition<S> {
    pub(crate) fn apply() -> Self {
        Self(PhantomData)
    }
}

/// This function updates the current state and sends a transition event.
///
/// The `new_state` is an option to allow for removal - `None` will trigger the
/// removal of the `State<S>` resource from the [`World`].
pub(crate) fn internal_apply_state_transition<S: States>(
    mut event: EventWriter<StateTransitionEvent<S>>,
    mut commands: Commands,
    current_state: Option<ResMut<State<S>>>,
    new_state: Option<S>,
) {
    event.send(StateTransitionEvent {
        before: match (current_state, new_state.as_ref()) {
            // Update the state resource.
            (Some(mut current_state), Some(new_state)) => {
                Some(mem::replace(&mut current_state.0, new_state.clone()))
            }
            // Insert the state resource.
            (None, Some(new_state)) => {
                commands.insert_resource(State(new_state.clone()));
                None
            }
            // Remove the state resource.
            (Some(current_state), None) => {
                commands.remove_resource::<State<S>>();
                Some(current_state.get().clone())
            }
            (None, None) => return,
        },
        after: new_state,
    });
}

/// Sets up the schedules and systems for handling state transitions
/// within a [`World`].
///
/// Runs automatically when using `App` to insert states, but needs to
/// be added manually in other situations.
pub fn setup_state_transitions_in_world(
    world: &mut World,
    startup_label: Option<InternedScheduleLabel>,
) {
    let mut schedules = world.get_resource_or_insert_with(Schedules::default);
    if schedules.contains(StateTransition) {
        return;
    }
    let mut schedule = Schedule::new(StateTransition);
    schedule.configure_sets(
        (
            StateTransitionSteps::ManualTransitions,
            StateTransitionSteps::DependentTransitions,
            StateTransitionSteps::ExitSchedules,
            StateTransitionSteps::TransitionSchedules,
            StateTransitionSteps::EnterSchedules,
        )
            .chain(),
    );
    schedules.insert(schedule);

    if let Some(startup) = startup_label {
        schedules.add_systems(startup, |world: &mut World| {
            let _ = world.try_run_schedule(StateTransition);
        });
    }
}

/// If a new state is queued in [`NextState<S>`], this system
/// takes the new state value from [`NextState<S>`] and updates [`State<S>`], as well as
/// sending the relevant [`StateTransitionEvent`].
///
/// If the [`State<S>`] resource does not exist, it does nothing. Removing or adding states
/// should be done at App creation or at your own risk.
///
/// For [`SubStates`](crate::state::SubStates) - it only applies the state if the `SubState` currently exists. Otherwise, it is wiped.
/// When a `SubState` is re-created, it will use the result of it's `should_exist` method.
pub fn apply_state_transition<S: FreelyMutableState>(
    event: EventWriter<StateTransitionEvent<S>>,
    commands: Commands,
    current_state: Option<ResMut<State<S>>>,
    next_state: Option<ResMut<NextState<S>>>,
) {
    let Some(mut next_state) = next_state else {
        return;
    };

    if let NextState::Pending(next_state) = mem::take(next_state.as_mut()) {
        internal_apply_state_transition(event, commands, current_state, Some(next_state));
    }
}

pub(crate) fn should_run_transition<S: States, T: ScheduleLabel>(
    mut first: Local<bool>,
    res: Option<Res<State<S>>>,
    mut event: EventReader<StateTransitionEvent<S>>,
) -> (Option<StateTransitionEvent<S>>, PhantomData<T>) {
    let first_mut = first.deref_mut();
    if !*first_mut {
        *first_mut = true;
        if let Some(res) = res {
            event.clear();

            return (
                Some(StateTransitionEvent {
                    before: None,
                    after: Some(res.get().clone()),
                }),
                PhantomData,
            );
        }
    }
    (event.read().last().cloned(), PhantomData)
}

pub(crate) fn run_enter<S: States>(
    In((transition, _)): In<(Option<StateTransitionEvent<S>>, PhantomData<OnEnter<S>>)>,
    world: &mut World,
) {
    let Some(transition) = transition else {
        return;
    };

    let Some(after) = transition.after else {
        return;
    };

    let _ = world.try_run_schedule(OnEnter(after));
}

pub(crate) fn run_exit<S: States>(
    In((transition, _)): In<(Option<StateTransitionEvent<S>>, PhantomData<OnExit<S>>)>,
    world: &mut World,
) {
    let Some(transition) = transition else {
        return;
    };

    let Some(before) = transition.before else {
        return;
    };

    let _ = world.try_run_schedule(OnExit(before));
}

pub(crate) fn run_transition<S: States>(
    In((transition, _)): In<(
        Option<StateTransitionEvent<S>>,
        PhantomData<OnTransition<S>>,
    )>,
    world: &mut World,
) {
    let Some(transition) = transition else {
        return;
    };
    let Some(from) = transition.before else {
        return;
    };
    let Some(to) = transition.after else {
        return;
    };

    let _ = world.try_run_schedule(OnTransition { from, to });
}
