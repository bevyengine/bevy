use legion::prelude::{Entity, Res};
use std::{collections::HashSet, sync::RwLock};

#[derive(Default)]
pub struct EntitiesWaitingForAssets {
    pub entities: RwLock<HashSet<Entity>>,
}

impl EntitiesWaitingForAssets {
    pub fn add(&self, entity: Entity) {
        self.entities
            .write()
            .expect("RwLock poisoned")
            .insert(entity);
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.entities
            .read()
            .expect("RwLock poisoned")
            .contains(entity)
    }

    pub fn clear(&self) {
        self.entities.write().expect("RwLock poisoned").clear();
    }

    pub fn clear_system(entities_waiting_for_assets: Res<EntitiesWaitingForAssets>) {
        entities_waiting_for_assets.clear();
    }
}
