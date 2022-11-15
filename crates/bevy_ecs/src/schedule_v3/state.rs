use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
use std::ops::{Deref, DerefMut};

use crate as bevy_ecs;
use crate::change_detection::Mut;
use crate::schedule_v3::{ScheduleLabel, SystemSet, WorldExt};
use crate::system::{Res, Resource};
use crate::world::World;

/// Types that can define states in a finite-state machine.
pub trait Statelike: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {}
impl<T> Statelike for T where T: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {}

/// TBD
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: Statelike>(pub S);

/// TBD
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: Statelike>(pub S);

/// TBD
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnUpdate<S: Statelike>(pub S);

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`] and [`OnExit(state)`]).
///
/// A state transition can be queued through the [`Transition<S>`] resource, and it will
/// be applied by the next [`apply_state_transition::<S>`] system.
#[derive(Resource)]
pub struct State<S: Statelike>(pub S);

/// The next state of [`State<S>`].
#[derive(Resource)]
pub struct Transition<S: Statelike>(pub Option<S>);

/// If a state transition is queued in [`Transition<S>`], updates [`State<S>`], then
/// runs the [`OnExit(exited_state)`] and [`OnEnter(entered_state)`] schedules.
pub fn apply_state_transition<S: Statelike>(world: &mut World) {
    world.resource_scope(|world, mut state: Mut<State<S>>| {
        if world.resource::<Transition<S>>().0.is_some() {
            let entered_state = world.resource_mut::<Transition<S>>().0.take().unwrap();
            let exited_state = mem::replace(&mut state.0, entered_state.clone());
            world.run_schedule(OnExit(exited_state));
            world.run_schedule(OnEnter(entered_state));
        }
    });
}
