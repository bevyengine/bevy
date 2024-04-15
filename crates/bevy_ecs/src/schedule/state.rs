use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use crate as bevy_ecs;
use crate::event::{Event, EventReader, EventWriter};
use crate::prelude::{FromWorld, Local, Res, ResMut};
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::{Commands, In, IntoSystem, Resource};
use crate::world::World;

use bevy_ecs_macros::SystemSet;
pub use bevy_ecs_macros::{States, SubStates};
use bevy_utils::all_tuples;

use self::sealed::StateSetSealed;

use super::{InternedScheduleLabel, IntoSystemConfigs, IntoSystemSetConfigs, Schedule, Schedules};

/// Types that can define world-wide states in a finite-state machine.
///
/// The [`Default`] trait defines the starting state.
/// Multiple states can be defined for the same world,
/// allowing you to classify the state of the world across orthogonal dimensions.
/// You can access the current state of type `T` with the [`State<T>`] resource,
/// and the queued state with the [`NextState<T>`] resource.
///
/// State transitions typically occur in the [`OnEnter<T::Variant>`] and [`OnExit<T::Variant>`] schedules,
/// which can be run by triggering the [`StateTransition`] schedule.
///
/// Types used as [`ComputedStates`] do not need to and should not derive [`States`].
/// [`ComputedStates`] are not intended to be manually mutated, but this functionality is provided
/// by the [`States`] derive and the associated [`FreelyMutableState`] trait.
///
/// # Example
///
/// ```
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
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {
    /// How many other states this state depends on.
    /// Used to help order transitions and de-duplicate [`ComputedStates`], as well as prevent cyclical
    /// `ComputedState` dependencies.
    const DEPENDENCY_DEPTH: usize = 1;
}

/// This trait allows a state to be mutated directly using the [`NextState<S>`] resource.
///
/// This is in contrast with [`ComputedStates`], which do not allow modification - they are
/// automatically derived.
///
/// It is implemented as part of the [`States`] derive, but can also be added manually.
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

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`] and [`OnExit(state)`]).
///
/// The current state value can be accessed through this resource. To *change* the state,
/// queue a transition in the [`NextState<S>`] resource, and it will be applied by the next
/// [`apply_state_transition::<S>`] system.
///
/// The starting state is defined via the [`Default`] implementation for `S`.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// fn game_logic(game_state: Res<State<GameState>>) {
///     match game_state.get() {
///         GameState::InGame => {
///             // Run game logic here...
///         },
///         _ => {},
///     }
/// }
/// ```
#[derive(Resource, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
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

impl<S: States + FromWorld> FromWorld for State<S> {
    fn from_world(world: &mut World) -> Self {
        Self(S::from_world(world))
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
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource at the time of [`apply_state_transition`] matters.
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// fn start_game(mut next_game_state: ResMut<NextState<GameState>>) {
///     next_game_state.set(GameState::InGame);
/// }
/// ```
#[derive(Resource, Debug, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
)]
pub enum NextState<S: FreelyMutableState> {
    /// No state transition is pending
    #[default]
    Unchanged,
    /// There is a pending transition for state `S`
    Pending(S),
}

impl<S: FreelyMutableState> NextState<S> {
    /// Tentatively set a pending state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        *self = Self::Pending(state);
    }

    /// Remove any pending changes to [`State<S>`]
    pub fn reset(&mut self) {
        *self = Self::Unchanged;
    }
}

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

/// Runs [state transitions](States).
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// Applies manual state transitions using [`NextState<S>`]
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
enum StateTransitionSteps {
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
    fn apply() -> Self {
        Self(PhantomData)
    }
}

