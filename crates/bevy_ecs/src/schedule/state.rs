use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
use std::ops::Deref;

use crate as bevy_ecs;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::{ScheduleLabel, SystemConfigs};
use crate::system::{Res, ResMut, Resource, SystemState};
use crate::world::World;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;

use crate::prelude::IntoSystemConfigs;
pub use bevy_ecs_macros::States;
use bevy_ecs_macros::SystemSet;

/// Types that can define world-wide states in a finite-state machine.
///
/// The [`Default`] trait defines the starting state.
/// Multiple states can be defined for the same world,
/// allowing you to classify the state of the world across orthogonal dimensions.
/// You can access the current state of type `T` with the [`State<T>`] resource,
/// and the queued state with the [`NextState<T>`] resource.
///
/// [`apply_state_transition_systems`] configures the systems,
/// and these systems are installed by `App::add_state`.
///
/// # Example
///
/// ```rust
/// use bevy_ecs::prelude::States;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///  #[default]
///   MainMenu,
///   SettingsMenu,
///   InGame,
/// }
///
/// ```
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug + Default {}

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// enters this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: States>(pub S);

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// exits this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: States>(pub S);

/// The label of a [`Schedule`](super::Schedule) that **only** runs whenever [`State<S>`]
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

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`] and [`OnExit(state)`]).
///
/// The current state value can be accessed through this resource. To *change* the state,
/// queue a transition in the [`NextState<S>`] resource, and it will be applied by the
/// [`apply_state_transition_systems`].
///
/// The starting state is defined via the [`Default`] implementation for `S`.
#[derive(Resource, Default, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default)
)]
pub struct State<S: States>(S);

impl<S: States> State<S> {
    /// Creates a new state with a specific value.
    ///
    /// To change the state use [`NextState<S>`] rather than using this to modify the `State<S>`.
    pub fn new(state: S) -> Self {
        Self(state)
    }

    /// Get the current state.
    pub fn get(&self) -> &S {
        &self.0
    }
}

impl<S: States> PartialEq<S> for State<S> {
    fn eq(&self, other: &S) -> bool {
        self.get() == other
    }
}

impl<S: States> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// The next state of [`State<S>`].
///
/// To queue a transition, just set the contained value to `Some(next_state)`.
#[derive(Resource, Default, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default)
)]
pub struct NextState<S: States>(pub Option<S>);

impl<S: States> NextState<S> {
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        self.0 = Some(state);
    }
}

/// The previous state of [`State<S>`].
///
/// This is used internally by Bevy and generally not meant to be used directly.
/// Precise semantics of data in this resource are not specified.
#[derive(Resource, Default, Debug)]
pub struct PrevState<S: States>(pub Option<S>);

/// Label to order all [`OnEnter`], [`OnTransition`], and [`OnExit`] systems.
///
/// All `OnEnter` will run before all `OnTransition`,
/// and all `OnTransition` will run before all `OnExit`.
///
/// The order of `OnEnter` systems relative to each other (and so on) is ambiguous by default.
#[derive(SystemSet, Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum StateTransitionSet {
    /// Call `OnEnter` once on first run.
    RunEnterOnce,
    /// Change all the states before invoking transition systems here.
    BeforeTransition,
    /// All [`OnEnter`] schedules are executed in this set.
    OnEnter,
    /// All [`OnTransition`] schedules are executed in this set.
    OnTransition,
    /// All [`OnExit`] schedules are executed in this set.
    OnExit,
    /// Cleanup `PrevState` after all transition systems have run.
    AfterTransition,
}

/// Run all state transition systems for the given state type.
///
/// This function is deprecated. It is not used internally by Bevy,
/// and will be removed in a future version.
#[deprecated(since = "0.13.0")]
pub fn apply_state_transition<S: States>(
    world: &mut World,
    next_state_changed_states: &mut SystemState<(Res<State<S>>, Res<NextState<S>>)>,
    before_transition_states: &mut SystemState<(
        ResMut<PrevState<S>>,
        ResMut<State<S>>,
        ResMut<NextState<S>>,
    )>,
    after_transition_states: &mut SystemState<ResMut<PrevState<S>>>,
) {
    let state_changed = {
        let (state, next) = next_state_changed_states.get(world);
        next_state_changed(state, next)
    };
    if state_changed {
        {
            let (prev, state, next) = before_transition_states.get_mut(world);
            apply_before_transition::<S>(prev, state, next);
        }
        run_exit_schedule::<S>(world);
        run_transition_schedule::<S>(world);
        run_enter_schedule::<S>(world);
        {
            let states = after_transition_states.get_mut(world);
            apply_after_transition::<S>(states);
        }
    }
}

/// Configure systems running state transition callbacks.
///
/// On state change, the following schedules will be run in order:
/// * [`OnExit<S>`]
/// * [`OnTransition<S>`]
/// * [`OnEnter<S>`]
///
/// This is called by `App::add_state`, and is not meant to be used directly.
pub fn apply_state_transition_systems<S: States>() -> SystemConfigs {
    // Extra `.chain()` calls case `apply_state_transition_systems` is used directly
    // without properly configuring the `StateTransitionSet` ordering.
    (
        apply_before_transition::<S>.in_set(StateTransitionSet::BeforeTransition),
        run_exit_schedule::<S>.in_set(StateTransitionSet::OnExit),
        run_transition_schedule::<S>.in_set(StateTransitionSet::OnTransition),
        run_enter_schedule::<S>.in_set(StateTransitionSet::OnEnter),
        apply_after_transition::<S>.in_set(StateTransitionSet::AfterTransition),
    )
        .chain()
        .run_if(next_state_changed::<S>)
}

/// Condition used in the beginning of the schedule to check if the next state has changed.
fn next_state_changed<S: States>(state: Res<State<S>>, next_state: Res<NextState<S>>) -> bool {
    match &next_state.0 {
        None => false,
        Some(next_state) => next_state != state.get(),
    }
}

/// Change the state before invoking transition systems.
fn apply_before_transition<S: States>(
    mut prev_state: ResMut<PrevState<S>>,
    mut state: ResMut<State<S>>,
    mut next_state: ResMut<NextState<S>>,
) {
    let entered = next_state.0.take();
    let entered = entered.expect("NextState<S> should be Some if next_state_changed::<S> is true");

    debug_assert_ne!(*state, entered);

    let exited = mem::replace(&mut state.0, entered);
    prev_state.0 = Some(exited);
}

/// If a new state is queued in [`NextState<S>`], this system
/// runs the [`OnExit(exited_state)`] schedule, if it exists.
fn run_exit_schedule<S: States>(world: &mut World) {
    let exited = world.resource::<PrevState<S>>().0.clone();
    let exited = exited.expect("PrevState<S> should be Some after apply_before_transition::<S>");

    // Try to run the schedules if they exist.
    world.try_run_schedule(OnExit(exited)).ok();
}

/// If a new state is queued in [`NextState<S>`], this system
/// runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition), if it exists.
fn run_transition_schedule<S: States>(world: &mut World) {
    let exited = world.resource::<PrevState<S>>().0.clone();
    let exited = exited.expect("PrevState<S> should be Some after apply_before_transition::<S>");

    let entered = world.resource::<State<S>>().0.clone();

    // Try to run the schedules if they exist.
    world
        .try_run_schedule(OnTransition {
            from: exited,
            to: entered,
        })
        .ok();
}

/// Run the enter schedule (if it exists) for the current state.
pub fn run_enter_schedule<S: States>(world: &mut World) {
    world
        .try_run_schedule(OnEnter(world.resource::<State<S>>().0.clone()))
        .ok();
}

/// Change the state after invoking transition systems.
fn apply_after_transition<S: States>(mut prev: ResMut<PrevState<S>>) {
    let prev = prev.0.take();

    debug_assert!(
        prev.is_some(),
        "PrevState<S> should be Some after apply_before_transition::<S>"
    );
}
