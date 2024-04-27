use super::*;
use crate::component::{ComponentHooks, StorageType};

/// Command to attach an entity observer to an entity
pub(crate) struct AttachObserver {
    pub(crate) target: Entity,
    pub(crate) observer: Entity,
}

impl Command for AttachObserver {
    fn apply(self, world: &mut World) {
        if let Some(mut target) = world.get_entity_mut(self.target) {
            let mut observed_by = target
                .entry::<ObservedBy>()
                .or_insert_with(|| ObservedBy(vec![]));
            observed_by.0.push(self.observer);
        } else {
            world.despawn(self.observer);
        }
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
                if let Some(mut entity) = world.commands().get_entity(e) {
                    entity.despawn();
                };
            });
        });
    }
}
