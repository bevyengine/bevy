use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use super::Condition;
use crate as bevy_ecs;
use crate::prelude::Res;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::{IntoSystem, Resource};
use crate::world::World;
pub use bevy_ecs_macros::{entering, exiting, state_matches, transitioning, StateMatcher, States};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;

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
/// States are commonly defined as simple enums, with the [`States`] derive macro.
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
///
/// However, states can also be structs:
///
/// ```rust
/// use bevy_ecs::prelude::States;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// struct Level(u32);
/// ```
///
/// Or more complex structures with multiple layers:
/// This can be useful for complex state machines to ensure that invalid states are unrepresentable.
///
/// ```rust {
/// use bevy_ecs::prelude::States;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum AppState {
///     #[default]
///     Loading,
///     MainMenu,
///     Playing {
///        paused: bool,
///        game_mode: GameMode,
///     }
/// }
///
/// // Note that we're *not* deriving `States` for `GameMode` here:
/// // we don't want to be able to set the game mode without also setting the `AppState::Playing` state.
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
/// enum GameMode {
///     #[default]
///     SinglePlayer,
///     Tutorial,
///     MultiPlayer,
/// }
/// ```
///
///

pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug + Default {
    /// Provides a default instance of the [`OnStateEntry`] schedule label for this state type
    fn entering() -> OnStateEntry<Self> {
        OnStateEntry::default()
    }

    /// Provides a default instance of the [`OnStateExit`] schedule label for this state type
    fn exiting() -> OnStateExit<Self> {
        OnStateExit::default()
    }

    /// Provides a default instance of the [`OnStateTransition`] schedule label for this state type
    fn transitioning() -> OnStateTransition<Self> {
        OnStateTransition::default()
    }
}

/// A marker struct denoting that the StateMatcher
/// is on a [`States`] object directly, and relies
/// on it's Eq implementation
pub struct StatesInherentMatcher;

/// A marker struct denoting that the StateMatcher is derived from a function or closure
/// that takes in a single state reference:
///
/// `Fn(&S) -> bool`
pub struct SimpleFnMatcher;
/// A marker struct denoting that the StateMatcher is derived from a function or closure
/// that takes in a single state reference and an optional secondary state reference:
///
/// `Fn(&S, Option<&S>) -> bool`
pub struct SimpleTransitionFnMatcher;
/// A marker struct denoting that the StateMatcher is derived from a function or closure
/// that takes in a single state reference and an optional secondary state reference:
///
/// `Fn(Option<&S>, Option<&S>) -> bool`
pub struct SimpleOptionalTransitionFnMatcher;
/// A marker struct denoting that the StateMatcher is derived from a function or closure
/// that takes in a single state reference and an optional secondary state reference:
///
/// `Fn(&S, Option<&S>) -> MatchesStateTransition`
pub struct TransitionFnMatcher;
/// A marker struct denoting that the StateMatcher is derived from a function or closure
/// that takes in a single state reference and an optional secondary state reference:
///
/// `Fn(Option<&S>, Option<&S>) -> MatchesStateTransition`
pub struct OptionalTransitionFnMatcher;

/// An enum describing the possible result of a state transition match.
///
/// If you are just matching a single state, treat `TransitionMatches` and `MainMatches` as truthy
/// If you are matching a transition between two states, only `TransitionMatches` should be considered truthy
#[derive(Eq, Clone, Copy, PartialEq, Debug)]
pub enum MatchesStateTransition {
    /// This means the transition is considered valid by the matcher.
    TransitionMatches,
    /// This means that the Main value matches, but the transition as a whole might not. Useful for inferring the `match_state` function in a matcher, handling `every` macros.
    MainMatches,
    /// This means that neither the Main value doesn't match, and the transition is invalid.
    NoMatch,
}

impl From<bool> for MatchesStateTransition {
    fn from(value: bool) -> Self {
        match value {
            true => MatchesStateTransition::TransitionMatches,
            false => MatchesStateTransition::NoMatch,
        }
    }
}

/// Types that can match world-wide states.
pub trait StateMatcher<S: States, Marker = ()>: Send + Sync + Sized + 'static {
    /// Check whether to match with the current state
    fn match_state(&self, state: &S) -> bool {
        self.match_state_transition(Some(state), None) != MatchesStateTransition::NoMatch
    }

    /// Check whether to match a state transition
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition;
}

impl<S: States> StateMatcher<S, StatesInherentMatcher> for S {
    fn match_state_transition(&self, state: Option<&S>, _: Option<&S>) -> MatchesStateTransition {
        let Some(state) = state else {
            return false.into();
        };
        (self == state).into()
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S) -> bool> StateMatcher<S, SimpleFnMatcher> for F {
    fn match_state_transition(&self, state: Option<&S>, _: Option<&S>) -> MatchesStateTransition {
        let Some(state) = state else {
            return false.into();
        };
        self(state).into()
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> MatchesStateTransition>
    StateMatcher<S, TransitionFnMatcher> for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let Some(main) = main else {
            return false.into();
        };
        self(main, secondary)
    }
}

impl<
        S: States,
        F: 'static + Send + Sync + Fn(Option<&S>, Option<&S>) -> MatchesStateTransition,
    > StateMatcher<S, OptionalTransitionFnMatcher> for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        self(main, secondary)
    }
}
impl<S: States, F: 'static + Send + Sync + Fn(&S, Option<&S>) -> bool>
    StateMatcher<S, SimpleTransitionFnMatcher> for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        let Some(main) = main else {
            return false.into();
        };
        if !self(main, None) {
            return false.into();
        }
        match self(main, secondary) {
            true => MatchesStateTransition::TransitionMatches,
            false => MatchesStateTransition::MainMatches,
        }
    }
}

