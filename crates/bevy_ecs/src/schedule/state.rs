use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::sync::Arc;

use crate as bevy_ecs;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::{IntoSystem, Resource};
use crate::world::World;
pub use bevy_ecs_macros::state_matcher;
pub use bevy_ecs_macros::States;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;

use super::{BoxedCondition, IntoConditionalScheduleLabel};
use bevy_ecs::prelude::Res;

/// Types that can define world-wide states in a finite-state machine.
///
/// The [`Default`] trait defines the starting state.
/// Multiple states can be defined for the same world,
/// allowing you to classify the state of the world across orthogonal dimensions.
/// You can access the current state of type `T` with the [`State<T>`] resource,
/// and the queued state with the [`NextState<T>`] resource.
///
/// State transitions typically occur in the [`OnEnter<T>`] and [`OnExit<T>`] schedules,
/// which can be run via the [`apply_state_transition::<T>`] system.
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

/// Types that can match world-wide states.
pub trait StateMatcher<S: States>: 'static + Send + Sync + Clone {
    /// Check whether to match with the current state
    fn match_state(&self, state: &S) -> bool;
}

impl<S: States> StateMatcher<S> for S {
    fn match_state(&self, state: &S) -> bool {
        self == state
    }
}

/// A struct for containing a state matcher function
#[derive(Clone)]
pub struct StateMatcherFunction<S: States>(Arc<dyn Fn(&S) -> bool + 'static + Send + Sync>);

impl<S: States> StateMatcher<S> for StateMatcherFunction<S> {
    fn match_state(&self, state: &S) -> bool {
        self.0(state)
    }
}

/// A trait for definint items that can become a state matcher
pub trait IntoStateMatcher<S: States, M: StateMatcher<S>> {
    /// Transform item into a state matcher
    fn into_state_matcher(self) -> M;
}

impl<S: States, F: Fn(&S) -> bool + 'static + Send + Sync>
    IntoStateMatcher<S, StateMatcherFunction<S>> for F
{
    fn into_state_matcher(self) -> StateMatcherFunction<S> {
        StateMatcherFunction(Arc::new(self))
    }
}

impl<S: States, M: StateMatcher<S>> IntoStateMatcher<S, M> for M {
    fn into_state_matcher(self) -> M {
        self
    }
}

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// enters this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: States>(pub S);

impl<S: States> OnEnter<S> {
    /// Entering a state that matches the matcher from a state that doesn't
    pub fn matching<M: StateMatcher<S>>(matcher: M) -> OnEnterMatching<S, M> {
        OnEnterMatching {
            matcher,
            strict: false,
            phantom: PhantomData::<S>,
        }
    }

    /// Entering a state that matches the matcher regardless of the previous state
    pub fn matching_strict<M: StateMatcher<S>>(matcher: M) -> OnEnterMatching<S, M> {
        OnEnterMatching {
            matcher,
            strict: true,
            phantom: PhantomData::<S>,
        }
    }

    /// Entering a state that matches the matcher from a state that doesn't
    pub fn match_function<F: Fn(&S) -> bool + 'static + Send + Sync>(
        matcher: F,
    ) -> OnEnterMatching<S, StateMatcherFunction<S>> {
        let matcher = matcher.into_state_matcher();
        OnEnterMatching {
            matcher,
            strict: false,
            phantom: PhantomData::<S>,
        }
    }

    /// Entering a state that matches the matcher from a state that doesn't
    pub fn match_function_strict<F: Fn(&S) -> bool + 'static + Send + Sync>(
        matcher: F,
    ) -> OnEnterMatching<S, StateMatcherFunction<S>> {
        let matcher = matcher.into_state_matcher();
        OnEnterMatching {
            matcher,
            strict: true,
            phantom: PhantomData::<S>,
        }
    }
}

/// Generate OnEnter schedules, using either:
/// - a pre-existing matcher, like so `on_enter!(MyMatcher)`
/// - a matching function, like so `on_enter!(MyState, |s| false)`
#[macro_export]
macro_rules! on_enter {
    ($type:ident, $expression:pat) => {{
        let matcher = |state: &$type| matches!(state.clone(), $expression);

        OnEnter::match_function(matcher)
    }};
    ($expression:expr) => {{
        OnEnter::matching($expression)
    }};
}

/// Generate strict OnEnter schedules, using either:
/// - a pre-existing matcher, like so `on_enter_strict!(MyMatcher)`
/// - a matching function, like so `on_enter_strict!(MyState, |s| false)
#[macro_export]
macro_rules! on_enter_strict {
    ($type:ident, $expression:pat) => {{
        let matcher = |state: &$type| matches!(state.clone(), $expression);

        OnEnter::match_function_strict(matcher)
    }};
    ($expression:expr) => {{
        OnEnter::matching_strict($expression)
    }};
}

/// Generate OnExit schedules, using either:
/// - a pre-existing matcher, like so `on_exit!(MyMatcher)`
/// - a matching function, like so `on_exit!(MyState, |s| false)`
#[macro_export]
macro_rules! on_exit {
    ($type:ident, $expression:pat) => {{
        let matcher = |state: &$type| matches!(state.clone(), $expression);

        OnExit::match_function(matcher)
    }};
    ($expression:expr) => {{
        OnExit::matching($expression)
    }};
}

