use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
use std::ops::Deref;

use crate as bevy_ecs;
use crate::change_detection::DetectChangesMut;
use crate::event::Event;
use crate::prelude::FromWorld;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::Resource;
use crate::world::World;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;

pub use bevy_ecs_macros::States;
use bevy_utils::tracing::warn;

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
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {
    /// Returns whether the transition from this state to `target` is allowed.
    ///
    /// The default implementation returns always `true`.
    #[inline]
    fn can_transit_to(&self, target: &Self) -> bool {
        let _ = target;
        true
    }
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
    world.try_run_schedule(OnEnter(state.0.clone())).ok();
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Sends a relevant [`StateTransitionEvent`]
/// - Runs the [`OnExit(exited_state)`] schedule, if it exists.
/// - Runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition), if it exists.
/// - Runs the [`OnEnter(entered_state)`] schedule, if it exists.
pub fn apply_state_transition<S: States>(world: &mut World) {
    // We want to take the `NextState` resource,
    // but only mark it as changed if it wasn't empty.
    let Some(mut next_state_resource) = world.get_resource_mut::<NextState<S>>() else {
        return;
    };
    if let Some(entered) = next_state_resource.bypass_change_detection().0.take() {
        next_state_resource.set_changed();
        match world.get_resource_mut::<State<S>>() {
            Some(mut state_resource) => {
                if !state_resource.can_transit_to(&entered) {
                    warn!(
                        "unallowed state transition from {:?} to {entered:?}",
                        *state_resource
                    );
                    return;
                }

                if *state_resource == entered {
                    return;
                }

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
                world.try_run_schedule(OnEnter(entered)).ok();
            }
            None => {
                world.insert_resource(State(entered.clone()));
                world.try_run_schedule(OnEnter(entered)).ok();
            }
        };
    }
}

#[cfg(test)]
mod tests {

    use super::States;

    use crate as bevy_ecs;

    #[test]
    fn state_transition_check() {
        #[derive(States, Clone, PartialEq, Eq, Hash, Debug)]
        enum MyState {
            #[transition_to(not(A, C))]
            A,
            B,
            #[transition_to(A, B)]
            C,
        }

        use MyState::*;
        assert!(!MyState::A.can_transit_to(&A));
        assert!(MyState::A.can_transit_to(&B));
        assert!(!MyState::A.can_transit_to(&C));

        assert!(MyState::B.can_transit_to(&A));
        assert!(MyState::B.can_transit_to(&B));
        assert!(MyState::B.can_transit_to(&C));

        assert!(MyState::C.can_transit_to(&A));
        assert!(MyState::C.can_transit_to(&B));
        assert!(!MyState::C.can_transit_to(&C));
    }

    /// Check that the default implementation of [`States`] allow all transition.
    #[test]
    fn default_state_transition_is_allowed() {
        #[derive(States, Clone, PartialEq, Eq, Hash, Debug)]
        enum MyState {
            A,
            B,
            C,
        }

        use MyState::*;
        assert!(MyState::A.can_transit_to(&A));
        assert!(MyState::A.can_transit_to(&B));
        assert!(MyState::A.can_transit_to(&C));

        assert!(MyState::B.can_transit_to(&A));
        assert!(MyState::B.can_transit_to(&B));
        assert!(MyState::B.can_transit_to(&C));

        assert!(MyState::C.can_transit_to(&A));
        assert!(MyState::C.can_transit_to(&B));
        assert!(MyState::C.can_transit_to(&C));
    }
}
