pub mod condition;
pub mod state;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::condition::*;
    #[doc(hidden)]
    pub use crate::state::{
        apply_state_transition, ComputedStates, NextState, OnEnter, OnExit, OnTransition, State,
        StateSet, StateTransition, StateTransitionEvent, States, SubStates,
    };
}
