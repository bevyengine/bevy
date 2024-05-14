use bevy_ecs::{
    event::{EventReader, EventWriter},
    schedule::{IntoSystemConfigs, IntoSystemSetConfigs, Schedule},
    system::{Commands, IntoSystem, Res, ResMut},
};
use bevy_utils::all_tuples;

use self::sealed::StateSetSealed;

use super::{
    apply_state_transition, computed_states::ComputedStates, internal_apply_state_transition,
    run_enter, run_exit, run_transition, should_run_transition, sub_states::SubStates,
    ApplyStateTransition, OnEnter, OnExit, OnTransition, RefreshState, State, StateTransitionEvent,
    StateTransitionSteps, States,
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
/// the [`ComputedStates`] & [`SubStates`].
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
        let system = |refresh: Option<ResMut<RefreshState<T>>>,
                      mut parent_changed: EventReader<StateTransitionEvent<S::RawState>>,
                      event: EventWriter<StateTransitionEvent<T>>,
                      commands: Commands,
                      current_state: Option<ResMut<State<T>>>,
                      state_set: Option<Res<State<S::RawState>>>| {
            let refresh = refresh.is_some_and(|mut x| std::mem::take(&mut x.0));
            if !refresh && parent_changed.is_empty() {
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

        schedule
            .add_systems(system.in_set(ApplyStateTransition::<T>::apply()))
            .add_systems(
                should_run_transition::<T, OnEnter<T>>
                    .pipe(run_enter::<T>)
                    .in_set(StateTransitionSteps::EnterSchedules),
            )
            .add_systems(
                should_run_transition::<T, OnExit<T>>
                    .pipe(run_exit::<T>)
                    .in_set(StateTransitionSteps::ExitSchedules),
            )
            .add_systems(
                should_run_transition::<T, OnTransition<T>>
                    .pipe(run_transition::<T>)
                    .in_set(StateTransitionSteps::TransitionSchedules),
            )
            .configure_sets(
                ApplyStateTransition::<T>::apply()
                    .in_set(StateTransitionSteps::DependentTransitions)
                    .after(ApplyStateTransition::<S::RawState>::apply()),
            );
    }

    fn register_sub_state_systems_in_schedule<T: SubStates<SourceStates = Self>>(
        schedule: &mut Schedule,
    ) {
        let system = |mut parent_changed: EventReader<StateTransitionEvent<S::RawState>>,
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
                    T::should_exist(state_set)
                } else {
                    None
                };

            if current_state.is_none() || new_state.is_none() {
                internal_apply_state_transition(event, commands, current_state, new_state);
            }
        };

        schedule
            .add_systems(system.in_set(ApplyStateTransition::<T>::apply()))
            .add_systems(
                apply_state_transition::<T>.in_set(StateTransitionSteps::ManualTransitions),
            )
            .add_systems(
                should_run_transition::<T, OnEnter<T>>
                    .pipe(run_enter::<T>)
                    .in_set(StateTransitionSteps::EnterSchedules),
            )
            .add_systems(
                should_run_transition::<T, OnExit<T>>
                    .pipe(run_exit::<T>)
                    .in_set(StateTransitionSteps::ExitSchedules),
            )
            .add_systems(
                should_run_transition::<T, OnTransition<T>>
                    .pipe(run_transition::<T>)
                    .in_set(StateTransitionSteps::TransitionSchedules),
            )
            .configure_sets(
                ApplyStateTransition::<T>::apply()
                    .in_set(StateTransitionSteps::DependentTransitions)
                    .after(ApplyStateTransition::<S::RawState>::apply()),
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
                let system = |refresh: Option<ResMut<RefreshState<T>>>,
                              ($(mut $evt),*,): ($(EventReader<StateTransitionEvent<$param::RawState>>),*,),
                              event: EventWriter<StateTransitionEvent<T>>,
                              commands: Commands,
                              current_state: Option<ResMut<State<T>>>,
                              ($($val),*,): ($(Option<Res<State<$param::RawState>>>),*,)| {
                    let refresh = refresh.is_some_and(|mut x| std::mem::take(&mut x.0));
                    if !refresh && ($($evt.is_empty())&&*) {
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

                schedule
                    .add_systems(system.in_set(ApplyStateTransition::<T>::apply()))
                    .add_systems(should_run_transition::<T, OnEnter<T>>.pipe(run_enter::<T>).in_set(StateTransitionSteps::EnterSchedules))
                    .add_systems(should_run_transition::<T, OnExit<T>>.pipe(run_exit::<T>).in_set(StateTransitionSteps::ExitSchedules))
                    .add_systems(should_run_transition::<T, OnTransition<T>>.pipe(run_transition::<T>).in_set(StateTransitionSteps::TransitionSchedules))
                    .configure_sets(
                        ApplyStateTransition::<T>::apply()
                        .in_set(StateTransitionSteps::DependentTransitions)
                        $(.after(ApplyStateTransition::<$param::RawState>::apply()))*
                    );
            }

            fn register_sub_state_systems_in_schedule<T: SubStates<SourceStates = Self>>(
                schedule: &mut Schedule,
            ) {
                let system = |($(mut $evt),*,): ($(EventReader<StateTransitionEvent<$param::RawState>>),*,),
                              event: EventWriter<StateTransitionEvent<T>>,
                              commands: Commands,
                              current_state: Option<ResMut<State<T>>>,
                              ($($val),*,): ($(Option<Res<State<$param::RawState>>>),*,)| {
                    if ($($evt.is_empty())&&*) {
                        return;
                    }
                    $($evt.clear();)*

                    let new_state = if let ($(Some($val)),*,) = ($($param::convert_to_usable_state($val.as_deref())),*,) {
                        T::should_exist(($($val),*, ))
                    } else {
                        None
                    };

                    if current_state.is_none() || new_state.is_none() {
                        internal_apply_state_transition(event, commands, current_state, new_state);
                    }
                };

                schedule
                    .add_systems(system.in_set(ApplyStateTransition::<T>::apply()))
                    .add_systems(apply_state_transition::<T>.in_set(StateTransitionSteps::ManualTransitions))
                    .add_systems(should_run_transition::<T, OnEnter<T>>.pipe(run_enter::<T>).in_set(StateTransitionSteps::EnterSchedules))
                    .add_systems(should_run_transition::<T, OnExit<T>>.pipe(run_exit::<T>).in_set(StateTransitionSteps::ExitSchedules))
                    .add_systems(should_run_transition::<T, OnTransition<T>>.pipe(run_transition::<T>).in_set(StateTransitionSteps::TransitionSchedules))
                    .configure_sets(
                        ApplyStateTransition::<T>::apply()
                        .in_set(StateTransitionSteps::DependentTransitions)
                        $(.after(ApplyStateTransition::<$param::RawState>::apply()))*
                    );
            }
        }
    };
}

all_tuples!(impl_state_set_sealed_tuples, 1, 15, S, s, ereader);
