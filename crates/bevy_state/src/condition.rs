use crate::state::{State, States};
use bevy_ecs::{change_detection::DetectChanges, system::Res};

/// A [`Condition`](bevy_ecs::prelude::Condition)-satisfying system that returns `true`
/// if the state machine exists.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
/// # #[derive(Resource, Default)]
/// # struct Counter(u8);
/// # let mut app = Schedule::default();
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
/// enum GameState {
///     #[default]
///     Playing,
///     Paused,
/// }
///
/// app.add_systems(
///     // `state_exists` will only return true if the
///     // given state exists
///     my_system.run_if(state_exists::<GameState>),
/// );
///
/// fn my_system(mut counter: ResMut<Counter>) {
///     counter.0 += 1;
/// }
///
/// // `GameState` does not yet exist `my_system` won't run
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 0);
///
/// world.init_resource::<State<GameState>>();
///
/// // `GameState` now exists so `my_system` will run
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 1);
/// ```
pub fn state_exists<S: States>(current_state: Option<Res<State<S>>>) -> bool {
    current_state.is_some()
}

/// Generates a [`Condition`](bevy_ecs::prelude::Condition)-satisfying closure that returns `true`
/// if the state machine is currently in `state`.
///
/// Will return `false` if the state does not exist or if not in `state`.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
/// # #[derive(Resource, Default)]
/// # struct Counter(u8);
/// # let mut app = Schedule::default();
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
/// enum GameState {
///     #[default]
///     Playing,
///     Paused,
/// }
///
/// world.init_resource::<State<GameState>>();
///
/// app.add_systems((
///     // `in_state` will only return true if the
///     // given state equals the given value
///     play_system.run_if(in_state(GameState::Playing)),
///     pause_system.run_if(in_state(GameState::Paused)),
/// ));
///
/// fn play_system(mut counter: ResMut<Counter>) {
///     counter.0 += 1;
/// }
///
/// fn pause_system(mut counter: ResMut<Counter>) {
///     counter.0 -= 1;
/// }
///
/// // We default to `GameState::Playing` so `play_system` runs
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 1);
///
/// *world.resource_mut::<State<GameState>>() = State::new(GameState::Paused);
///
/// // Now that we are in `GameState::Pause`, `pause_system` will run
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 0);
/// ```
pub fn in_state<S: States>(state: S) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
    move |current_state: Option<Res<State<S>>>| match current_state {
        Some(current_state) => *current_state == state,
        None => false,
    }
}

/// A [`Condition`](bevy_ecs::prelude::Condition)-satisfying system that returns `true`
/// if the state machine changed state.
///
/// To do things on transitions to/from specific states, use their respective OnEnter/OnExit
/// schedules. Use this run condition if you want to detect any change, regardless of the value.
///
/// Returns false if the state does not exist or the state has not changed.
///
/// # Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
/// # #[derive(Resource, Default)]
/// # struct Counter(u8);
/// # let mut app = Schedule::default();
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #[derive(States, Clone, Copy, Default, Eq, PartialEq, Hash, Debug)]
/// enum GameState {
///     #[default]
///     Playing,
///     Paused,
/// }
///
/// world.init_resource::<State<GameState>>();
///
/// app.add_systems(
///     // `state_changed` will only return true if the
///     // given states value has just been updated or
///     // the state has just been added
///     my_system.run_if(state_changed::<GameState>),
/// );
///
/// fn my_system(mut counter: ResMut<Counter>) {
///     counter.0 += 1;
/// }
///
/// // `GameState` has just been added so `my_system` will run
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 1);
///
/// // `GameState` has not been updated so `my_system` will not run
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 1);
///
/// *world.resource_mut::<State<GameState>>() = State::new(GameState::Paused);
///
/// // Now that `GameState` has been updated `my_system` will run
/// app.run(&mut world);
/// assert_eq!(world.resource::<Counter>().0, 2);
/// ```
pub fn state_changed<S: States>(current_state: Option<Res<State<S>>>) -> bool {
    let Some(current_state) = current_state else {
        return false;
    };
    current_state.is_changed()
}

#[cfg(test)]
mod tests {
    use crate as bevy_state;

    use bevy_ecs::schedule::{Condition, IntoSystemConfigs, Schedule};

    use crate::prelude::*;
    use bevy_state_macros::States;

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum TestState {
        #[default]
        A,
        B,
    }

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(state_exists::<TestState>)
                .distributive_run_if(in_state(TestState::A).or(in_state(TestState::B)))
                .distributive_run_if(state_changed::<TestState>),
        );
    }
}
