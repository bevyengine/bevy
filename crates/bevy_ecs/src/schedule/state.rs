use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
use std::ops::Deref;

use crate as bevy_ecs;
#[cfg(feature = "bevy_reflect")]
use crate::reflect::ReflectResource;
use crate::schedule::ScheduleLabel;
use crate::system::Resource;
use crate::world::World;
pub use bevy_ecs_macros::States;
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

/// A schedule for every time a state is entered
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Entering;
/// A schedule for every time a state is changed
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Transitioning;

/// A schedule for every time a state is exited
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Exiting;

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
pub(crate) struct PreviousState<S: States>(S);

impl<S: States> PreviousState<S> {
    pub(crate) fn new(state: S) -> Self {
        Self(state)
    }

    /// Get the current state.
    pub(crate) fn get(&self) -> &S {
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
    Setter(
        #[cfg_attr(feature = "bevy_reflect", reflect(ignore, default = "default_setter"))]
        Box<dyn Fn(S) -> S + Sync + Send>,
    ),
}

fn default_setter<S: States>() -> Box<dyn Fn(S) -> S + Sync + Send> {
    Box::new(|state: S| state)
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
    // We are running the transition schedule because you could be transitioning from having no State resource to having one.
    world.try_run_schedule(Transitioning).ok();
    world.try_run_schedule(Entering).ok();
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Runs the [`OnExit(exited_state)`] and [`Exiting`] schedules, if they exist.
/// - Runs the [`OnTransition { from: exited_state, to: entered_state }`](OnTransition) and [`Transitioning`] schedules, if they exist.
/// - Runs the [`OnEnter(entered_state)`] and [`Entering`] schedules, if they exist.
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
            world.insert_resource(PreviousState::new(current_state));
            let mut state_resource = world.resource_mut::<State<S>>();
            let exited = mem::replace(&mut state_resource.0, entered.clone());
            // Try to run the schedules if they exist.
            world.try_run_schedule(OnExit(exited.clone())).ok();
            world.try_run_schedule(Exiting).ok();
            world
                .try_run_schedule(OnTransition {
                    from: exited,
                    to: entered.clone(),
                })
                .ok();
            world.try_run_schedule(Transitioning).ok();
            world.try_run_schedule(OnEnter(entered)).ok();
            world.try_run_schedule(Entering).ok();
            world.remove_resource::<PreviousState<S>>();
        }

        world.insert_resource(NextState::<S>::Keep);
    }
}
