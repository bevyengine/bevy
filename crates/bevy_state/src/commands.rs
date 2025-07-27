use bevy_ecs::{error::Result, system::Commands, world::World};
use log::debug;

use crate::state::{FreelyMutableState, NextState};

/// Extension trait for [`Commands`] adding `bevy_state` helpers.
pub trait CommandsStatesExt {
    /// Sets the next state the app should move to.
    ///
    /// Internally this schedules a command that updates the [`NextState<S>`](crate::prelude::NextState)
    /// resource with `state`.
    ///
    /// Note that commands introduce sync points to the ECS schedule, so modifying `NextState`
    /// directly may be more efficient depending on your use-case.
    fn set_state<S: FreelyMutableState>(&mut self, state: S);

    /// Sets the next state the app should move to, returning a Result.
    ///
    /// Returns an error if the [`NextState<S>`](crate::prelude::NextState) resource does not exist.
    ///
    /// Internally this schedules a command that updates the [`NextState<S>`](crate::prelude::NextState)
    /// resource with `state`.
    ///
    /// Note that commands introduce sync points to the ECS schedule, so modifying `NextState`
    /// directly may be more efficient depending on your use-case.
    fn try_set_state<S: FreelyMutableState>(&mut self, state: S);
}

impl CommandsStatesExt for Commands<'_, '_> {
    fn set_state<S: FreelyMutableState>(&mut self, state: S) {
        self.queue(move |w: &mut World| {
            let mut next = w.resource_mut::<NextState<S>>();
            if let NextState::Pending(prev) = &*next {
                if *prev != state {
                    debug!("overwriting next state {prev:?} with {state:?}");
                }
            }
            next.set(state);
        });
    }

    fn try_set_state<S: FreelyMutableState>(&mut self, state: S) {
        self.queue(move |w: &mut World| -> Result {
            let component_id = w.components().get_valid_resource_id(core::any::TypeId::of::<NextState<S>>())
                .ok_or(bevy_ecs::world::error::ResourceFetchError::NotRegistered)?;
            
            match w.get_resource_mut::<NextState<S>>() {
                Some(mut next) => {
                    if let NextState::Pending(prev) = &*next {
                        if *prev != state {
                            debug!("overwriting next state {prev:?} with {state:?}");
                        }
                    }
                    next.set(state);
                    Ok(())
                }
                None => Err(bevy_ecs::world::error::ResourceFetchError::DoesNotExist(component_id).into()),
            }
        });
    }
}
