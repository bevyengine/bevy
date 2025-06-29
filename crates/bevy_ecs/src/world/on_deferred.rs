use alloc::{boxed::Box, vec::Vec};

use crate::{
    resource::Resource,
    world::{Mut, World},
};

/// A resource holding the set of all actions to take just before applying deferred actions.
#[derive(Resource, Default)]
pub struct OnDeferred(Vec<Box<dyn FnMut(&mut World) + Send + Sync + 'static>>);

impl OnDeferred {
    /// Adds an action to be executed before applying deferred actions.
    pub fn add(&mut self, action: impl FnMut(&mut World) + Send + Sync + 'static) {
        self.0.push(Box::new(action));
    }

    /// Executes the actions in [`OnDeferred`] from `world`. Does nothing if [`OnDeferred`] does not
    /// exist in the world.
    pub(crate) fn execute(world: &mut World) {
        world.try_resource_scope(|world, mut this: Mut<Self>| {
            for action in &mut this.0 {
                action(world);
            }
        });
    }
}
