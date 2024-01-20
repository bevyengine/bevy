use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use crate as bevy_ecs;
use crate::event::Event;
use crate::prelude::FromWorld;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::Resource;
use crate::world::World;

pub use bevy_ecs_macros::States;
use bevy_utils::all_tuples;

use self::sealed::StateSetSealed;

use super::{IntoSystemConfigs, Schedule, Schedules};

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
pub enum NextState<S: States> {
    /// No transition has been planned
    #[default]
    Unchanged,
    /// There is a transition planned for state `S`
    Set(S),
    /// There is a planned removal of the state `S`
    Remove,
}

impl<S: States> NextState<S> {
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        *self = Self::Set(state);
    }

    /// Tentatively set a planned removal of the [`State<S>`] resource.
    pub fn remove(&mut self) {
        *self = Self::Remove;
    }

    /// Remove any planned changes to [`State<S>`]
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
    world
        .try_run_schedule(ComputeDependantStates::<S>::default())
        .ok();
    world.try_run_schedule(OnEnter(state)).ok();
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Sends a relevant [`StateTransitionEvent`]
/// - Runs the [`OnExit(exited_state)`] schedule, if it exists.
/// - Runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition), if it exists.
/// - Derive any dependant states through the [`ComputeDependantStates::<S>`] schedule, if it exists.
/// - Runs the [`OnEnter(entered_state)`] schedule, if it exists.
pub fn apply_state_transition<S: States>(world: &mut World) {
    // We want to take the `NextState` resource,
    // but only mark it as changed if it wasn't empty.
    let Some(next_state_resource) = world.get_resource::<NextState<S>>() else {
        return;
    };

    match next_state_resource {
        NextState::Set(entered) => {
            let entered = entered.clone();
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
                        world
                            .try_run_schedule(ComputeDependantStates::<S>::default())
                            .ok();
                        world.try_run_schedule(OnEnter(entered)).ok();
                    }
                }
                None => {
                    world.insert_resource(State(entered.clone()));
                    world
                        .try_run_schedule(ComputeDependantStates::<S>::default())
                        .ok();
                    world.try_run_schedule(OnEnter(entered)).ok();
                }
            };
        }
        NextState::Remove => {
            if let Some(resource) = world.remove_resource::<State<S>>() {
                world.try_run_schedule(OnExit(resource.0)).ok();
                world
                    .try_run_schedule(ComputeDependantStates::<S>::default())
                    .ok();
            }
        }
        _ => {
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
pub trait ComputedStates: States {
    /// The set of states from which the [`Self`] is derived.
    ///
    /// This can either be a single implementor of [`States`], or a tuple
    /// containing multiple implementors of [`States`].
    type SourceStates: StateSet;

    /// This function gets called whenever one of the [`SourceStates`] changes.
    /// The result is used to set the value of [`State<Self>`].
    ///
    /// If the result is [`None`], the [`State<Self>`] resource will be removed from the world.
    fn compute(
        sources: <<Self as ComputedStates>::SourceStates as StateSet>::Optionals,
    ) -> Option<Self>;

    /// This function sets up systems that compute the state whenever one of the [`SourceStates`]
    /// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
    /// used.
    fn register_state_compute_systems_in_schedule(schedules: &mut Schedules) {
        Self::SourceStates::register_compute_systems_for_dependent_state::<Self>(schedules);
    }
}

mod sealed {
    /// Sealed trait used to prevent external implementations.
    pub trait StateSetSealed {}
}

/// This trait is used allow implementors of [`States`], as well
/// as tuples containing exclusively implementors of [`States`], to
/// be used as [`ComputedStates::SourceStates`].
///
/// It is sealed, and auto implemented for all [`States`] types and
/// tuples containing them.
pub trait StateSet: sealed::StateSetSealed {
    /// The set of states converted into a set of optional states.
    ///
    /// If `StateSet` is a single type, it is wrapped in an `Option`;
    /// If `StateSet` is a tuple, each element within the tuple is wrapped instead:
    /// `(S1, S2, S3)` becomes `(Option<S1>, Option<S2>, Option<S3>)`.
    type Optionals;

    /// Sets up the systems needed to compute `T` whenever any `State` in this
    /// `StateSet` is changed.
    fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(
        schedules: &mut Schedules,
    );
}

impl<S: States> StateSetSealed for S {}

impl<S: States> StateSet for S {
    type Optionals = Option<S>;
    fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(
        schedules: &mut Schedules,
    ) {
        let system = |mut next_state: crate::prelude::ResMut<NextState<T>>,
                      state_set: Option<crate::prelude::Res<State<S>>>| {
            match T::compute(state_set.map(|v| v.0.clone())) {
                Some(updated) => {
                    next_state.set(updated);
                }
                None => {
                    next_state.remove();
                }
            }
        };
        let label = ComputeDependantStates::<S>::default();
        match schedules.get_mut(label.clone()) {
            Some(schedule) => {
                schedule.add_systems((system, apply_state_transition::<T>).chain());
            }
            None => {
                let mut schedule = Schedule::new(label);
                schedule.add_systems((system, apply_state_transition::<T>).chain());
                schedules.insert(schedule);
            }
        }
    }
}

macro_rules! impl_state_set_sealed_tuples {
    ($(($param: ident, $val: ident)), *) => {
        impl<$($param: States),*> StateSetSealed for  ($($param,)*) {}

        impl<$($param: States),*> StateSet for  ($($param,)*) {
            type Optionals = ($(Option<$param>,)*);

            fn register_compute_systems_for_dependent_state<T: ComputedStates<SourceStates = Self>>(schedules: &mut Schedules) {

                let system = |mut next_state: crate::prelude::ResMut<NextState<T>>,  ($($val),*,): ($(Option<crate::prelude::Res<State<$param>>>),*,)| {
                    match T::compute(($($val.map(|v| v.0.clone())),*, )) {
                        Some(updated) => {
                            next_state.set(updated);
                        },
                        None => {
                            next_state.remove();
                        },
                    }
                };

                $(let label = ComputeDependantStates::<$param>::default();
                match schedules.get_mut(label.clone()) {
                    Some(schedule) => {
                        schedule.add_systems((system, apply_state_transition::<T>).chain());
                    },
                    None => {
                        let mut schedule = Schedule::new(label);
                        schedule.add_systems((system, apply_state_transition::<T>).chain());
                        schedules.insert(schedule);
                    },
                })*
            }
        }
    };
}

all_tuples!(impl_state_set_sealed_tuples, 1, 15, S, s);

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_ecs;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum SimpleState {
        #[default]
        A,
        B(bool),
    }

    #[derive(States, PartialEq, Eq, Debug, Hash, Clone)]
    enum TestComputedState {
        BisTrue,
        BisFalse,
    }

    impl ComputedStates for TestComputedState {
        type SourceStates = SimpleState;

        fn compute(sources: Option<SimpleState>) -> Option<Self> {
            sources.and_then(|source| match source {
                SimpleState::A => None,
                SimpleState::B(value) => Some(if value { Self::BisTrue } else { Self::BisFalse }),
            })
        }
    }

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    struct TemporaryScheduleLabel;

    #[test]
    fn computed_state_with_a_single_source_is_correctly_derived() {
        let mut world = World::new();
        world.init_resource::<State<SimpleState>>();
        world.init_resource::<NextState<TestComputedState>>();
        let mut schedules = Schedules::new();
        TestComputedState::register_state_compute_systems_in_schedule(&mut schedules);
        let mut apply_changes = Schedule::new(TemporaryScheduleLabel);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<TestComputedState>>());

        world.insert_resource(NextState::Set(SimpleState::B(true)));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(true)
        );
        assert_eq!(
            world.resource::<State<TestComputedState>>().0,
            TestComputedState::BisTrue
        );

        world.insert_resource(NextState::Set(SimpleState::B(false)));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(
            world.resource::<State<SimpleState>>().0,
            SimpleState::B(false)
        );
        assert_eq!(
            world.resource::<State<TestComputedState>>().0,
            TestComputedState::BisFalse
        );

        world.insert_resource(NextState::Set(SimpleState::A));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert!(!world.contains_resource::<State<TestComputedState>>());
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    struct OtherState {
        a_flexible_value: &'static str,
        another_value: u8,
    }

    #[derive(States, PartialEq, Eq, Debug, Hash, Clone)]
    enum ComplexComputedState {
        InAAndStrIsBobOrJane,
        InTrueBAndUsizeAbove8,
    }

    impl ComputedStates for ComplexComputedState {
        type SourceStates = (SimpleState, OtherState);

        fn compute(
            sources: <<Self as ComputedStates>::SourceStates as StateSet>::Optionals,
        ) -> Option<Self> {
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
        world.init_resource::<NextState<ComplexComputedState>>();

        let mut schedules = Schedules::new();

        ComplexComputedState::register_state_compute_systems_in_schedule(&mut schedules);

        println!("Got here");
        let mut apply_changes = Schedule::new(TemporaryScheduleLabel);
        apply_changes.add_systems(apply_state_transition::<SimpleState>);
        apply_changes.add_systems(apply_state_transition::<OtherState>);
        schedules.insert(apply_changes);

        world.insert_resource(schedules);

        println!("Running schedule");
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(world.resource::<State<SimpleState>>().0, SimpleState::A);
        assert_eq!(
            world.resource::<State<OtherState>>().0,
            OtherState::default()
        );
        assert!(!world.contains_resource::<State<ComplexComputedState>>());

        println!("Doesn't Contains resource");

        world.insert_resource(NextState::Set(SimpleState::B(true)));
        world.run_schedule(TemporaryScheduleLabel);
        assert!(!world.contains_resource::<State<ComplexComputedState>>());
        println!("Still doesn't contain resource");

        world.insert_resource(NextState::Set(OtherState {
            a_flexible_value: "felix",
            another_value: 13,
        }));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(
            world.resource::<State<ComplexComputedState>>().0,
            ComplexComputedState::InTrueBAndUsizeAbove8
        );
        println!("B true and unsuized");

        world.insert_resource(NextState::Set(SimpleState::A));
        world.insert_resource(NextState::Set(OtherState {
            a_flexible_value: "jane",
            another_value: 13,
        }));
        world.run_schedule(TemporaryScheduleLabel);
        assert_eq!(
            world.resource::<State<ComplexComputedState>>().0,
            ComplexComputedState::InAAndStrIsBobOrJane
        );
        println!("Jane");

        world.insert_resource(NextState::Set(SimpleState::B(false)));
        world.insert_resource(NextState::Set(OtherState {
            a_flexible_value: "jane",
            another_value: 13,
        }));
        world.run_schedule(TemporaryScheduleLabel);
        assert!(!world.contains_resource::<State<ComplexComputedState>>());
        println!("No longer contains");
    }
}
