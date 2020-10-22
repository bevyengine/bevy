use bevy_ecs::prelude::*;
use bevy_property::Properties;
use bevy_utils::{HashMap, HashSet};
use std::{
    borrow::Cow,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A collection of labels
#[derive(Default, Properties)]
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

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.labels.iter().map(|label| label.deref())
    }
}

/// Maintains a mapping from [Entity](bevy_ecs::prelude::Entity) ids to entity labels and entity labels to [Entities](bevy_ecs::prelude::Entity).
#[derive(Debug, Default)]
pub struct EntityLabels {
    label_entities: HashMap<Cow<'static, str>, Vec<Entity>>,
    entity_labels: HashMap<Entity, HashSet<Cow<'static, str>>>,
}

impl EntityLabels {
    pub fn get(&self, label: &str) -> Option<&[Entity]> {
        self.label_entities
            .get(label)
            .map(|entities| entities.as_slice())
    }
}

pub(crate) fn entity_labels_system(
    mut entity_labels: ResMut<EntityLabels>,
    // TODO: use change tracking when add/remove events are added
    // mut query: Query<(Entity, Changed<Labels>)>,
    mut query: Query<(Entity, &Labels)>,
) {
    let entity_labels = entity_labels.deref_mut();
    for (entity, labels) in &mut query.iter() {
        let current_labels = entity_labels
            .entity_labels
            .entry(entity)
            .or_insert_with(HashSet::default);
        for removed_label in current_labels.difference(&labels.labels) {
            if let Some(entities) = entity_labels.label_entities.get_mut(removed_label) {
                entities.retain(|e| *e != entity);
            }
        }

        for added_label in labels.labels.difference(&current_labels) {
            if let Some(entities) = entity_labels.label_entities.get_mut(added_label) {
                entities.push(entity);
            }
        }

        *current_labels = labels.labels.clone();
    }
}
