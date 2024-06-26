use bevy_ecs::{
    event::{EventReader, EventWriter},
    schedule::{IntoSystemConfigs, IntoSystemSetConfigs, Schedule},
    system::{Commands, IntoSystem, Res, ResMut},
};
use bevy_utils::all_tuples;

use self::sealed::StateSetSealed;

use super::{
    computed_states::ComputedStates, internal_apply_state_transition, last_transition, run_enter,
    run_exit, run_transition, sub_states::SubStates, take_next_state, ApplyStateTransition,
    EnterSchedules, ExitSchedules, NextState, State, StateTransitionEvent, StateTransitionSteps,
    States, TransitionSchedules,
};

mod sealed {
    /// Sealed trait used to prevent external implementations of [`StateSet`](super::StateSet).
    pub trait StateSetSealed {}
}

/// A [`States`] type or tuple of types which implement [`States`].
///
/// This trait is used allow implementors of [`States`], as well
/// as tuples containing exclusively implementors of [`States`], to
/// be used as [`ComputedStates::SourceStates`].
///
/// It is sealed, and auto implemented for all [`States`] types and
/// tuples containing them.
pub trait StateSet: sealed::StateSetSealed {
    /// The total [`DEPENDENCY_DEPTH`](`States::DEPENDENCY_DEPTH`) of all
    /// the states that are part of this [`StateSet`], added together.
    ///
    /// Used to de-duplicate computed state executions and prevent cyclic
    /// computed states.
    const SET_DEPENDENCY_DEPTH: usize;

    /// Sets up the systems needed to compute `T` whenever any `State` in this
    /// `StateSet` is changed.
    fn register_computed_state_systems_in_schedule<T: ComputedStates<SourceStates = Self>>(
        schedule: &mut Schedule,
    );

    /// Sets up the systems needed to compute whether `T` exists whenever any `State` in this
    /// `StateSet` is changed.
    fn register_sub_state_systems_in_schedule<T: SubStates<SourceStates = Self>>(
        schedule: &mut Schedule,
    );
}

/// The `InnerStateSet` trait is used to isolate [`ComputedStates`] & [`SubStates`] from
/// needing to wrap all state dependencies in an [`Option<S>`].
///
/// Some [`ComputedStates`]'s might need to exist in different states based on the existence
/// of other states. So we needed the ability to use[`Option<S>`] when appropriate.
///
/// The isolation works because it is implemented for both S & [`Option<S>`], and has the `RawState` associated type
/// that allows it to know what the resource in the world should be. We can then essentially "unwrap" it in our
/// `StateSet` implementation - and the behaviour of that unwrapping will depend on the arguments expected by the
/// the [`ComputedStates`] & [`SubStates]`.
trait InnerStateSet: Sized {
    type RawState: States;

    const DEPENDENCY_DEPTH: usize;

    fn convert_to_usable_state(wrapped: Option<&State<Self::RawState>>) -> Option<Self>;
}

impl<S: States> InnerStateSet for S {
    type RawState = Self;

    const DEPENDENCY_DEPTH: usize = S::DEPENDENCY_DEPTH;

    fn convert_to_usable_state(wrapped: Option<&State<Self::RawState>>) -> Option<Self> {
        wrapped.map(|v| v.0.clone())
    }
}

impl<S: States> InnerStateSet for Option<S> {
    type RawState = S;

    const DEPENDENCY_DEPTH: usize = S::DEPENDENCY_DEPTH;

    fn convert_to_usable_state(wrapped: Option<&State<Self::RawState>>) -> Option<Self> {
        Some(wrapped.map(|v| v.0.clone()))
    }
}

impl<S: InnerStateSet> StateSetSealed for S {}

impl<S: InnerStateSet> StateSet for S {
    const SET_DEPENDENCY_DEPTH: usize = S::DEPENDENCY_DEPTH;

