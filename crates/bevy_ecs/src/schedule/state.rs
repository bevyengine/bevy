use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use crate as bevy_ecs;
use crate::change_detection::DetectChangesMut;
use crate::event::Event;
use crate::prelude::FromWorld;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::{ScheduleLabel, apply_deferred};
use crate::system::{Resource, Commands};
use crate::world::World;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;

pub use bevy_ecs_macros::States;

use self::sealed::StateSetSealed;

use super::{Schedules, IntoSystemConfigs, Schedule};

/// Types that can define world-wide states in a finite-state machine.
///
/// The [`Default`] trait defines the starting state.
/// Multiple states can be defined for the same world,
/// allowing you to classify the state of the world across orthogonal dimensions.
/// You can access the current state of type `T` with the [`State<T>`] resource,
/// and the queued state with the [`NextState<T>`] resource.
///
/// State transitions typically occur in the [`OnEnter<T::Variant>`] and [`OnExit<T::Variant>`] schedules,
/// which can be run via the [`apply_state_transition::<T>`] system.
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
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {}

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

/// The label of a [`Schedule`](super::Schedule) that runs systems
/// to derive states from this one.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeriveStates<S: States>(PhantomData<S>);

impl<S: States> Default for DeriveStates<S> {
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
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
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
/// If this occurs at the same time as [`RemoveState<S>`] - the removal takes priority.
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
#[derive(Resource, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default)
)]
pub struct NextState<S: States>(pub Option<S>);

impl<S: States> Default for NextState<S> {
    fn default() -> Self {
        Self(None)
    }
}

impl<S: States> NextState<S> {
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        self.0 = Some(state);
    }
}

/// Remove any state of type [`S`].
///
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource at the time of [`apply_state_transition`] matters.
/// 
/// This takes priority over [`NextState<S>`].
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
/// fn remove_game_state(mut commands: Commands) {
///     commands.init_resource::<RemoveState<S>>();
/// }
/// ```
#[derive(Resource, Debug)]
pub struct RemoveState<S: States>(PhantomData<S>);

impl<S: States> Default for RemoveState<S> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Event sent when any state transition of `S` happens.
///
/// If you know exactly what state you want to respond to ahead of time, consider [`OnEnter`], [`OnTransition`], or [`OnExit`]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Event)]
pub struct StateTransitionEvent<S: States> {
    /// the state we were in before
    pub before: S,
    /// the state we're in now
    pub after: S,
}