/// Generate strict OnExit schedules, using either:
/// - a pre-existing matcher, like so `on_exit_strict!(MyMatcher)`
/// - a matching function, like so `on_exit_strict!(MyState, |s| false)
#[macro_export]
macro_rules! on_exit_strict {
    ($type:ident, $expression:pat) => {{
        let matcher = |state: &$type| matches!(state.clone(), $expression);

        OnExit::match_function_strict(matcher)
    }};
    ($expression:expr) => {{
        OnExit::matching_strict($expression)
    }};
}

/// Generate in_state condition using either:
/// - a pre-existing matcher, like so `on_exit!(MyMatcher)`
/// - a matching function, like so `on_exit!(MyState, |s| false)`
#[macro_export]
macro_rules! in_state {
    ($type:ident, $expression:pat) => {{
        let matcher = |state: &$type| matches!(state.clone(), $expression);
        let matcher: StateMatcherFunction<$type> = matcher.into_state_matcher();

        in_state(matcher)
    }};
    ($expression:expr) => {{
        in_state($expression)
    }};
}

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// enters a matching state from a non-matching state.
pub struct OnEnterMatching<S: States, M: StateMatcher<S>> {
    matcher: M,
    strict: bool,
    phantom: PhantomData<S>,
}

impl<S: States, M: StateMatcher<S>> IntoConditionalScheduleLabel<OnStateEntry<S>>
    for OnEnterMatching<S, M>
{
    fn into_conditional_schedule_label(self) -> (OnStateEntry<S>, Option<super::BoxedCondition>) {
        let matcher = self.matcher;
        let matcher: BoxedCondition = if self.strict {
            Box::new(IntoSystem::into_system(
                move |next: Res<State<S>>, previous: Option<Res<PreviousState<S>>>| match previous {
                    Some(previous) => matcher.match_state(&next) && (next.0 != previous.0),
                    None => matcher.match_state(&next),
                },
            ))
        } else {
            Box::new(IntoSystem::into_system(
                move |next: Res<State<S>>, previous: Option<Res<PreviousState<S>>>| match previous {
                    Some(previous) => matcher.match_state(&next) && !matcher.match_state(&previous),
                    None => matcher.match_state(&next),
                },
            ))
        };

        (OnStateEntry::<S>(PhantomData::<S>), Some(matcher))
    }
}

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// exits this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: States>(pub S);

impl<S: States> OnExit<S> {
    /// Exiting a matching state to a state that doesn't match
    pub fn matching<M: StateMatcher<S>>(matcher: M) -> OnExitMatching<S, M> {
        OnExitMatching {
            matcher,
            strict: false,
            phantom: PhantomData::<S>,
        }
    }

    /// Exiting a matching state regardless of what the next state is
    pub fn matching_strict<M: StateMatcher<S>>(matcher: M) -> OnExitMatching<S, M> {
        OnExitMatching {
            matcher,
            strict: true,
            phantom: PhantomData::<S>,
        }
    }

    /// Entering a state that matches the matcher from a state that doesn't
    pub fn match_function<F: Fn(&S) -> bool + 'static + Send + Sync>(
        matcher: F,
    ) -> OnExitMatching<S, StateMatcherFunction<S>> {
        let matcher = matcher.into_state_matcher();
        OnExitMatching {
            matcher,
            strict: false,
            phantom: PhantomData::<S>,
        }
    }

    /// Entering a state that matches the matcher from a state that doesn't
    pub fn match_function_strict<F: Fn(&S) -> bool + 'static + Send + Sync>(
        matcher: F,
    ) -> OnExitMatching<S, StateMatcherFunction<S>> {
        let matcher = matcher.into_state_matcher();
        OnExitMatching {
            matcher,
            strict: true,
            phantom: PhantomData::<S>,
        }
    }
}

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// exits a matching state without entering another matching state.
pub struct OnExitMatching<S: States, M: StateMatcher<S>> {
    matcher: M,
    strict: bool,
    phantom: PhantomData<S>,
}

impl<S: States, M: StateMatcher<S>> IntoConditionalScheduleLabel<OnStateExit<S>>
    for OnExitMatching<S, M>
{
    fn into_conditional_schedule_label(self) -> (OnStateExit<S>, Option<super::BoxedCondition>) {
        let matcher = self.matcher;

        let matcher: BoxedCondition = if self.strict {
            Box::new(IntoSystem::into_system(
                move |next: Option<Res<State<S>>>, previous: Res<PreviousState<S>>| match next {
                    Some(next) => matcher.match_state(&previous) && (next.0 != previous.0),
                    None => matcher.match_state(&previous),
                },
            ))
        } else {
            Box::new(IntoSystem::into_system(
                move |next: Option<Res<State<S>>>, previous: Res<PreviousState<S>>| match next {
                    Some(next) => !matcher.match_state(&next) && matcher.match_state(&previous),
                    None => matcher.match_state(&previous),
                },
            ))
        };

        (OnStateExit::<S>(PhantomData::<S>), Some(matcher))
    }
}

