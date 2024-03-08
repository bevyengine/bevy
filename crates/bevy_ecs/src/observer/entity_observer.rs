use super::*;
use crate::component::{ComponentHooks, StorageType};

/// Component to signify an entity observer being attached to an entity
/// Can be modelled by parent-child relationship if/when that is enforced
pub(crate) struct AttachObserver(pub(crate) Entity);

impl Component for AttachObserver {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    // When `AttachObserver` is inserted onto an event add it to `ObservedBy`
    // or insert `ObservedBy` if it doesn't exist
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world, entity, _| {
            let attached_observer = world.get::<AttachObserver>(entity).unwrap().0;
            if let Some(mut observed_by) = world.get_mut::<ObservedBy>(entity) {
                observed_by.0.push(attached_observer);
            } else {
                world
                    .commands()
                    .entity(entity)
                    .insert(ObservedBy(vec![attached_observer]));
            }
        });
    }
}

/// Tracks a list of entity observers for the attached entity
pub(crate) struct ObservedBy(Vec<Entity>);

impl Component for ObservedBy {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            let mut component = world.get_mut::<ObservedBy>(entity).unwrap();
            let observed_by = std::mem::take(&mut component.0);
            observed_by.iter().for_each(|&e| {
                world.commands().entity(e).despawn();
            });
        });
    }
}
