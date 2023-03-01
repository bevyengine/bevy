use std::fmt::Debug;
use std::hash::Hash;
use std::mem;

use crate as bevy_ecs;
use crate::change_detection::DetectChangesMut;
use crate::schedule::{ScheduleLabel, SystemSet};
use crate::system::Resource;
use crate::world::World;

pub use bevy_ecs_macros::States;

/// Types that can define world-wide states in a finite-state machine.
///
/// The [`Default`] trait defines the starting state.
/// Multiple states can be defined for the same world,
/// allowing you to classify the state of the world across orthogonal dimensions.
/// You can access the current state of type `T` with the [`State<T>`] resource,
/// and the queued state with the [`NextState<T>`] resource.
///
/// State transitions typically occur in the [`OnEnter<T::Variant>`] and [`OnExit<T:Variant>`] schedules,
/// which can be run via the [`apply_state_transition::<T>`] system.
/// Systems that run each frame in various states are typically stored in the main schedule,
/// and are conventionally part of the [`OnUpdate(T::Variant)`] system set.
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
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug + Default {
    type Iter: Iterator<Item = Self>;

    /// Returns an iterator over all the state variants.
    fn variants() -> Self::Iter;
}

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// enters this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: States>(pub S);

/// The label of a [`Schedule`](super::Schedule) that runs whenever [`State<S>`]
/// exits this state.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: States>(pub S);

/// A [`SystemSet`] that will run within `CoreSet::Update` when this state is active.
///
/// This set, when created via `App::add_state`, is configured with both a base set and a run condition.
/// If all you want is the run condition, use the [`in_state`](crate::schedule::common_conditions::in_state)
/// [condition](super::Condition) directly.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnUpdate<S: States>(pub S);

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`] and [`OnExit(state)`]).
///
/// The current state value can be accessed through this resource. To *change* the state,
/// queue a transition in the [`NextState<S>`] resource, and it will be applied by the next
/// [`apply_state_transition::<S>`] system.
///
/// The starting state is defined via the [`Default`] implementation for `S`.
#[derive(Resource, Default, Debug)]
pub struct State<S: States>(pub S);

/// The next state of [`State<S>`].
///
/// To queue a transition, just set the contained value to `Some(next_state)`.
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource at the time of [`apply_state_transition`] matters.
#[derive(Resource, Default, Debug)]
pub struct NextState<S: States>(pub Option<S>);

impl<S: States> NextState<S> {
    /// Tentatively set a planned state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        self.0 = Some(state);
    }
}

/// Run the enter schedule for the current state
pub fn run_enter_schedule<S: States>(world: &mut World) {
    world.run_schedule(OnEnter(world.resource::<State<S>>().0.clone()));
}

/// If a new state is queued in [`NextState<S>`], this system:
/// - Takes the new state value from [`NextState<S>`] and updates [`State<S>`].
/// - Runs the [`OnExit(exited_state)`] schedule.
/// - Runs the [`OnEnter(entered_state)`] schedule.
pub fn apply_state_transition<S: States>(world: &mut World) {
    // We want to take the `NextState` resource,
    // but only mark it as changed if it wasn't empty.
    let mut next_state_resource = world.resource_mut::<NextState<S>>();
    if let Some(entered) = next_state_resource.bypass_change_detection().0.take() {
        next_state_resource.set_changed();

        let exited = mem::replace(&mut world.resource_mut::<State<S>>().0, entered.clone());
        world.run_schedule(OnExit(exited));
        world.run_schedule(OnEnter(entered));
    }
}
