use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
//use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

pub type Index = u16;

/// Provides a way of describing a hierarchy or named entities
/// and means for finding then in the world
#[derive(Debug, Clone)]
pub struct NamedHierarchy {
    /// Entity identification made by parent index and name
    entities: Vec<(Index, Name)>,
}

impl Default for NamedHierarchy {
    fn default() -> Self {
        Self {
            // ? NOTE: Since the root has no parent in this context it points to a place outside the vec bounds
            entities: vec![(Index::MAX, Name::default())],
        }
    }
}

impl NamedHierarchy {
    /// Number of entities registered.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Gets the entity parent index and `Name` components
    #[inline(always)]
    pub fn get_entity(&self, entity_index: Index) -> &(Index, Name) {
        &self.entities[entity_index as usize]
    }

    pub fn iter(&self) -> impl Iterator<Item = &(Index, Name)> {
        self.entities.iter()
    }

    /// Adds a new entity hierarchy path separated by backslashes (`'/'`)
    /// return the entity index and if was or not inserted
    pub fn get_or_insert_entity(&mut self, entity_path: &str) -> (Index, bool) {
        let mut entity_created = false;
        let mut entity = 0; // Start search from root
        for name in entity_path.split('/') {
            // Ignore the first '/' or '///'
            if name.is_empty() {
                continue;
            }

            if let Some(e) = self
                .entities
                .iter()
                .position(|(p, n)| (*p, n.as_str()) == (entity, name))
            {
                // Found entity
                // ? NOTE: Conversion will never panic because the collection
                // ? size will only increase in the else branch where a
                // ? safe cast is performed
                entity = e as Index;
            } else {
                // Add entity
                let e = self.entities.len();
                self.entities.push((entity, Name::from_str(name)));
                entity_created = true;
                // Soft limit added to save memory, identical to the curve limit
                entity = Index::try_from(e).expect("entities limit reached");
            }
        }

        (entity, entity_created)
    }

    /// Returns the entity path if found.
    ///
    /// The `NamedHierarchy` stores a the entity path in a specific way to improve search performance
    /// thus it needs to rebuilt in the human readable format
    pub fn get_entity_path_at(&self, mut entity_index: Index) -> Option<String> {
        let mut path = None;

        while let Some((parent_index, name)) = self.entities.get(entity_index as usize) {
            if let Some(path) = path.as_mut() {
                *path = format!("{}/{}", name.as_str(), path);
            } else {
                path = Some(name.as_str().to_string());
            }

            entity_index = *parent_index;
        }

        path
    }

    // TODO: Docs of how to proper use the find_* functions

    /// Finds an entity given a set of queries, see the example bellow
    /// how to proper call this function,
    ///
    /// ```rust
    /// let mut entities_table_cache = vec![];
    /// entities_table_cache.resize(clip.hierarchy.len(), None);
    /// // Assign the root entity as the first element
    /// entities_table_cache[0] = Some(root);
    ///
    /// let found_entity = named_hierarchy.find_entity(2, &mut entities_table_cache, children_query, name_query);
    /// ```
    pub fn find_entity(
        &self,
        entity_index: Index,
        entities_table_cache: &mut Vec<Option<Entity>>,
        children_query: &mut Query<(&Children,)>,
        name_query: &mut Query<(&Parent, &Name)>,
    ) -> Option<Entity> {
        if let Some(entity) = &entities_table_cache[entity_index as usize] {
            Some(*entity)
        } else {
            let (parent_index, entity_name) = &self.entities[entity_index as usize];

            // Use recursion to find the entity parent
            self.find_entity(
                *parent_index,
                entities_table_cache,
                children_query,
                name_query,
            )
            .and_then(|parent_entity| {
                if let Ok((children,)) = children_query.get(parent_entity) {
                    children
                        .iter()
                        .find(|entity| {
                            if let Ok((current_parent, name)) = name_query.get(**entity) {
                                // ! FIXME: Parent changes before the children update it self,
                                // ! to account for that we also must double check entity parent component it self
                                if current_parent.0 != parent_entity || name != entity_name {
                                    return false;
                                }

                                // Update cache
                                entities_table_cache[entity_index as usize] = Some(**entity);
                                true
                            } else {
                                false
                            }
                        })
                        .copied()
                } else {
                    None
                }
            })
        }
    }

    /// Finds an entity given a reference to the (`World`)[bevy_ecs::World], see the example bellow
    /// how to proper call this function,
    ///
    /// ```rust
    /// let mut entities_table_cache = vec![];
    /// entities_table_cache.resize(clip.hierarchy.len(), None);
    /// // Assign the root entity as the first element
    /// entities_table_cache[0] = Some(root);
    ///
    /// let found_entity = named_hierarchy.find_entity_in_world(2, &mut entities_table_cache, &world);
    /// ```
    pub fn find_entity_in_world(
        &self,
        entity_index: Index,
        entities_table_cache: &mut Vec<Option<Entity>>,
        world: &World,
    ) -> Option<Entity> {
        if let Some(entity) = &entities_table_cache[entity_index as usize] {
            Some(*entity)
        } else {
            let (parent_index, entity_name) = &self.entities[entity_index as usize];

            // Use recursion to find the entity parent
            self.find_entity_in_world(*parent_index, entities_table_cache, world)
                .and_then(|parent_entity| {
                    if let Ok(children) = world.get::<Children>(parent_entity) {
                        children
                            .iter()
                            .find(|entity| {
                                if let Ok((current_parent, name)) =
                                    world.query_one::<(&Parent, &Name)>(**entity)
                                {
                                    // ! FIXME: Parent changes before the children update it self,
                                    // ! to account for that we also must double check entity parent component it self
                                    if current_parent.0 != parent_entity || name != entity_name {
                                        return false;
                                    }

                                    // Update cache
                                    entities_table_cache[entity_index as usize] = Some(**entity);
                                    true
                                } else {
                                    false
                                }
                            })
                            .copied()
                    } else {
                        None
                    }
                })
        }
    }
}
