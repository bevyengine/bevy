use crate::{
    self as bevy_ecs, change_detection::Mut, schedule::SystemLabel, schedule_v3::Systems,
    system::Res, world::World,
};

use std::{
    fmt::Debug,
    hash::Hash,
    ops::{Deref, DerefMut},
};

/// Types that can define states in a finite-state machine.
pub trait State: 'static + Send + Sync + Clone + PartialEq + Eq + Debug + Hash {}
impl<T> State for T where T: 'static + Send + Sync + Clone + PartialEq + Eq + Debug + Hash {}

// /// A [`SystemLabel`] for the system set that runs during a state's "on enter" transition.
// #[derive(SystemLabel)]
// pub struct OnEnter<S: State>(pub S);

// /// A [`SystemLabel`] for the system set that runs during a state's "on exit" transition.
// #[derive(SystemLabel)]
// pub struct OnExit<S: State>(pub S);

/// A finite-state machine whose transitions (enter and exit) have associated system sets
/// ([`OnEnter(state)`] and [`OnExit(state)`]).
///
/// A state transition can be queued through the [`NextState<S>`] resource, and it will
/// be applied by the next [`apply_state_transition::<S>`] system.
pub struct CurrentState<S: State>(pub S);

/// The previous state of the finite-state machine.
pub struct PrevState<S: State>(pub Option<S>);

/// The upcoming state of the finite-state machine.
pub struct NextState<S: State>(pub Option<S>);

/// If a state transition is queued, updates the state machine, then runs the
/// [`OnExit(old_state)`] and [`OnEnter(new_state)`] schedules.
pub fn apply_state_transition<S: State>(world: &mut World) {
    todo!();
    // world.resource_scope(|world, mut state: Mut<CurrentState<S>>| {
    //     if world.resource::<NextState<S>>().0.is_some() {
    //         let new_state = world.resource_mut::<NextState<S>>().0.take().unwrap();
    //         let old_state = std::mem::replace(&mut state.0, new_state.clone());
    //         world.resource_mut::<PrevState<S>>().0 = Some(old_state.clone());
    //         run_schedule(OnExit(old_state), world);
    //         run_schedule(OnEnter(new_state), world);
    //     }
    // });
}