/// A schedule for every time a state of type S is entered
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnStateEntry<S: States>(PhantomData<S>);

/// A schedule for every time a state of type S is exited
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnStateExit<S: States>(PhantomData<S>);

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
/// queue a transition in the [`NextState<S>`] resource, and it will be applied by the next
/// [`apply_state_transition::<S>`] system.
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

impl<S: States + PartialEq> PartialEq<S> for State<S> {
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

#[derive(Resource, Default, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default)
)]
struct PreviousState<S: States>(S);

impl<S: States> PreviousState<S> {
    /// Get the current state.
    pub fn get(&self) -> &S {
        &self.0
    }
}

impl<S: States + PartialEq> PartialEq<S> for PreviousState<S> {
    fn eq(&self, other: &S) -> bool {
        self.get() == other
    }
}

impl<S: States> Deref for PreviousState<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// The next state of [`State<S>`].
///
/// To queue a transition, just set the contained value to `Some(next_state)`.
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource at the time of [`apply_state_transition`] matters.
#[derive(Resource, Default, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default)
)]
pub enum NextState<S: States> {
    /// Do not change the state.
    #[default]
    MaintainCurrent,
    /// Change the state to a specific, pre-determined value
    StateValue(S),
    /// Change the state to a value determined by the given closure
    StateSetter(Arc<dyn Fn(S) -> S + Sync + Send>),
}

impl<S: States> Debug for NextState<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaintainCurrent => write!(f, "MaintainCurrent"),
            Self::StateValue(arg0) => f.debug_tuple("StateValue").field(arg0).finish(),
            Self::StateSetter(_) => write!(f, "StateSetter"),
        }
    }
}

impl<S: States> NextState<S> {
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        *self = Self::StateValue(state)
    }
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn setter(&mut self, setter: impl Fn(S) -> S + 'static + Sync + Send) {
        *self = Self::StateSetter(Arc::new(setter));
    }
}

/// If the state doesn't exist, initializes it to default runs OnEnter
pub fn initialize_state_and_enter<S: States>(world: &mut World) {
    world.insert_resource(NextState::<S>::MaintainCurrent);

    if world.contains_resource::<State<S>>() {
        return;
    }

    let default = S::default();

    world.insert_resource(State(default.clone()));

    world.try_run_schedule(OnEnter(default)).ok();
    world
        .try_run_schedule(OnStateEntry::<S>(PhantomData::<S>))
        .ok();
}

/// If the state exists, removes it and runs OnExit
pub fn remove_state_from_world<S: States>(world: &mut World) {
    world.remove_resource::<NextState<S>>();

    let Some(state) = world.get_resource::<State<S>>() else {
        return;
    };
    let state = state.0.clone();

    world.remove_resource::<State<S>>();

    world.insert_resource(PreviousState(state.clone()));

    world.try_run_schedule(OnExit(state)).ok();
    world
        .try_run_schedule(OnStateExit::<S>(PhantomData::<S>))
        .ok();
    world.remove_resource::<PreviousState<S>>();
}

/// Run the enter schedule (if it exists) for the current state.
pub fn run_enter_schedule<S: States>(world: &mut World) {
    let Some(state) = world.get_resource::<State<S>>().map(|s| s.0.clone()) else {
        return;
    };
    world.try_run_schedule(OnEnter(state)).ok();
    world
        .try_run_schedule(OnStateEntry::<S>(PhantomData::<S>))
        .ok();
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Runs the [`OnExit(exited_state)`] schedule, if it exists.
/// - Runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition), if it exists.
/// - Runs the [`OnEnter(entered_state)`] schedule, if it exists.
pub fn apply_state_transition<S: States>(world: &mut World) {
    let Some(next_state_resource) = world.get_resource::<NextState<S>>() else {
        return;
    };
    let Some(current_state) = world.get_resource::<State<S>>().map(|s| s.0.clone()) else {
        return;
    };
    let entered = match next_state_resource {
        NextState::MaintainCurrent => None,
        NextState::StateValue(v) => Some(v.clone()),
        NextState::StateSetter(f) => Some(f(current_state.clone())),
    };
    if let Some(entered) = entered {
        world.insert_resource(NextState::<S>::MaintainCurrent);
        if current_state != entered {
            world.insert_resource(PreviousState(current_state));
            let mut state_resource = world.resource_mut::<State<S>>();
            let exited = mem::replace(&mut state_resource.0, entered.clone());
            // Try to run the schedules if they exist.
            world.try_run_schedule(OnExit(exited.clone())).ok();
            world
                .try_run_schedule(OnStateExit::<S>(PhantomData::<S>))
                .ok();
            world
                .try_run_schedule(OnTransition {
                    from: exited,
                    to: entered.clone(),
                })
                .ok();
            world.try_run_schedule(OnEnter(entered)).ok();
            world
                .try_run_schedule(OnStateEntry::<S>(PhantomData::<S>))
                .ok();
            world.remove_resource::<PreviousState<S>>();
        }
    }
}
