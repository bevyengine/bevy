use bevy_ecs::{
    entity::Entity,
    query::Changed,
    reflect::ReflectComponent,
    system::{Query, RemovedComponents, ResMut},
};
use bevy_reflect::Reflect;
use bevy_utils::{HashMap, HashSet};
use std::{
    borrow::Cow,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A collection of labels
#[derive(Default, Reflect)]
#[reflect(Component)]
pub struct Labels {
    labels: HashSet<Cow<'static, str>>,
}

impl Debug for Labels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for label in self.iter() {
            list.entry(&label);
        }

        list.finish()
    }
}

impl<'a, T, L: Into<Cow<'static, str>>> From<T> for Labels
where
    T: IntoIterator<Item = L>,
{
    fn from(value: T) -> Self {
        let mut labels = HashSet::default();
        for label in value {
            labels.insert(label.into());
        }
        Self { labels }
    }
}

impl Labels {
    pub fn contains<T: Into<Cow<'static, str>>>(&self, label: T) -> bool {
        self.labels.contains(&label.into())
    }

    pub fn insert<T: Into<Cow<'static, str>>>(&mut self, label: T) {
        self.labels.insert(label.into());
    }

    pub fn remove<T: Into<Cow<'static, str>>>(&mut self, label: T) {
        self.labels.remove(&label.into());
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.labels.iter().map(|label| label.deref())
    }
}

/// Maintains a mapping from [Entity](bevy_ecs::prelude::Entity) ids to entity labels and entity
/// labels to [Entities](bevy_ecs::prelude::Entity).
#[derive(Debug, Default)]
pub struct EntityLabels {
    label_entities: HashMap<Cow<'static, str>, Vec<Entity>>,
    entity_labels: HashMap<Entity, HashSet<Cow<'static, str>>>,
}

impl EntityLabels {
    pub fn get(&self, label: &str) -> &[Entity] {
        self.label_entities
            .get(label)
            .map(|entities| entities.as_slice())
            .unwrap_or(&[])
    }
}

pub(crate) fn entity_labels_system(
    mut entity_labels: ResMut<EntityLabels>,
    removed_labels: RemovedComponents<Labels>,
    query: Query<(Entity, &Labels), Changed<Labels>>,
) {
    let entity_labels = entity_labels.deref_mut();

    for entity in removed_labels.iter() {
        if let Some(labels) = entity_labels.entity_labels.get(&entity) {
            for label in labels.iter() {
                if let Some(entities) = entity_labels.label_entities.get_mut(label) {
                    entities.retain(|e| *e != entity);
                }
            }
        }
    }

    for (entity, labels) in query.iter() {
        let current_labels = entity_labels
            .entity_labels
            .entry(entity)
            .or_insert_with(HashSet::default);

        for removed_label in current_labels.difference(&labels.labels) {
            if let Some(entities) = entity_labels.label_entities.get_mut(removed_label) {
                entities.retain(|e| *e != entity);
            }
        }

        for added_label in labels.labels.difference(current_labels) {
            entity_labels
                .label_entities
                .entry(added_label.clone())
                .or_insert_with(Vec::new)
                .push(entity);
        }

        *current_labels = labels.labels.clone();
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        schedule::{Schedule, Stage, SystemStage},
        world::World,
    };

    use super::*;

    fn setup() -> (World, Schedule) {
        let mut world = World::new();
        world.insert_resource(EntityLabels::default());
        let mut schedule = Schedule::default();
        schedule.add_stage("test", SystemStage::single_threaded());
        schedule.add_system_to_stage("test", entity_labels_system);
        (world, schedule)
    }

    fn holy_cow() -> Labels {
        Labels::from(["holy", "cow"].iter().cloned())
    }

    fn holy_shamoni() -> Labels {
        Labels::from(["holy", "shamoni"].iter().cloned())
    }

    #[test]
    fn adds_spawned_entity() {
        let (mut world, mut schedule) = setup();

        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[e1], "holy");
        assert_eq!(entity_labels.get("cow"), &[e1], "cow");
        assert_eq!(entity_labels.get("shalau"), &[], "shalau");
    }

    #[test]
    fn add_labels() {
        let (mut world, mut schedule) = setup();
        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        world.get_mut::<Labels>(e1).unwrap().insert("shalau");
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[e1], "holy");
        assert_eq!(entity_labels.get("cow"), &[e1], "cow");
        assert_eq!(entity_labels.get("shalau"), &[e1], "shalau");
    }

    #[test]
    fn remove_labels() {
        let (mut world, mut schedule) = setup();
        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        world.get_mut::<Labels>(e1).unwrap().remove("holy");
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[], "holy");
        assert_eq!(entity_labels.get("cow"), &[e1], "cow");
        assert_eq!(entity_labels.get("shalau"), &[], "shalau");
    }

    #[test]
    fn removes_despawned_entity() {
        let (mut world, mut schedule) = setup();
        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        assert!(world.despawn(e1));
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[], "holy");
        assert_eq!(entity_labels.get("cow"), &[], "cow");
        assert_eq!(entity_labels.get("shalau"), &[], "shalau");
    }

    #[test]
    fn removes_labels_when_component_removed() {
        let (mut world, mut schedule) = setup();
        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        world.entity_mut(e1).remove::<Labels>().unwrap();
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[], "holy");
        assert_eq!(entity_labels.get("cow"), &[], "cow");
        assert_eq!(entity_labels.get("shalau"), &[], "shalau");
    }

    #[test]
    fn adds_another_spawned_entity() {
        let (mut world, mut schedule) = setup();
        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        let e2 = world.spawn().insert(holy_shamoni()).id();
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[e1, e2], "holy");
        assert_eq!(entity_labels.get("cow"), &[e1], "cow");
        assert_eq!(entity_labels.get("shamoni"), &[e2], "shamoni");
        assert_eq!(entity_labels.get("shalau"), &[], "shalau");
    }

    #[test]
    fn removes_despawned_entity_but_leaves_other() {
        let (mut world, mut schedule) = setup();
        let e1 = world.spawn().insert(holy_cow()).id();
        schedule.run(&mut world);

        let e2 = world.spawn().insert(holy_shamoni()).id();
        schedule.run(&mut world);

        assert!(world.despawn(e1));
        schedule.run(&mut world);

        let entity_labels = world.get_resource::<EntityLabels>().unwrap();
        assert_eq!(entity_labels.get("holy"), &[e2], "holy");
        assert_eq!(entity_labels.get("cow"), &[], "cow");
        assert_eq!(entity_labels.get("shamoni"), &[e2], "shamoni");
        assert_eq!(entity_labels.get("shalau"), &[], "shalau");
    }
}