/// Run the enter schedule (if it exists) for the current state.
pub fn run_enter_schedule<S: States>(world: &mut World) {
    let Some(state) = world.get_resource::<State<S>>() else {
        return;
    };
    let state = state.0.clone();
    world.try_run_schedule(DeriveStates::<S>::default()).ok();
    world.try_run_schedule(OnEnter(state)).ok();
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Sends a relevant [`StateTransitionEvent`]
/// - Runs the [`OnExit(exited_state)`] schedule, if it exists.
/// - Runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition), if it exists.
/// - Derive any derived states through the [`DeriveState::<S>`] schedule, if it exists.
/// - Runs the [`OnEnter(entered_state)`] schedule, if it exists.
/// 
/// If [`RemoveState<S>`] exists in the world, even if a new state is queued, this system will instead:
/// - remove the [`RemoveState<S>`] resource
/// - remove the [`NextState<S>`] resource
/// - remove the [`State<S>`] resource
/// - Runs the [`OnExit(exited_state)`] schedule, if it exists.
/// - Derive any derived states through the [`DeriveState::<S>`] schedule, if it exists.
pub fn apply_state_transition<S: States>(world: &mut World) {
    if world.remove_resource::<RemoveState<S>>().is_some() {
        world.remove_resource::<NextState<S>>();
        let Some(resource) = world.remove_resource::<State<S>>() else {
            return;
        };
        
        world.try_run_schedule(OnExit(resource.0)).ok();
        world.try_run_schedule(DeriveStates::<S>::default()).ok();
        return;
    }
    // We want to take the `NextState` resource,
    // but only mark it as changed if it wasn't empty.
    let Some(mut next_state_resource) = world.get_resource_mut::<NextState<S>>() else {
        return;
    };

    if let Some(entered) = next_state_resource.bypass_change_detection().0.take() {
        next_state_resource.set_changed();
        match world.get_resource_mut::<State<S>>() {
            Some(mut state_resource) => {
                if *state_resource != entered {
                    let exited = mem::replace(&mut state_resource.0, entered.clone());
                    world.send_event(StateTransitionEvent {
                        before: exited.clone(),
                        after: entered.clone(),
                    });
                    // Try to run the schedules if they exist.
                    world.try_run_schedule(OnExit(exited.clone())).ok();
                    world
                        .try_run_schedule(OnTransition {
                            from: exited,
                            to: entered.clone(),
                        })
                        .ok();
                    world.try_run_schedule(DeriveStates::<S>::default()).ok();
                    world.try_run_schedule(OnEnter(entered)).ok();
                }
            }
            None => {
                world.insert_resource(State(entered.clone()));
                world.try_run_schedule(DeriveStates::<S>::default()).ok();
                world.try_run_schedule(OnEnter(entered)).ok();
            }
        };
    }
}

/// Trait defining a derived state
/// 
/// A derived state is a state implementing [`States`] that is deterministically determined from one or more other [`States`].
pub trait DerivedStates: States {
    /// The set of states from which the [`Self`] is derived.
    type SourceStates: StateSet;

    /// A function for deriving the states.
    fn derive(sources: <<Self as DerivedStates>::SourceStates as StateSet>::Optionals) -> Option<Self>;
}

mod sealed {
    pub trait StateSetSealed {}
}

/// Trait defining valid sets of states used as a source for a Derived State.
pub trait StateSet: sealed::StateSetSealed {

    /// The set of states converted into a set of optional states.
    type Optionals;

    /// A function used to register a derived state with the app schedule.
    fn generate_derivations<T: DerivedStates<SourceStates = Self>>(schedules: &mut Schedules);
}

impl<S: States> StateSetSealed for S {}

impl<S: States> StateSet for S {
    type Optionals = Option<S>;
    fn generate_derivations<T: DerivedStates<SourceStates = Self>>(schedules: &mut Schedules) {
        let system = |mut commands: Commands, state_set: Option<crate::prelude::Res<State<S>>>| {
            match T::derive(state_set.map(|v| v.0.clone())) {
                Some(updated) => {
                    commands.insert_resource(NextState(Some(updated)));
                },
                None => {
                    commands.insert_resource(RemoveState::<T>::default());
                },
            }
        };
        let label = DeriveStates::<S>::default();
        match schedules.get_mut(label.clone()) {
            Some(schedule) => {
                schedule.add_systems((system, apply_deferred, apply_state_transition::<T>).chain());
            },
            None => {
                let mut schedule = Schedule::new(label);
                schedule.add_systems((system, apply_deferred, apply_state_transition::<T>).chain());
                schedules.insert(schedule);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{*};
    use crate as bevy_ecs;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum TestState {
        #[default]
        A,
        B(bool),
    }

    #[derive(States, PartialEq, Eq, Debug, Hash, Clone)]
    enum DerivedState {
        BisTrue,
        BisFalse
    }

    impl DerivedStates for DerivedState{
        type SourceStates = TestState;

        fn derive(sources: <<Self as DerivedStates>::SourceStates as StateSet>::Optionals) -> Option<Self> {
            sources.and_then(|source| {
                match source {
                    TestState::A => None,
                    TestState::B(value) => Some(if value {
                        Self::BisTrue
                    } else {
                        Self::BisFalse
                    }),
                }
            })
        }
    }

    
    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TemporaryScheduleLabel;

    #[test]
    fn derived_state_gets_created_correctly() {
        let mut world = World::new();
        world.init_resource::<State<TestState>>();
        let mut schedules = Schedules::new();
        TestState::generate_derivations::<DerivedState>(&mut schedules);
        let mut apply_changes = Schedule::new(TemporaryScheduleLabel);
        apply_changes.add_systems(apply_state_transition::<TestState>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<TestState>>().0, TestState::A);
        assert!(!world.contains_resource::<State<DerivedState>>());

        world.insert_resource(NextState(Some(TestState::B(true))));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<TestState>>().0, TestState::B(true));
        assert_eq!(world.resource::<State<DerivedState>>().0, DerivedState::BisTrue);
        
        world.insert_resource(NextState(Some(TestState::B(false))));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<TestState>>().0, TestState::B(false));
        assert_eq!(world.resource::<State<DerivedState>>().0, DerivedState::BisFalse);

        
        world.insert_resource(NextState(Some(TestState::A)));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<TestState>>().0, TestState::A);
        assert!(!world.contains_resource::<RemoveState<DerivedState>>());
        assert!(!world.contains_resource::<State<DerivedState>>());
    }
}