/// This function actually applies a state change, and registers the required
/// schedules for downstream computed states and transition schedules.
///
/// The `new_state` is an option to allow for removal - `None` will trigger the
/// removal of the `State<S>` resource from the [`World`].
fn internal_apply_state_transition<S: States>(
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
        match schedules.get_mut(startup) {
            Some(schedule) => {
                schedule.add_systems(|world: &mut World| {
                    let _ = world.try_run_schedule(StateTransition);
                });
            }
            None => {
                let mut schedule = Schedule::new(startup);

                schedule.add_systems(|world: &mut World| {
                    let _ = world.try_run_schedule(StateTransition);
                });

                schedules.insert(schedule);
            }
        }
    }
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Sends a relevant [`StateTransitionEvent`]
/// - Runs the [`OnExit(exited_state)`] schedule, if it exists.
/// - Runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition), if it exists.
/// - Derive any dependant states through the [`ComputeDependantStates::<S>`] schedule, if it exists.
/// - Runs the [`OnEnter(entered_state)`] schedule, if it exists.
///
/// If the [`State<S>`] resource does not exist, it does nothing. Removing or adding states
/// should be done at App creation or at your own risk.
pub fn apply_state_transition<S: FreelyMutableState>(
    event: EventWriter<StateTransitionEvent<S>>,
    commands: Commands,
    current_state: Option<ResMut<State<S>>>,
    next_state: Option<ResMut<NextState<S>>>,
) {
    // We want to check if the State and NextState resources exist
    let (Some(next_state_resource), Some(current_state)) = (next_state, current_state) else {
        return;
    };

    match next_state_resource.as_ref() {
        NextState::Pending(new_state) => {
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
        NextState::Unchanged => {
            // This is the default value, so we don't need to re-insert the resource
            return;
        }
    }

    *next_state_resource.value = NextState::<S>::Unchanged;
}

fn should_run_transition<S: States, T: ScheduleLabel>(
    first: Local<bool>,
    res: Option<Res<State<S>>>,
    mut event: EventReader<StateTransitionEvent<S>>,
) -> (Option<StateTransitionEvent<S>>, PhantomData<T>) {
    if !*first.0 {
        *first.0 = true;
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

fn run_enter<S: States>(
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

fn run_exit<S: States>(
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

fn run_transition<S: States>(
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

/// Trait defining a state that is automatically derived from other [`States`].
///
/// A Computed State is a state that is deterministically derived from a set of `SourceStates`.
/// The [`StateSet`] is passed into the `compute` method whenever one of them changes, and the
/// result becomes the state's value.
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// /// Computed States require some state to derive from
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame { paused: bool }
/// }
///
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct InGame;
///
/// impl ComputedStates for InGame {
///     /// We set the source state to be the state, or a tuple of states,
///     /// we want to depend on. You can also wrap each state in an Option,
///     /// if you want the computed state to execute even if the state doesn't
///     /// currently exist in the world.
///     type SourceStates = AppState;
///
///     /// We then define the compute function, which takes in
///     /// your SourceStates
///     fn compute(sources: AppState) -> Option<Self> {
///         match sources {
///             /// When we are in game, we want to return the InGame state
///             AppState::InGame { .. } => Some(InGame),
///             /// Otherwise, we don't want the `State<InGame>` resource to exist,
///             /// so we return None.
///             _ => None
///         }
///     }
/// }
/// ```
///
/// you can then add it to an App, and from there you use the state as normal
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// # struct App;
/// # impl App {
/// #   fn new() -> Self { App }
/// #   fn init_state<S>(&mut self) -> &mut Self {self}
/// #   fn add_computed_state<S>(&mut self) -> &mut Self {self}
/// # }
/// # struct AppState;
/// # struct InGame;
///
///     App::new()
///         .init_state::<AppState>()
///         .add_computed_state::<InGame>();
/// ```
pub trait ComputedStates: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {
    /// The set of states from which the [`Self`] is derived.
    ///
    /// This can either be a single type that implements [`States`], an Option of a type
    /// that implements [`States`], or a tuple
    /// containing multiple types that implement [`States`] or Optional versions of them.
    ///
    /// For example, `(MapState, EnemyState)` is valid, as is `(MapState, Option<EnemyState>)`
    type SourceStates: StateSet;

    /// This function gets called whenever one of the [`SourceStates`](Self::SourceStates) changes.
    /// The result is used to set the value of [`State<Self>`].
    ///
    /// If the result is [`None`], the [`State<Self>`] resource will be removed from the world.
    fn compute(sources: Self::SourceStates) -> Option<Self>;

    /// This function sets up systems that compute the state whenever one of the [`SourceStates`](Self::SourceStates)
    /// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
    /// used.
    fn register_state_compute_systems_in_schedule(schedule: &mut Schedule) {
        Self::SourceStates::register_compute_systems_for_dependent_state::<Self>(schedule);
    }
}

impl<S: ComputedStates> States for S {
    const DEPENDENCY_DEPTH: usize = S::SourceStates::SET_DEPENDENCY_DEPTH + 1;
}

mod sealed {
    /// Sealed trait used to prevent external implementations of [`StateSet`](super::StateSet).
    pub trait StateSetSealed {}
}

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
    fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(
        schedule: &mut Schedule,
    );

    /// Sets up the systems needed to compute whether `T` exists whenever any `State` in this
    /// `StateSet` is changed.
    fn register_state_exist_systems_in_schedule<T: SubStates<SourceStates = Self>>(
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

    fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(
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

    fn register_state_exist_systems_in_schedule<T: SubStates<SourceStates = Self>>(
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
                    T::exists(state_set)
                } else {
                    None
                };

            match new_state {
                Some(value) => {
                    if current_state.is_none() {
                        internal_apply_state_transition(
                            event,
                            commands,
                            current_state,
                            Some(value),
                        );
                    }
                }
                None => {
                    internal_apply_state_transition(event, commands, current_state, None);
                }
            };
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
/// Trait defining a state whose value is automatically computed from other [`States`].
///
/// A sub-state is a state that exists only when the source state meet certain conditions,
/// but unlike [`ComputedStates`] - while they exist they can be manually modified.
///
/// The [`StateSet`] is passed into the `exist` method whenever one of them changes, and the
/// result is used to handle it's existence. If the result is `Some(Self)`, and the state doesn't exist,
/// the state is set to the provided value. If it is `None`, the state is removed. Otherwise - the computation
/// is not used to impact the state's value at all.
///
/// The default approach to creating [`SubStates`] is using the derive macro, and defining a single source state
/// and value to determine it's existence.
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame
/// }
///
///
/// #[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// #[source(AppState = AppState::InGame)]
/// enum GamePhase {
///     #[default]
///     Setup,
///     Battle,
///     Conclusion
/// }
/// ```
///
/// you can then add it to an App, and from there you use the state as normal:
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// # struct App;
/// # impl App {
/// #   fn new() -> Self { App }
/// #   fn init_state<S>(&mut self) -> &mut Self {self}
/// #   fn add_sub_state<S>(&mut self) -> &mut Self {self}
/// # }
/// # struct AppState;
/// # struct GamePhase;
///
///     App::new()
///         .init_state::<AppState>()
///         .add_sub_state::<GamePhase>();
/// ```
///
/// In more complex situations, the recommendation is to use an intermediary computed state, like so:
///
/// ```
/// # use bevy_ecs::prelude::*;
///
/// /// Computed States require some state to derive from
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame { paused: bool }
/// }
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct InGame;
///
/// impl ComputedStates for InGame {
///     /// We set the source state to be the state, or set of states,
///     /// we want to depend on. Any of the states can be wrapped in an Option.
///     type SourceStates = Option<AppState>;
///
///     /// We then define the compute function, which takes in the AppState
///     fn compute(sources: Option<AppState>) -> Option<Self> {
///         match sources {
///             /// When we are in game, we want to return the InGame state
///             Some(AppState::InGame { .. }) => Some(InGame),
///             /// Otherwise, we don't want the `State<InGame>` resource to exist,
///             /// so we return None.
///             _ => None
///         }
///     }
/// }
///
/// #[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// #[source(InGame = InGame)]
/// enum GamePhase {
///     #[default]
///     Setup,
///     Battle,
///     Conclusion
/// }
/// ```
///
/// However, you can also manually implement them. If you do so, you'll also need to manually implement the `States` & `FreelyMutableState` traits.
/// Unlike the derive, this does not require an implementation of [`Default`], since you are providing the `exists` function
/// directly.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::schedule::FreelyMutableState;
///
/// /// Computed States require some state to derive from
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame { paused: bool }
/// }
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// enum GamePhase {
///     Setup,
///     Battle,
///     Conclusion
/// }
///
/// impl SubStates for GamePhase {
///     /// We set the source state to be the state, or set of states,
///     /// we want to depend on. Any of the states can be wrapped in an Option.
///     type SourceStates = Option<AppState>;
///
///     /// We then define the compute function, which takes in the [`Self::SourceStates`]
///     fn exists(sources: Option<AppState>) -> Option<Self> {
///         match sources {
///             /// When we are in game, so we want a GamePhase state to exist, and the default is
///             /// GamePhase::Setup
///             Some(AppState::InGame { .. }) => Some(GamePhase::Setup),
///             /// Otherwise, we don't want the `State<GamePhase>` resource to exist,
///             /// so we return None.
///             _ => None
///         }
///     }
/// }
///
/// impl States for GamePhase {
///     const DEPENDENCY_DEPTH : usize = <GamePhase as SubStates>::SourceStates::SET_DEPENDENCY_DEPTH + 1;
/// }
///
/// impl FreelyMutableState for GamePhase {}
/// ```
pub trait SubStates: States + FreelyMutableState {
    /// The set of states from which the [`Self`] is derived.
    ///
    /// This can either be a single type that implements [`States`], or a tuple
    /// containing multiple types that implement [`States`], or any combination of
    /// types implementing [`States`] and Options of types implementing [`States`]
    type SourceStates: StateSet;

    /// This function gets called whenever one of the [`SourceStates`](Self::SourceStates) changes.
    /// The result is used to determine the existence of [`State<Self>`].
    ///
    /// If the result is [`None`], the [`State<Self>`] resource will be removed from the world, otherwise
    /// if the [`State<Self>`] resource doesn't exist - it will be created with the [`Some`] value.
    fn exists(sources: Self::SourceStates) -> Option<Self>;

    /// This function sets up systems that compute the state whenever one of the [`SourceStates`](Self::SourceStates)
    /// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
    /// used.
    fn register_state_exist_systems_in_schedules(schedule: &mut Schedule) {
        Self::SourceStates::register_state_exist_systems_in_schedule::<Self>(schedule);
    }
}

macro_rules! impl_state_set_sealed_tuples {
    ($(($param: ident, $val: ident, $evt: ident)), *) => {
        impl<$($param: InnerStateSet),*> StateSetSealed for  ($($param,)*) {}

        impl<$($param: InnerStateSet),*> StateSet for  ($($param,)*) {

            const SET_DEPENDENCY_DEPTH : usize = $($param::DEPENDENCY_DEPTH +)* 0;


            fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(
                schedule: &mut Schedule,
            ) {
                let system = |($(mut $evt),*,): ($(EventReader<StateTransitionEvent<$param::RawState>>),*,), event: EventWriter<StateTransitionEvent<T>>, commands: Commands, current_state: Option<ResMut<State<T>>>, ($($val),*,): ($(Option<Res<State<$param::RawState>>>),*,)| {
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

            fn register_state_exist_systems_in_schedule<T: SubStates<SourceStates = Self>>(
                schedule: &mut Schedule,
            ) {
                let system = |($(mut $evt),*,): ($(EventReader<StateTransitionEvent<$param::RawState>>),*,), event: EventWriter<StateTransitionEvent<T>>, commands: Commands, current_state: Option<ResMut<State<T>>>, ($($val),*,): ($(Option<Res<State<$param::RawState>>>),*,)| {
                    if ($($evt.is_empty())&&*) {
                        return;
                    }
                    $($evt.clear();)*

                    let new_state = if let ($(Some($val)),*,) = ($($param::convert_to_usable_state($val.as_deref())),*,) {
                        T::exists(($($val),*, ))
                    } else {
                        None
                    };
                    match new_state {
                        Some(value) => {
                            if current_state.is_none() {
                                internal_apply_state_transition(event, commands, current_state, Some(value));
                            }
                        }
                        None => {
                            internal_apply_state_transition(event, commands, current_state, None);
                        },
                    };
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

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::SubStates;

    use super::*;
    use crate as bevy_ecs;

    use crate::event::EventRegistry;

    use crate::prelude::ResMut;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum SimpleState {
        #[default]
        A,
        B(bool),
    }

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum TestComputedState {
        BisTrue,
        BisFalse,
    }

    impl ComputedStates for TestComputedState {
        type SourceStates = Option<SimpleState>;

        fn compute(sources: Option<SimpleState>) -> Option<Self> {
            sources.and_then(|source| match source {
                SimpleState::A => None,
                SimpleState::B(value) => Some(if value { Self::BisTrue } else { Self::BisFalse }),
            })
        }
    }

    #[test]
    fn computed_state_with_a_single_source_is_correctly_derived() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestComputedState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        TestComputedState::register_state_compute_systems_in_schedule(&mut apply_changes);
        SimpleState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<TestComputedState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<TestComputedState>>().0,
            TestComputedState::BisTrue
        );

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert_eq!(
            world.resource::<State<TestComputedState>>().0,
            TestComputedState::BisFalse
        );

        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<TestComputedState>>());
    }

    #[derive(SubStates, PartialEq, Eq, Debug, Default, Hash, Clone)]
    #[source(SimpleState = SimpleState::B(true))]
    enum SubState {
        #[default]
        One,
        Two,
    }

    #[test]
    fn sub_state_exists_only_when_allowed_but_can_be_modified_freely() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SubState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        SubState::register_state_exist_systems_in_schedules(&mut apply_changes);
        SimpleState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubState>>());

        world.insert_resource(NextState::Pending(SubState::Two));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(world.resource::<State<SubState>>().0, SubState::One);

        world.insert_resource(NextState::Pending(SubState::Two));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(world.resource::<State<SubState>>().0, SubState::Two);

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert!(!world.contains_resource::<State<SubState>>());
    }

    #[derive(SubStates, PartialEq, Eq, Debug, Default, Hash, Clone)]
    #[source(TestComputedState = TestComputedState::BisTrue)]
    enum SubStateOfComputed {
        #[default]
        One,
        Two,
    }

    #[test]
    fn substate_of_computed_states_works_appropriately() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestComputedState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SubStateOfComputed>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);
        TestComputedState::register_state_compute_systems_in_schedule(&mut apply_changes);
        SubStateOfComputed::register_state_exist_systems_in_schedules(&mut apply_changes);
        SimpleState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubStateOfComputed>>());

        world.insert_resource(NextState::Pending(SubStateOfComputed::Two));
        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<SubStateOfComputed>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<SubStateOfComputed>>().0,
            SubStateOfComputed::One
        );

        world.insert_resource(NextState::Pending(SubStateOfComputed::Two));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<SubStateOfComputed>>().0,
            SubStateOfComputed::Two
        );

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert!(!world.contains_resource::<State<SubStateOfComputed>>());
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    struct OtherState {
        a_flexible_value: &'static str,
        another_value: u8,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum ComplexComputedState {
        InAAndStrIsBobOrJane,
        InTrueBAndUsizeAbove8,
    }

    impl ComputedStates for ComplexComputedState {
        type SourceStates = (Option<SimpleState>, Option<OtherState>);

        fn compute(sources: (Option<SimpleState>, Option<OtherState>)) -> Option<Self> {
            match sources {
                (Some(simple), Some(complex)) => {
                    if simple == SimpleState::A
                        && (complex.a_flexible_value == "bob" || complex.a_flexible_value == "jane")
                    {
                        Some(ComplexComputedState::InAAndStrIsBobOrJane)
                    } else if simple == SimpleState::B(true) && complex.another_value > 8 {
                        Some(ComplexComputedState::InTrueBAndUsizeAbove8)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    #[test]
    fn complex_computed_state_gets_derived_correctly() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<OtherState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<ComplexComputedState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<State<OtherState>>();

        let mut schedules = Schedules::new();
        let mut apply_changes = Schedule::new(StateTransition);

        ComplexComputedState::register_state_compute_systems_in_schedule(&mut apply_changes);

        SimpleState::register_state(&mut apply_changes);
        OtherState::register_state(&mut apply_changes);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world, None);

        world.run_schedule(StateTransition);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(
            world.resource::<State<OtherState>>().0,
            OtherState::default()
        );
        assert!(!world.contains_resource::<State<ComplexComputedState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.run_schedule(StateTransition);
        assert!(!world.contains_resource::<State<ComplexComputedState>>());

        world.insert_resource(NextState::Pending(OtherState {
            a_flexible_value: "felix",
            another_value: 13,
        }));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<ComplexComputedState>>().0,
            ComplexComputedState::InTrueBAndUsizeAbove8
        );

        world.insert_resource(NextState::Pending(SimpleState::A));
        world.insert_resource(NextState::Pending(OtherState {
            a_flexible_value: "jane",
            another_value: 13,
        }));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<ComplexComputedState>>().0,
            ComplexComputedState::InAAndStrIsBobOrJane
        );

        world.insert_resource(NextState::Pending(SimpleState::B(false)));
        world.insert_resource(NextState::Pending(OtherState {
            a_flexible_value: "jane",
            another_value: 13,
        }));
        world.run_schedule(StateTransition);
        assert!(!world.contains_resource::<State<ComplexComputedState>>());
    }

    #[derive(Resource, Default)]
    struct ComputedStateTransitionCounter {
        enter: usize,
        exit: usize,
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum SimpleState2 {
        #[default]
        A1,
        B2,
    }

    #[derive(PartialEq, Eq, Debug, Hash, Clone)]
    enum TestNewcomputedState {
        A1,
        B2,
        B1,
    }

    impl ComputedStates for TestNewcomputedState {
        type SourceStates = (Option<SimpleState>, Option<SimpleState2>);

        fn compute((s1, s2): (Option<SimpleState>, Option<SimpleState2>)) -> Option<Self> {
            match (s1, s2) {
                (Some(SimpleState::A), Some(SimpleState2::A1)) => Some(TestNewcomputedState::A1),
                (Some(SimpleState::B(true)), Some(SimpleState2::B2)) => {
                    Some(TestNewcomputedState::B2)
                }
                (Some(SimpleState::B(true)), _) => Some(TestNewcomputedState::B1),
                _ => None,
            }
        }
    }

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct Startup;

    #[test]
    fn computed_state_transitions_are_produced_correctly() {
        let mut world = World::new();
        EventRegistry::register_event::<StateTransitionEvent<SimpleState>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<SimpleState2>>(&mut world);
        EventRegistry::register_event::<StateTransitionEvent<TestNewcomputedState>>(&mut world);
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<State<SimpleState2>>();
        world.init_resource::<Schedules>();

        setup_state_transitions_in_world(&mut world, Some(Startup.intern()));

        let mut schedules = world
            .get_resource_mut::<Schedules>()
            .expect("Schedules don't exist in world");
        let apply_changes = schedules
            .get_mut(StateTransition)
            .expect("State Transition Schedule Doesn't Exist");

        TestNewcomputedState::register_state_compute_systems_in_schedule(apply_changes);

        SimpleState::register_state(apply_changes);
        SimpleState2::register_state(apply_changes);

        schedules.insert({
            let mut schedule = Schedule::new(OnEnter(TestNewcomputedState::A1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.enter += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnExit(TestNewcomputedState::A1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.exit += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnEnter(TestNewcomputedState::B1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.enter += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnExit(TestNewcomputedState::B1));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.exit += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnEnter(TestNewcomputedState::B2));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.enter += 1;
            });
            schedule
        });

        schedules.insert({
            let mut schedule = Schedule::new(OnExit(TestNewcomputedState::B2));
            schedule.add_systems(|mut count: ResMut<ComputedStateTransitionCounter>| {
                count.exit += 1;
            });
            schedule
        });

        world.init_resource::<ComputedStateTransitionCounter>();

        setup_state_transitions_in_world(&mut world, None);

        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(world.resource::<State<SimpleState2>>().0, SimpleState2::A1);
        assert!(!world.contains_resource::<State<TestNewcomputedState>>());

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.insert_resource(NextState::Pending(SimpleState2::B2));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<TestNewcomputedState>>().0,
            TestNewcomputedState::B2
        );
        assert_eq!(world.resource::<ComputedStateTransitionCounter>().enter, 1);
        assert_eq!(world.resource::<ComputedStateTransitionCounter>().exit, 0);

        world.insert_resource(NextState::Pending(SimpleState2::A1));
        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<TestNewcomputedState>>().0,
            TestNewcomputedState::A1
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().enter,
            2,
            "Should Only Enter Twice"
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().exit,
            1,
            "Should Only Exit Once"
        );

        world.insert_resource(NextState::Pending(SimpleState::B(true)));
        world.insert_resource(NextState::Pending(SimpleState2::B2));
        world.run_schedule(StateTransition);
        assert_eq!(
            world.resource::<State<TestNewcomputedState>>().0,
            TestNewcomputedState::B2
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().enter,
            3,
            "Should Only Enter Three Times"
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().exit,
            2,
            "Should Only Exit Twice"
        );

        world.insert_resource(NextState::Pending(SimpleState::A));
        world.run_schedule(StateTransition);
        assert!(!world.contains_resource::<State<TestNewcomputedState>>());
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().enter,
            3,
            "Should Only Enter Three Times"
        );
        assert_eq!(
            world.resource::<ComputedStateTransitionCounter>().exit,
            3,
            "Should Only Exit Twice"
        );
    }
}
