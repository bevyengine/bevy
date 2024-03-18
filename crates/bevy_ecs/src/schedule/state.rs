use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::{collections::BTreeMap, fmt::Debug};

use crate as bevy_ecs;
use crate::event::Event;
use crate::prelude::{FromWorld, ResMut};
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::Resource;
use crate::world::World;

pub use bevy_ecs_macros::{States, SubStates};
use bevy_utils::{all_tuples, HashSet};

use self::sealed::StateSetSealed;

use super::{InternedScheduleLabel, Schedule, Schedules};

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
pub trait FreelyMutableState: States {}

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

/// The label of a [`Schedule`] that runs systems
/// to derive computed states from this one.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComputeDependantStates<S: States>(PhantomData<S>);

impl<S: States> Default for ComputeDependantStates<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// The label of a [`Schedule`] that runs the system
/// deriving a given [`ComputedStates`] or the existence of
/// a given [`SubStates`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ComputeComputedState<S: States>(PhantomData<S>);

impl<S: States> Default for ComputeComputedState<S> {
    fn default() -> Self {
        Self(Default::default())
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

/// Run the enter schedule (if it exists) for the current state.
pub fn run_enter_schedule<S: States>(world: &mut World) {
    let Some(state) = world.get_resource::<State<S>>() else {
        return;
    };
    let state = state.0.clone();
    world
        .try_run_schedule(ComputeDependantStates::<S>::default())
        .ok();
    world.try_run_schedule(OnEnter(state)).ok();
}

#[derive(Resource, Default)]
struct StateTransitionSchedules {
    dependant_schedules: BTreeMap<usize, HashSet<InternedScheduleLabel>>,
    exit_schedules: BTreeMap<usize, HashSet<InternedScheduleLabel>>,
    transition_schedules: BTreeMap<usize, HashSet<InternedScheduleLabel>>,
    enter_schedules: BTreeMap<usize, HashSet<InternedScheduleLabel>>,
}

/// Runs [state transitions](States).
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// Applies manual state transitions using [`NextState<S>`]
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ManualStateTransitions;

/// This function actually applies a state change, and registers the required
/// schedules for downstream computed states and transition schedules.
///
/// The `new_state` is an option to allow for removal - `None` will trigger the
/// removal of the `State<S>` resource from the [`World`].
fn internal_apply_state_transition<S: States>(world: &mut World, new_state: Option<S>) {
    match new_state {
        Some(entered) => {
            match world.get_resource_mut::<State<S>>() {
                // If the [`State<S>`] resource exists, and the state is not the one we are
                // entering - we need to set the new value, compute dependant states, send transition events
                // and register transition schedules.
                Some(mut state_resource) => {
                    if *state_resource != entered {
                        let exited = mem::replace(&mut state_resource.0, entered.clone());

                        world
                            .try_run_schedule(ComputeDependantStates::<S>::default())
                            .ok();

                        world.send_event(StateTransitionEvent {
                            before: Some(exited.clone()),
                            after: Some(entered.clone()),
                        });

                        let mut state_transition_schedules =
                            world.get_resource_or_insert_with(StateTransitionSchedules::default);
                        state_transition_schedules
                            .exit_schedules
                            .entry(S::DEPENDENCY_DEPTH)
                            .or_default()
                            .insert(OnExit(exited.clone()).intern());
                        state_transition_schedules
                            .transition_schedules
                            .entry(S::DEPENDENCY_DEPTH)
                            .or_default()
                            .insert(
                                OnTransition {
                                    from: exited,
                                    to: entered.clone(),
                                }
                                .intern(),
                            );
                        state_transition_schedules
                            .enter_schedules
                            .entry(S::DEPENDENCY_DEPTH)
                            .or_default()
                            .insert(OnEnter(entered.clone()).intern());
                    }
                }
                None => {
                    // If the [`State<S>`] resource does not exist, we create it, compute dependant states, send a transition event and register the `OnEnter` schedule.
                    world.insert_resource(State(entered.clone()));

                    world
                        .try_run_schedule(ComputeDependantStates::<S>::default())
                        .ok();

                    world.send_event(StateTransitionEvent {
                        before: None,
                        after: Some(entered.clone()),
                    });

                    let mut state_transition_schedules =
                        world.get_resource_or_insert_with(StateTransitionSchedules::default);
                    state_transition_schedules
                        .enter_schedules
                        .entry(S::DEPENDENCY_DEPTH)
                        .or_default()
                        .insert(OnEnter(entered.clone()).intern());
                }
            };
        }
        None => {
            // We first remove the [`State<S>`] resource, and if one existed we compute dependant states, send a transition event and run the `OnExit` schedule.
            if let Some(resource) = world.remove_resource::<State<S>>() {
                world
                    .try_run_schedule(ComputeDependantStates::<S>::default())
                    .ok();

                world.send_event(StateTransitionEvent {
                    before: Some(resource.get().clone()),
                    after: None,
                });

                let mut state_transition_schedules =
                    world.get_resource_or_insert_with(StateTransitionSchedules::default);
                state_transition_schedules
                    .exit_schedules
                    .entry(S::DEPENDENCY_DEPTH)
                    .or_default()
                    .insert(OnExit(resource.0).intern());
            }
        }
    }
}

fn prepare_state_transitions(world: &mut World) {
    world.insert_resource(StateTransitionSchedules::default());
}

fn execute_state_transition_schedules(world: &mut World) {
    if let Some(schedules) = world.remove_resource::<StateTransitionSchedules>() {
        for (_, schedules) in schedules.exit_schedules.into_iter().rev() {
            for schedule in schedules {
                let _ = world.try_run_schedule(schedule);
            }
        }

        for (_, schedules) in schedules.transition_schedules {
            for schedule in schedules {
                let _ = world.try_run_schedule(schedule);
            }
        }

        for (_, schedules) in schedules.enter_schedules {
            for schedule in schedules {
                let _ = world.try_run_schedule(schedule);
            }
        }
    }
}

fn execute_state_transitions(world: &mut World) {
    prepare_state_transitions(world);
    let _ = world.try_run_schedule(ManualStateTransitions);
    while let Some((_, compute)) = world
        .get_resource_mut::<StateTransitionSchedules>()
        .and_then(|mut schedules| schedules.dependant_schedules.pop_first())
    {
        for schedule in compute {
            let _ = world.try_run_schedule(schedule);
        }
    }
    execute_state_transition_schedules(world);
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
    schedule.add_systems(execute_state_transitions);
    schedules.insert(schedule);
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
pub fn apply_state_transition<S: FreelyMutableState>(world: &mut World) {
    // We want to check if the State and NextState resources exist
    let (Some(next_state_resource), Some(current_state)) = (
        world.get_resource::<NextState<S>>(),
        world.get_resource::<State<S>>(),
    ) else {
        return;
    };

    match next_state_resource {
        NextState::Pending(new_state) => {
            if new_state != current_state.get() {
                let new_state = new_state.clone();
                internal_apply_state_transition(world, Some(new_state));
            }
        }
        NextState::Unchanged => {
            // This is the default value, so we don't need to re-insert the resource
            return;
        }
    }

    world.insert_resource(NextState::<S>::Unchanged);
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
    fn register_state_compute_systems_in_schedule(schedules: &mut Schedules) {
        Self::SourceStates::register_compute_systems_for_dependent_state::<Self>(schedules);
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
        schedules: &mut Schedules,
    );

    /// Sets up the systems needed to compute whether `T` exists whenever any `State` in this
    /// `StateSet` is changed.
    fn register_state_exist_systems_in_schedule<T: SubStates<SourceStates = Self>>(
        schedules: &mut Schedules,
    );
}

/// The [`InnnerStateSet`] trait is used to isolate [`ComputedStates`] & [`SubStates`] from
/// needing to use only [`Option<S>`] via the (removed) StateSet::OptionalStateSet associated type.
/// 
/// Originally, that was done because [`State<S>`] resources can be removed from the world,
/// and we do not want our systems panicing when they attempt to compute based on a removed/missing state.
/// 
/// But beyond that - some [`ComputedStates`]'s might need to exist in different states based on the existance
/// of other states. So we needed the ability to use[`Option<S>`] when appropriate.
/// 
/// The isolation works because it is implemented for both S & [`Option<S>`], and has the [`RawState`] associated type
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
        schedules: &mut Schedules,
    ) {
        {
            let system = |world: &mut World| {
                let state_set = world.get_resource::<State<S::RawState>>();
                let new_state = if let Some(state_set) = S::convert_to_usable_state(state_set) {
                    T::compute(state_set)
                } else {
                    None
                };
                internal_apply_state_transition(world, new_state);
            };
            let label = ComputeComputedState::<T>::default();
            schedules.insert({
                let mut schedule = Schedule::new(label);
                schedule.add_systems(system);
                schedule
            });
        }

        {
            let system = |mut transitions: ResMut<StateTransitionSchedules>| {
                transitions
                    .dependant_schedules
                    .entry(<T as InnerStateSet>::DEPENDENCY_DEPTH)
                    .or_default()
                    .insert(ComputeComputedState::<T>::default().intern());
            };
            let label = ComputeDependantStates::<S::RawState>::default();
            match schedules.get_mut(label.clone()) {
                Some(schedule) => {
                    schedule.add_systems(system);
                }
                None => {
                    let mut schedule = Schedule::new(label);
                    schedule.add_systems(system);
                    schedules.insert(schedule);
                }
            }
        }
    }

    fn register_state_exist_systems_in_schedule<T: SubStates<SourceStates = Self>>(
        schedules: &mut Schedules,
    ) {
        {
            let system = |world: &mut World| {
                let state_set = world.get_resource::<State<S::RawState>>();
                let new_state = if let Some(state_set) = S::convert_to_usable_state(state_set) {
                    T::exists(state_set)
                } else {
                    None
                };
                match new_state {
                    Some(value) => {
                        if !world.contains_resource::<State<T>>() {
                            internal_apply_state_transition(world, Some(value));
                        }
                    }
                    None => internal_apply_state_transition::<T>(world, None),
                };
            };
            let label = ComputeComputedState::<T>::default();
            schedules.insert({
                let mut schedule = Schedule::new(label);
                schedule.add_systems(system);
                schedule
            });
        }

        {
            let system = |mut transitions: ResMut<StateTransitionSchedules>| {
                transitions
                    .dependant_schedules
                    .entry(T::DEPENDENCY_DEPTH)
                    .or_default()
                    .insert(ComputeComputedState::<T>::default().intern());
            };
            let label = ComputeDependantStates::<S::RawState>::default();
            match schedules.get_mut(label.clone()) {
                Some(schedule) => {
                    schedule.add_systems(system);
                }
                None => {
                    let mut schedule = Schedule::new(label);
                    schedule.add_systems(system);
                    schedules.insert(schedule);
                }
            }
        }
    }
}
/// Trait defining a state that is automatically derived from other [`States`].
///
/// A Sub State is a state that exists only when the source state meet certain conditions,
/// but unlike [`ComputedStates`] - while they exist they can be manually modified.
///
/// The [`StateSet`] is passed into the `exist` method whenever one of them changes, and the
/// result is used to handle it's existence. If the result is `Some(Self)`, and the state doesn't exist,
/// the state is set to the provided value. If it is `None`, the state is removed. Otherwise - the computation
/// is not used to impact the state's value at all.
///
/// The default approach to creating [`SubStates`] is using the derive macro, and defining a single source state
/// and value to determine it's existence. Note that this approach requires implementing [`Default`] as well.
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
/// In more complex situations, the recommendation is to use an intermediary compute state, like so:
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
/// However, you can also manually implement them. Note that you'll also need to manually implement the `States` & `FreelyMutableState` traits.
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
    fn register_state_exist_systems_in_schedules(schedules: &mut Schedules) {
        Self::SourceStates::register_state_exist_systems_in_schedule::<Self>(schedules);
    }
}

