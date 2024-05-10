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
                    if *state_resource != entered {
                        let exited = mem::replace(&mut state_resource.0, entered.clone());

                        event.send(StateTransitionEvent {
                            before: Some(exited.clone()),
                            after: Some(entered.clone()),
                        });
                    }
                }
                None => {
                    // If the [`State<S>`] resource does not exist, we create it, compute dependant states, send a transition event and register the `OnEnter` schedule.
                    commands.insert_resource(State(entered.clone()));

                    event.send(StateTransitionEvent {
                        before: None,
                        after: Some(entered.clone()),
                    });
                }
            };
        }
        None => {
            // We first remove the [`State<S>`] resource, and if one existed we compute dependant states, send a transition event and run the `OnExit` schedule.
            if let Some(resource) = current_state {
                commands.remove_resource::<State<S>>();

                event.send(StateTransitionEvent {
                    before: Some(resource.get().clone()),
                    after: None,
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
    // We want to check if the State and NextState resources exist
    let Some(mut next_state_resource) = next_state else {
        return;
    };

    match next_state_resource.as_ref() {
        NextState::Pending(new_state) => {
            if let Some(current_state) = current_state {
                if new_state != current_state.get() {
                    let new_state = new_state.clone();
                    internal_apply_state_transition(
                        event,
                        commands,
                        Some(current_state),
                        Some(new_state),
                    );
                }
            }
        }
        NextState::Unchanged => {
            // This is the default value, so we don't need to re-insert the resource
            return;
        }
    }

    *next_state_resource.as_mut() = NextState::<S>::Unchanged;
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