    fn register_computed_state_systems_in_schedule<T: ComputedStates<SourceStates = Self>>(
        schedule: &mut Schedule,
    ) {
        let apply_state_transition =
            |mut parent_changed: EventReader<StateTransitionEvent<S::RawState>>,
             event: EventWriter<StateTransitionEvent<T>>,
             commands: Commands,
             current_state: Option<ResMut<State<T>>>,
             state_set: Option<Res<State<S::RawState>>>| {
                if parent_changed.is_empty() {
                    return;
                }
                parent_changed.clear();

                let new_state =
                    if let Some(state_set) = S::convert_to_usable_state(state_set.as_deref()) {
                        T::compute(state_set)
                    } else {
                        None
                    };

                internal_apply_state_transition(event, commands, current_state, new_state);
            };

        schedule.configure_sets((
            ApplyStateTransition::<T>::default()
                .in_set(StateTransitionSteps::DependentTransitions)
                .after(ApplyStateTransition::<S::RawState>::default()),
            ExitSchedules::<T>::default()
                .in_set(StateTransitionSteps::ExitSchedules)
                .before(ExitSchedules::<S::RawState>::default()),
            TransitionSchedules::<T>::default().in_set(StateTransitionSteps::TransitionSchedules),
            EnterSchedules::<T>::default()
                .in_set(StateTransitionSteps::EnterSchedules)
                .after(EnterSchedules::<S::RawState>::default()),
        ));

        schedule
            .add_systems(apply_state_transition.in_set(ApplyStateTransition::<T>::default()))
            .add_systems(
                last_transition::<T>
                    .pipe(run_exit::<T>)
                    .in_set(ExitSchedules::<T>::default()),
            )
            .add_systems(
                last_transition::<T>
                    .pipe(run_transition::<T>)
                    .in_set(TransitionSchedules::<T>::default()),
            )
            .add_systems(
                last_transition::<T>
                    .pipe(run_enter::<T>)
                    .in_set(EnterSchedules::<T>::default()),
            );
    }

    fn register_sub_state_systems_in_schedule<T: SubStates<SourceStates = Self>>(
        schedule: &mut Schedule,
    ) {
        // | parent changed | next state | already exists | should exist | what happens                     |
        // | -------------- | ---------- | -------------- | ------------ | -------------------------------- |
        // | false          | false      | false          | -            | -                                |
        // | false          | false      | true           | -            | -                                |
        // | false          | true       | false          | false        | -                                |
        // | true           | false      | false          | false        | -                                |
        // | true           | true       | false          | false        | -                                |
        // | true           | false      | true           | false        | Some(current) -> None            |
        // | true           | true       | true           | false        | Some(current) -> None            |
        // | true           | false      | false          | true         | None -> Some(default)            |
        // | true           | true       | false          | true         | None -> Some(next)               |
        // | true           | true       | true           | true         | Some(current) -> Some(next)      |
        // | false          | true       | true           | true         | Some(current) -> Some(next)      |
        // | true           | false      | true           | true         | Some(current) -> Some(current)   |

        let apply_state_transition =
            |mut parent_changed: EventReader<StateTransitionEvent<S::RawState>>,
             event: EventWriter<StateTransitionEvent<T>>,
             commands: Commands,
             current_state_res: Option<ResMut<State<T>>>,
             next_state_res: Option<ResMut<NextState<T>>>,
             state_set: Option<Res<State<S::RawState>>>| {
                let parent_changed = parent_changed.read().last().is_some();
                let next_state = take_next_state(next_state_res);

                if !parent_changed && next_state.is_none() {
                    return;
                }

                let current_state = current_state_res.as_ref().map(|s| s.get()).cloned();

                let initial_state = if parent_changed {
                    if let Some(state_set) = S::convert_to_usable_state(state_set.as_deref()) {
                        T::should_exist(state_set)
                    } else {
                        None
                    }
                } else {
                    current_state.clone()
                };
                let new_state = initial_state.map(|x| next_state.or(current_state).unwrap_or(x));

                internal_apply_state_transition(event, commands, current_state_res, new_state);
            };

        schedule.configure_sets((
            ApplyStateTransition::<T>::default()
                .in_set(StateTransitionSteps::DependentTransitions)
                .after(ApplyStateTransition::<S::RawState>::default()),
            ExitSchedules::<T>::default()
                .in_set(StateTransitionSteps::ExitSchedules)
                .before(ExitSchedules::<S::RawState>::default()),
            TransitionSchedules::<T>::default().in_set(StateTransitionSteps::TransitionSchedules),
            EnterSchedules::<T>::default()
                .in_set(StateTransitionSteps::EnterSchedules)
                .after(EnterSchedules::<S::RawState>::default()),
        ));

        schedule
            .add_systems(apply_state_transition.in_set(ApplyStateTransition::<T>::default()))
            .add_systems(
                last_transition::<T>
                    .pipe(run_exit::<T>)
                    .in_set(ExitSchedules::<T>::default()),
            )
            .add_systems(
                last_transition::<T>
                    .pipe(run_transition::<T>)
                    .in_set(TransitionSchedules::<T>::default()),
            )
            .add_systems(
                last_transition::<T>
                    .pipe(run_enter::<T>)
                    .in_set(EnterSchedules::<T>::default()),
            );
    }
}