macro_rules! impl_state_set_sealed_tuples {
    ($(($param: ident, $val: ident)), *) => {
        impl<$($param: InnerStateSet),*> StateSetSealed for  ($($param,)*) {}

        impl<$($param: InnerStateSet),*> StateSet for  ($($param,)*) {

            const SET_DEPENDENCY_DEPTH : usize = $($param::DEPENDENCY_DEPTH +)* 0;

            fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(schedules: &mut Schedules) {
                {
                    let system =  |world: &mut World| {
                        let ($($val),*,) = ($(world.get_resource::<State<$param::RawState>>()),*,);

                        let new_state = if let ($(Some($val)),*,) = ($($param::convert_to_usable_state($val)),*,) {
                            T::compute(($($val),*, ))
                        } else {
                            None
                        };
                        internal_apply_state_transition(world, new_state);
                    };

                    let label = ComputeComputedState::<T>::default();
                    schedules.insert({
                        let mut schedule = Schedule::new(label);
                        schedule.add_systems(system);
                        schedule
                    });
                }

                {
                    let system = |mut transitions: ResMut<StateTransitionSchedules>| {
                        transitions.dependant_schedules.entry(<T as InnerStateSet>::DEPENDENCY_DEPTH).or_default().insert(ComputeComputedState::<T>::default().intern());
                    };

                    $(let label = ComputeDependantStates::<$param::RawState>::default();
                    match schedules.get_mut(label.clone()) {
                        Some(schedule) => {
                            schedule.add_systems(system);
                        },
                        None => {
                            let mut schedule = Schedule::new(label);
                            schedule.add_systems(system);
                            schedules.insert(schedule);
                        },
                    })*
                }
            }


            fn register_state_exist_systems_in_schedule<T: SubStates<SourceStates = Self>>(schedules: &mut Schedules) {
                {
                    let system =  |world: &mut World| {
                        let ($($val),*,) = ($(world.get_resource::<State<$param::RawState>>()),*,);

                        let new_state = if let ($(Some($val)),*,) = ($($param::convert_to_usable_state($val)),*,) {
                            T::exists(($($val),*, ))
                        } else {
                            None
                        };
                        match new_state {
                            Some(value) => {
                                if !world.contains_resource::<State<T>>() {
                                    internal_apply_state_transition(world, Some(value));
                                }
                            },
                            None => internal_apply_state_transition::<T>(world, None),
                        };
                    };

                    let label = ComputeComputedState::<T>::default();
                    schedules.insert({
                        let mut schedule = Schedule::new(label);
                        schedule.add_systems(system);
                        schedule
                    });
                }

                {
                    let system = |mut transitions: ResMut<StateTransitionSchedules>| {
                        transitions.dependant_schedules.entry(T::DEPENDENCY_DEPTH).or_default().insert(ComputeComputedState::<T>::default().intern());
                    };

                    $(let label = ComputeDependantStates::<$param::RawState>::default();
                    match schedules.get_mut(label.clone()) {
                        Some(schedule) => {
                            schedule.add_systems(system);
                        },
                        None => {
                            let mut schedule = Schedule::new(label);
                            schedule.add_systems(system);
                            schedules.insert(schedule);
                        },
                    })*
                }
            }
        }
    };
}