impl<S: States, F: 'static + Send + Sync + Fn(Option<&S>, Option<&S>) -> bool>
    StateMatcher<S, SimpleOptionalTransitionFnMatcher> for F
{
    fn match_state_transition(
        &self,
        main: Option<&S>,
        secondary: Option<&S>,
    ) -> MatchesStateTransition {
        if !self(main, None) {
            return false.into();
        }
        match self(main, secondary) {
            true => MatchesStateTransition::TransitionMatches,
            false => MatchesStateTransition::MainMatches,
        }
    }
}
/// Get a [`Condition`] for running whenever `MainResource<S>` matches regardless of
/// whether `SecondaryResource<S>` matches, so long as they are not identical
fn run_condition_on_match<
    MainResource: crate::prelude::Resource + Deref<Target = S>,
    SecondaryResource: crate::prelude::Resource + Deref<Target = S>,
    S: States,
    M,
>(
    matcher: impl StateMatcher<S, M>,
) -> impl Condition<()> {
    IntoSystem::into_system(
        move |main: Option<Res<MainResource>>, secondary: Option<Res<SecondaryResource>>| {
            let main = main.as_ref().map(|v| v.as_ref().deref());
            let secondary = secondary.as_ref().map(|v| v.as_ref().deref());

            if let (Some(main), Some(secondary)) = (main, secondary) {
                if main == secondary {
                    return false;
                }
            }

            let result = matcher.match_state_transition(main, secondary);
            result == MatchesStateTransition::TransitionMatches
        },
    )
}

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

/// A schedule for every time a state of type S is entered
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct OnStateEntry<S: States>(PhantomData<S>);

/// A schedule for every time a state of type S is changed
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct OnStateTransition<S: States>(PhantomData<S>);

/// A schedule for every time a state of type S is exited
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct OnStateExit<S: States>(PhantomData<S>);

impl<S: States> OnStateEntry<S> {
    /// Get a `ConditionalScheduleLabel` for whenever we enter a matching `S`
    /// from a non-matching `S`
    ///
    /// designed to be used via [`entering!`] macro
    pub fn matching<Marker>(self, matcher: impl StateMatcher<S, Marker>) -> impl Condition<()> {
        run_condition_on_match::<State<S>, PreviousState<S>, _, _>(matcher)
    }
}

impl<S: States> OnStateExit<S> {
    /// Get a `ConditionalScheduleLabel` for whenever we exit a matching `S`
    /// to a non-matching `S`
    ///
    /// designed to be used via [`exiting!`] macro
    pub fn matching<Marker>(self, matcher: impl StateMatcher<S, Marker>) -> impl Condition<()> {
        run_condition_on_match::<PreviousState<S>, State<S>, _, _>(matcher)
    }
}

impl<S: States> OnStateTransition<S> {
    /// Get a `ConditionalScheduleLabel` for whenever we move from a state that matches `from` and not `to`,
    /// to a state that matches `to` and not `from`
    pub fn matching<Marker1, Marker2>(
        self,
        from: impl StateMatcher<S, Marker1>,
        to: impl StateMatcher<S, Marker2>,
    ) -> impl Condition<()> {
        run_condition_on_match::<State<S>, PreviousState<S>, _, _>(from).and_then(
            run_condition_on_match::<PreviousState<S>, State<S>, _, _>(to),
        )
    }
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
    fn get(&self) -> &S {
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
#[derive(Resource, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default)
)]
pub enum NextState<S: States> {
    /// Do not change the state.
    #[default]
    Keep,
    /// Change the state to a specific, pre-determined value
    Value(S),
    /// Change the state to a value determined by the given closure
    Setter(Box<dyn Fn(S) -> S + Sync + Send>),
}

impl<S: States> Debug for NextState<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keep => write!(f, "Keep"),
            Self::Value(arg0) => f.debug_tuple("Value").field(arg0).finish(),
            Self::Setter(_) => write!(f, "Setter"),
        }
    }
}

impl<S: States> NextState<S> {
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        *self = Self::Value(state);
    }
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn setter(&mut self, setter: impl Fn(S) -> S + 'static + Sync + Send) {
        *self = Self::Setter(Box::new(setter));
    }
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
        NextState::Keep => None,
        NextState::Value(v) => Some(v.clone()),
        NextState::Setter(f) => Some(f(current_state.clone())),
    };
    if let Some(entered) = entered {
        if current_state != entered {
            world.insert_resource(PreviousState(current_state));
            let mut state_resource = world.resource_mut::<State<S>>();
            let exited = mem::replace(&mut state_resource.0, entered.clone());
            // Try to run the schedules if they exist.
            world.try_run_schedule(OnExit(exited.clone())).ok();
            world.try_run_schedule(OnStateExit::<S>::default()).ok();
            world
                .try_run_schedule(OnTransition {
                    from: exited,
                    to: entered.clone(),
                })
                .ok();
            world
                .try_run_schedule(OnStateTransition::<S>::default())
                .ok();
            world.try_run_schedule(OnEnter(entered)).ok();
            world.try_run_schedule(OnStateEntry::<S>::default()).ok();
            world.remove_resource::<PreviousState<S>>();
        }

        world.insert_resource(NextState::<S>::Keep);
    }
}