macro_rules! impl_state_set_sealed_tuples {
    ($(($param: ident, $val: ident, $evt: ident)), *) => {
        impl<$($param: InnerStateSet),*> StateSetSealed for  ($($param,)*) {}

        impl<$($param: InnerStateSet),*> StateSet for  ($($param,)*) {

            const SET_DEPENDENCY_DEPTH : usize = $($param::DEPENDENCY_DEPTH +)* 0;


            fn register_computed_state_systems_in_schedule<T: ComputedStates<SourceStates = Self>>(
                schedule: &mut Schedule,
            ) {
                let apply_state_transition =
                    |($(mut $evt),*,): ($(EventReader<StateTransitionEvent<$param::RawState>>),*,),
                     event: EventWriter<StateTransitionEvent<T>>,
                     commands: Commands,
                     current_state: Option<ResMut<State<T>>>,
                     ($($val),*,): ($(Option<Res<State<$param::RawState>>>),*,)| {
                        if ($($evt.is_empty())&&*) {
                            return;
                        }
                        $($evt.clear();)*

                        let new_state = if let ($(Some($val)),*,) = ($($param::convert_to_usable_state($val.as_deref())),*,) {
                            T::compute(($($val),*, ))
                        } else {
                            None
                        };

                        internal_apply_state_transition(event, commands, current_state, new_state);
                    };

                schedule.configure_sets((
                    ApplyStateTransition::<T>::default()
                        .in_set(StateTransitionSteps::DependentTransitions)
                        $(.after(ApplyStateTransition::<$param::RawState>::default()))*,
                    ExitSchedules::<T>::default()
                        .in_set(StateTransitionSteps::ExitSchedules)
                        $(.before(ExitSchedules::<$param::RawState>::default()))*,
                    TransitionSchedules::<T>::default()
                        .in_set(StateTransitionSteps::TransitionSchedules),
                    EnterSchedules::<T>::default()
                        .in_set(StateTransitionSteps::EnterSchedules)
                        $(.after(EnterSchedules::<$param::RawState>::default()))*,
                ));

                schedule
                    .add_systems(apply_state_transition.in_set(ApplyStateTransition::<T>::default()))
                    .add_systems(last_transition::<T>.pipe(run_exit::<T>).in_set(ExitSchedules::<T>::default()))
                    .add_systems(last_transition::<T>.pipe(run_transition::<T>).in_set(TransitionSchedules::<T>::default()))
                    .add_systems(last_transition::<T>.pipe(run_enter::<T>).in_set(EnterSchedules::<T>::default()));
            }

            fn register_sub_state_systems_in_schedule<T: SubStates<SourceStates = Self>>(
                schedule: &mut Schedule,
            ) {
                let apply_state_transition =
                    |($(mut $evt),*,): ($(EventReader<StateTransitionEvent<$param::RawState>>),*,),
                     event: EventWriter<StateTransitionEvent<T>>,
                     commands: Commands,
                     current_state_res: Option<ResMut<State<T>>>,
                     next_state_res: Option<ResMut<NextState<T>>>,
                     ($($val),*,): ($(Option<Res<State<$param::RawState>>>),*,)| {
                        let parent_changed = ($($evt.read().last().is_some())&&*);
                        let next_state = take_next_state(next_state_res);

                        if !parent_changed && next_state.is_none() {
                            return;
                        }

                        let current_state = current_state_res.as_ref().map(|s| s.get()).cloned();

                        let initial_state = if parent_changed {
                            if let ($(Some($val)),*,) = ($($param::convert_to_usable_state($val.as_deref())),*,) {
                                T::should_exist(($($val),*, ))
                            } else {
                                None
                            }
                        } else {
                            current_state.clone()
                        };
                        let new_state = initial_state.map(|x| next_state.or(current_state).unwrap_or(x));

                        internal_apply_state_transition(event, commands, current_state_res, new_state);
                    };

                schedule.configure_sets((
                    ApplyStateTransition::<T>::default()
                        .in_set(StateTransitionSteps::DependentTransitions)
                        $(.after(ApplyStateTransition::<$param::RawState>::default()))*,
                    ExitSchedules::<T>::default()
                        .in_set(StateTransitionSteps::ExitSchedules)
                        $(.before(ExitSchedules::<$param::RawState>::default()))*,
                    TransitionSchedules::<T>::default()
                        .in_set(StateTransitionSteps::TransitionSchedules),
                    EnterSchedules::<T>::default()
                        .in_set(StateTransitionSteps::EnterSchedules)
                        $(.after(EnterSchedules::<$param::RawState>::default()))*,
                ));

                schedule
                    .add_systems(apply_state_transition.in_set(ApplyStateTransition::<T>::default()))
                    .add_systems(last_transition::<T>.pipe(run_exit::<T>).in_set(ExitSchedules::<T>::default()))
                    .add_systems(last_transition::<T>.pipe(run_transition::<T>).in_set(TransitionSchedules::<T>::default()))
                    .add_systems(last_transition::<T>.pipe(run_enter::<T>).in_set(EnterSchedules::<T>::default()));
            }
        }
    };
}

all_tuples!(impl_state_set_sealed_tuples, 1, 15, S, s, ereader);