all_tuples!(impl_state_set_sealed_tuples, 1, 15, S, s);

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::SubStates;

    use super::*;
    use crate as bevy_ecs;

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
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        TestComputedState::register_state_compute_systems_in_schedule(&mut schedules);
        let mut apply_changes = Schedule::new(ManualStateTransitions);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world);

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
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        SubState::register_state_exist_systems_in_schedules(&mut schedules);
        let mut apply_changes = Schedule::new(ManualStateTransitions);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        apply_changes.add_systems(apply_state_transition::<SubState>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world);

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
        world.init_resource::<State<SimpleState>>();
        let mut schedules = Schedules::new();
        TestComputedState::register_state_compute_systems_in_schedule(&mut schedules);
        SubStateOfComputed::register_state_exist_systems_in_schedules(&mut schedules);
        let mut apply_changes = Schedule::new(ManualStateTransitions);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        apply_changes.add_systems(apply_state_transition::<SubStateOfComputed>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world);

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
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<State<OtherState>>();

        let mut schedules = Schedules::new();

        ComplexComputedState::register_state_compute_systems_in_schedule(&mut schedules);

        let mut apply_changes = Schedule::new(ManualStateTransitions);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        apply_changes.add_systems(apply_state_transition::<OtherState>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        setup_state_transitions_in_world(&mut world);

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

    #[test]
    fn computed_state_transitions_are_produced_correctly() {
        let mut world = World::new();
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<State<SimpleState2>>();

        let mut schedules = Schedules::new();

        TestNewcomputedState::register_state_compute_systems_in_schedule(&mut schedules);

        let mut apply_changes = Schedule::new(ManualStateTransitions);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        apply_changes.add_systems(apply_state_transition::<SimpleState2>);
        schedules.insert(apply_changes);

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

        world.insert_resource(schedules);

        world.init_resource::<ComputedStateTransitionCounter>();

        setup_state_transitions_in_world(&mut world);

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
