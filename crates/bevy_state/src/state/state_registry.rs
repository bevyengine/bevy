use bevy_ecs::{
    system::{IntoSystem, SystemId},
    world::World,
};

use super::{last_transition, run_enter, run_exit, run_transition, FreelyMutableState};

/// Linearized state graph.
#[derive(Default)]
pub struct StateRegistry {
    states: Vec<StateEntry>,
}

impl StateRegistry {
    /// Register a root state
    pub fn register_root_state<S: FreelyMutableState>(&mut self, world: &mut World) {
        let depth = S::DEPENDENCY_DEPTH;
        let update = world.register_system(super::apply_state_transition::<S>);
        let on_exit = vec![world.register_system(last_transition::<S>.pipe(run_exit::<S>))];
        let on_transition =
            vec![world.register_system(last_transition::<S>.pipe(run_transition::<S>))];
        let on_enter = vec![world.register_system(last_transition::<S>.pipe(run_enter::<S>))];

        let entry = StateEntry {
            depth,
            update,
            on_exit,
            on_transition,
            on_enter,
        };
        self.insert_entry(entry);
    }

    //pub fn register_sub_state() {}

    //pub fn register_computed_state() {}

    /// Inserts one type erased state behavior into a depth-sorted vector.
    fn insert_entry(&mut self, entry: StateEntry) {
        if let Some((i, _)) = self
            .states
            .iter()
            .enumerate()
            .find(|s| s.1.depth >= entry.depth)
        {
            self.states.insert(i, entry);
        } else {
            self.states.push(entry);
        }
    }

    /// Runs state update functions and registered transitions.
    pub fn update(&self, world: &mut World) {
        // Run updates
        for state in self.states.iter() {
            world.run_system(state.update).unwrap();
        }

        // Run callbacks: exit, transition, enter
        for state in self.states.iter().rev() {
            for system in state.on_exit.iter() {
                world.run_system(*system).unwrap();
            }
        }
        for state in self.states.iter() {
            for system in state.on_transition.iter() {
                world.run_system(*system).unwrap();
            }
        }
        for state in self.states.iter() {
            for system in state.on_enter.iter() {
                world.run_system(*system).unwrap();
            }
        }
    }
}

// What about:
// - related type/component ids
pub struct StateEntry {
    /// Depth in the dependency graph
    depth: usize,
    /// Function that updates the state based on [`NextState`](crate::state::NextState) and parent states.
    update: SystemId,
    /// Systems that run when state is exited, executed in leaf-root graph order
    on_exit: Vec<SystemId>,
    /// Systems that run when state is changed, executed in arbitrary order
    on_transition: Vec<SystemId>,
    /// Systems that run when state is entered, executed in root-leaf graph order
    on_enter: Vec<SystemId>,
}
