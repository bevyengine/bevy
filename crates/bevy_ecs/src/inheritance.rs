//! This module contains code relating to component inheritance.
//!
//! [`InheritedComponents`] is the main structure holding data about inherited components, it can be used
//! to record and resolve archetypes/tables that contain components from an entity in another archetype/table.
//!
//! [`InheritFrom`] is the main user-facing component that allows some entity to inherit components from some other entity.
use alloc::collections::vec_deque::VecDeque;
use alloc::string::ToString;
use alloc::vec::Vec;
use bevy_platform_support::collections::{HashMap, HashSet};
use core::{alloc::Layout, ptr::NonNull};

use bevy_ptr::OwningPtr;

use crate::{
    archetype::{ArchetypeComponentId, ArchetypeEntity, ArchetypeId, ArchetypeRecord, Archetypes},
    component::{
        Component, ComponentCloneBehavior, ComponentDescriptor, ComponentId, Components,
        HookContext, StorageType,
    },
    entity::{Entities, Entity, EntityHashMap},
    storage::TableId,
    world::{DeferredWorld, World},
};

#[derive(Component)]
#[component(
    immutable,
    on_insert=InheritFrom::on_insert,
    on_remove=InheritFrom::on_remove_or_replace,
    on_replace=InheritFrom::on_remove_or_replace,
)]
/// Mark this entity to inherit components from the provided entity.
/// Multiple levels of single inheritance are supported, but each entity can inherit component from only one other entity.
///
/// At the moment, components are inherited only read-only, trying to access inherited component as mutable will behave as if
/// inherited component doesn't exist.
///
/// Circular inheritance is not supported and creating cycles will result in unpredictably-inherited components.
pub struct InheritFrom(pub Entity);

impl InheritFrom {
    fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
        let entity = ctx.entity;
        let base = world.entity(entity).get::<InheritFrom>().unwrap().0;
        world.commands().queue(move |world: &mut World| {
            let base_component_id =
                if let Some(id) = world.inherited_components.entities_to_ids.get(&base) {
                    *id
                } else {
                    let name = [base.to_string(), "-base".to_string()].join("");
                    let id =
                    // SAFETY: The component descriptor is for a ZST, so it's Send + Sync and drop is None
                    unsafe {
                    world.register_component_with_descriptor(ComponentDescriptor::new_with_layout(
                        name,
                        StorageType::Table,
                        Layout::new::<()>(),
                        None,
                        false,
                        ComponentCloneBehavior::Ignore,
                    ))
                    };
                    world.flush_components();
                    world.inherited_components.ids_to_entities.insert(id, base);
                    world.inherited_components.entities_to_ids.insert(base, id);
                    id
                };
            // SAFETY:
            // - NonNull::dangling is a valid data pointer for a ZST component.
            // - Component id is from the same world as entity.
            unsafe {
                world
                    .entity_mut(entity)
                    .insert_by_id(base_component_id, OwningPtr::new(NonNull::dangling()));
            }
            world.entity_mut(base).insert(Inherited);
        });
    }
    fn on_remove_or_replace(mut world: DeferredWorld, ctx: HookContext) {
        let entity = ctx.entity;
        let base = world.entity(entity).get::<InheritFrom>().unwrap().0;
        world.commands().queue(move |world: &mut World| {
            if let Some(&base_component_id) = world.inherited_components.entities_to_ids.get(&base)
            {
                if let Ok(mut entity) = world.get_entity_mut(entity) {
                    entity.remove_by_id(base_component_id);
                }
            }
        });
    }
}

#[derive(Component)]
#[component(
    immutable,
    storage = "SparseSet",
    on_remove=Inherited::on_remove,
)]
/// A marker component that indicates that this entity is inherited by other entities.
pub struct Inherited;

impl Inherited {
    fn on_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        world.commands().queue(move |world: &mut World| {
            let Some(component_id) = world.inherited_components.entities_to_ids.remove(&entity)
            else {
                return;
            };
            world
                .inherited_components
                .ids_to_entities
                .remove(&component_id);
            let Some(entities) =
                world
                    .archetypes()
                    .component_index()
                    .get(&component_id)
                    .map(|map| {
                        map.keys()
                            .flat_map(|archetype_id| {
                                world
                                    .archetypes()
                                    .get(*archetype_id)
                                    .unwrap()
                                    .entities()
                                    .iter()
                                    .map(ArchetypeEntity::id)
                            })
                            .collect::<Vec<_>>()
                    })
            else {
                return;
            };
            for entity in entities {
                world.entity_mut(entity).remove_by_id(component_id);
            }
        });
    }
}

#[derive(Default)]
/// Contains information about inherited components and entities.
///
/// If an archetype or a table has inherited components, this will contain
/// all the data required to get which components are inherited and to get the actual component data.
pub struct InheritedComponents {
    /// Mapping of entities to component ids representing the inherited archetypes and tables.
    /// Must be kept synchronized with `ids_to_entities`
    pub(crate) entities_to_ids: EntityHashMap<ComponentId>,
    /// Mapping of component ids to entities representing the inherited archetypes and tables.
    /// Must be kept synchronized with `entities_to_ids`
    pub(crate) ids_to_entities: HashMap<ComponentId, Entity>,
    /// List of inherited components along with entities containing the inherited component for the archetypes
    pub(crate) archetype_inherited_components:
        HashMap<ArchetypeId, HashMap<ComponentId, (Entity, ArchetypeComponentId)>>,
    /// List of inherited components along with entities containing the inherited component for the tables
    pub(crate) table_inherited_components: HashMap<TableId, HashMap<ComponentId, Entity>>,
}

impl InheritedComponents {
    /// This method must be called after a new archetype is created to initialized inherited components once.
    pub(crate) fn init_inherited_components(
        &mut self,
        entities: &Entities,
        components: &Components,
        archetypes: &mut Archetypes,
        archetype_id: ArchetypeId,
    ) {
        let archetype = &archetypes.archetypes[archetype_id.index()];
        if !archetype.has_inherited_components() {
            return;
        }
        // Empty inherited components hashmap to enable .chain
        let empty_inherited_components = HashMap::default();
        let archetype_inherited_components = HashMap::from_iter(
            archetype
                .table_components()
                .filter_map(|id| {
                    self.ids_to_entities
                        .get(&id)
                        .and_then(|entity| {
                            entities.get(*entity).map(|location| (location, *entity))
                        })
                        .and_then(|(location, entity)| {
                            archetypes
                                .get(location.archetype_id)
                                .map(|archetype| (archetype, entity))
                        })
                })
                .flat_map(|(inherited_archetype, entity)| {
                    inherited_archetype
                        .components_with_archetype_component_id()
                        .zip(core::iter::repeat(entity))
                        .map(|((component_id, archetype_component_id), entity)| {
                            (component_id, (entity, archetype_component_id))
                        })
                        .chain(
                            self.archetype_inherited_components
                                .get(&inherited_archetype.id())
                                .unwrap_or(&empty_inherited_components)
                                .iter()
                                .map(|(&id, &entity)| (id, entity)),
                        )
                })
                .filter(|(id, ..)| !archetype.contains(*id)),
        );

        // Update component index to include this archetype in all the inherited components.
        for (inherited_component, ..) in archetype_inherited_components.iter() {
            archetypes
                .by_component
                .entry(*inherited_component)
                .or_default()
                .insert(
                    archetype_id,
                    ArchetypeRecord {
                        column: None,
                        is_inherited: true,
                    },
                );
        }

        // If table already contains inherited components, it is already initialized.
        // Since the only difference between archetypes with the same table is in sparse set components,
        // we can skip reinitializing table's inherited components.
        // `update_inherited_components` will take care of updating existing table's components.
        self.table_inherited_components
            .entry(archetype.table_id())
            .or_insert_with(|| {
                HashMap::from_iter(
                    archetype_inherited_components
                        .iter()
                        .map(|(id, (e, ..))| (*id, *e))
                        .filter(|(id, ..)| {
                            components.get_info(*id).unwrap().storage_type() == StorageType::Table
                        }),
                )
            });

        self.archetype_inherited_components
            .insert(archetype_id, archetype_inherited_components);
    }

    /// This method must be called after an entity changes archetype to update all archetypes inheriting components from this entity.
    pub(crate) fn update_inherited_archetypes<const UPDATE_TABLES: bool>(
        &mut self,
        archetypes: &mut Archetypes,
        components: &Components,
        old_base_archetype_id: ArchetypeId,
        new_base_archetype_id: ArchetypeId,
        entity: Entity,
    ) {
        let Some(component_id) = self.entities_to_ids.get(&entity) else {
            return;
        };
        let old_base_archetype = &archetypes.archetypes[old_base_archetype_id.index()];
        let new_base_archetype = &archetypes.archetypes[new_base_archetype_id.index()];
        let added_components = new_base_archetype
            .components_with_archetype_component_id()
            .filter(|(component_id, ..)| !old_base_archetype.contains(*component_id))
            .collect::<Vec<_>>();
        let mut removed_components = old_base_archetype
            .components()
            .filter(|component_id| !new_base_archetype.contains(*component_id))
            .collect::<Vec<_>>();
        removed_components.sort_unstable();

        let mut inherited_entities_queue = VecDeque::from([(entity, *component_id)]);
        let mut archetypes_to_update = Vec::new();
        let mut processed_archetypes = HashSet::<ArchetypeId>::default();

        while let Some((entity, component_id)) = inherited_entities_queue.pop_front() {
            archetypes_to_update.extend(
                archetypes
                    .by_component
                    .get(&component_id)
                    .unwrap()
                    .keys()
                    .copied()
                    .filter(|archetype_id| !processed_archetypes.contains(archetype_id)),
            );
            for archetype in archetypes_to_update.drain(..) {
                // Update archetype's inherited components
                let archetype = &archetypes.archetypes[archetype.index()];
                let archetype_inherited_components = self
                    .archetype_inherited_components
                    .entry(archetype.id())
                    .or_default();
                archetype_inherited_components.retain(|component_id, _| {
                    removed_components.binary_search(component_id).is_err()
                });
                archetype_inherited_components.extend(
                    added_components
                        .iter()
                        .copied()
                        .filter(|(component_id, ..)| !archetype.contains(*component_id))
                        .zip(core::iter::repeat(entity))
                        .map(|((component_id, archetype_component_id), entity)| {
                            (component_id, (entity, archetype_component_id))
                        }),
                );

                // Update component index
                for removed_component in &removed_components {
                    archetypes
                        .by_component
                        .entry(*removed_component)
                        .and_modify(|map| {
                            if let Some(record) = map.get(&archetype.id()) {
                                if !record.is_inherited {
                                    return;
                                }
                            }
                            map.remove(&archetype.id());
                        });
                }
                for (added_component, ..) in &added_components {
                    archetypes
                        .by_component
                        .entry(*added_component)
                        .and_modify(|map| {
                            if !map.contains_key(&archetype.id()) {
                                map.insert(
                                    archetype.id(),
                                    ArchetypeRecord {
                                        column: None,
                                        is_inherited: true,
                                    },
                                );
                            }
                        });
                }

                // Update tables's inherited components
                if UPDATE_TABLES {
                    let table_inherited_components = self
                        .table_inherited_components
                        .entry(archetype.table_id())
                        .or_default();
                    table_inherited_components.retain(|component_id, _| {
                        removed_components.binary_search(component_id).is_err()
                    });
                    table_inherited_components.extend(
                        added_components
                            .iter()
                            .map(|(component_id, ..)| *component_id)
                            .filter(|component_id| {
                                !archetype.contains(*component_id)
                                    && components.get_info(*component_id).unwrap().storage_type()
                                        == StorageType::Table
                            })
                            .zip(core::iter::repeat(entity)),
                    );
                }

                if archetype.is_inherited() {
                    for (entity, component_id) in archetype.entities().iter().filter_map(|entity| {
                        self.entities_to_ids
                            .get(&entity.id())
                            .map(|component_id| (entity.id(), *component_id))
                    }) {
                        inherited_entities_queue.push_back((entity, component_id));
                    }
                    processed_archetypes.insert(archetype.id());
                }
            }
        }
    }

    /// Returns an iterator yielding all archetypes acting as a base for the passed archetype.
    pub fn get_base_archetypes(
        &self,
        entities: &Entities,
        archetype_id: ArchetypeId,
    ) -> impl Iterator<Item = ArchetypeId> {
        HashSet::<ArchetypeId>::from_iter(
            self.archetype_inherited_components
                .get(&archetype_id)
                .unwrap()
                .values()
                .filter_map(|(entity, ..)| {
                    entities.get(*entity).map(|location| location.archetype_id)
                }),
        )
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::query::{With, Without};
    use crate::system::{IntoSystem, Query, System};
    use crate::world::{Ref, World};

    use super::*;

    #[test]
    fn basic_inheritance() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component_id = world.register_component::<CompA>();
        let component = CompA(6);

        let base = world.spawn(component).id();
        let inherited = world.spawn(InheritFrom(base)).id();
        world.flush();

        let entity = world.get_entity(inherited).unwrap();
        assert_eq!(entity.get::<CompA>(), Some(&component));
        assert_eq!(
            entity.get_by_id(component_id).map(|c| {
                // SAFETY: CompA is registered with component_id
                unsafe { c.deref::<CompA>() }
            }),
            Ok(&component)
        );
        assert_eq!(
            entity.get_ref::<CompA>().map(Ref::into_inner),
            Some(&component)
        );

        let component_ticks = world.get_entity(base).unwrap().get_change_ticks::<CompA>();
        assert_eq!(entity.get_change_ticks::<CompA>(), component_ticks);
        assert_eq!(entity.get_change_ticks_by_id(component_id), component_ticks);
    }

    #[test]
    fn override_inherited() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component = CompA(7);

        let base = world.spawn(CompA(5)).id();
        let inherited = world.spawn((component, InheritFrom(base))).id();

        let entity = world.get_entity(inherited).unwrap();
        assert_eq!(entity.get::<CompA>(), Some(&component));
    }

    #[test]
    fn recursive_inheritance() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component = CompA(6);

        let base = world.spawn(component).id();
        let inherited1 = world.spawn(InheritFrom(base)).id();
        let inherited2 = world.spawn(InheritFrom(inherited1)).id();

        let entity = world.get_entity(inherited2).unwrap();
        assert_eq!(entity.get::<CompA>(), Some(&component));
    }

    #[test]
    fn move_inherited_entity_archetype() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);
        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompB(i32);

        let component1 = CompA(6);
        let component2 = CompB(1);

        let base = world.spawn(component1).id();
        let level1 = world.spawn(InheritFrom(base)).id();
        let level2 = world.spawn(InheritFrom(level1)).id();

        world.entity_mut(base).insert(component2);

        let entity = world.get_entity(level1).unwrap();
        assert_eq!(entity.get::<CompA>(), Some(&component1));
        assert_eq!(entity.get::<CompB>(), Some(&component2));
        let entity = world.get_entity(level2).unwrap();
        assert_eq!(entity.get::<CompA>(), Some(&component1));
        assert_eq!(entity.get::<CompB>(), Some(&component2));

        world.entity_mut(base).remove::<CompA>();
        let entity = world.get_entity(level1).unwrap();
        assert_eq!(entity.get::<CompA>(), None);
        assert_eq!(entity.get::<CompB>(), Some(&component2));
        let entity = world.get_entity(level2).unwrap();
        assert_eq!(entity.get::<CompA>(), None);
        assert_eq!(entity.get::<CompB>(), Some(&component2));

        world.entity_mut(base).remove::<CompB>();
        let entity = world.get_entity(level1).unwrap();
        assert_eq!(entity.get::<CompA>(), None);
        assert_eq!(entity.get::<CompB>(), None);
        let entity = world.get_entity(level2).unwrap();
        assert_eq!(entity.get::<CompA>(), None);
        assert_eq!(entity.get::<CompB>(), None);
    }

    #[test]
    fn inherit_from_shared_table() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA;
        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompB;
        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompC;

        let base1 = world.spawn(CompA).id();
        let base2 = world.spawn(CompB).id();
        let _inherited1 = world.spawn((CompC, InheritFrom(base1))).id();
        let _inherited2 = world.spawn((CompC, InheritFrom(base2))).id();

        let mut query = world.query::<&CompB>();
        assert_eq!(query.iter(&world).len(), 2);
    }

    #[test]
    fn inherited_with_normal_components() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA;
        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompB;

        let base = world.spawn(CompA).id();
        let inherited = world.spawn((CompB, InheritFrom(base))).id();

        let mut query = world.query::<(&CompB, &CompB)>();
        assert!(query.get(&world, inherited).is_ok());
    }

    #[test]
    fn query_inherited_component() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component = CompA(6);

        let base = world.spawn(component).id();
        let inherited = world.spawn(InheritFrom(base)).id();
        world.flush();

        let mut query = world.query::<&CompA>();
        assert_eq!(query.get(&world, inherited), Ok(&component));
        assert_eq!(query.iter(&world).map(|c| c.0).sum::<i32>(), 12);

        let mut query = world.query_filtered::<Entity, With<CompA>>();
        assert_eq!(query.iter(&world).len(), 2);
    }

    #[test]
    fn skip_inherited_if_mutable() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component = CompA(6);
        let component_id = world.register_component::<CompA>();

        let base = world.spawn(component).id();
        let inherited = world.spawn(InheritFrom(base)).id();

        assert!(world.get_mut::<CompA>(inherited).is_none());
        assert!(world.get_mut_by_id(inherited, component_id).is_none());

        let mut query = world.query::<&mut CompA>();
        assert!(query.get(&world, inherited).is_err());
        assert_eq!(query.iter(&world).len(), 1);
    }

    #[test]
    fn inherited_components_circular() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA;

        let entity1 = world.spawn(CompA).id();
        let entity2 = world.spawn_empty().id();

        world.entity_mut(entity1).insert(InheritFrom(entity2));
        world.entity_mut(entity2).insert(InheritFrom(entity1));
    }

    #[test]
    fn inherited_components_with_despawned_base() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA;

        let base = world.spawn(CompA).id();
        let entity = world.spawn(InheritFrom(base)).id();

        world.despawn(base);

        assert_eq!(world.entity(entity).get::<CompA>(), None);
        let mut query = world.query::<&CompA>();
        assert_eq!(query.iter(&world).next(), None);
    }

    #[test]
    fn inherited_components_ref() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(bool);

        let comp = CompA(true);

        let base = world.spawn(comp).id();
        let entity = world.spawn(InheritFrom(base)).id();

        assert_eq!(
            world.entity(entity).get_ref::<CompA>().map(Ref::into_inner),
            Some(&comp)
        );
        let mut query = world.query::<Ref<CompA>>();
        assert_eq!(query.iter(&world).next().map(Ref::into_inner), Some(&comp));
    }

    #[test]
    fn inherited_components_system() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA;

        let base = world.spawn(CompA).id();
        let inherited = world.spawn(InheritFrom(base)).id();

        let query_system = move |query: Query<Entity, With<CompA>>| {
            assert!(query.contains(base));
            assert!(query.contains(inherited));
        };

        let mut system = IntoSystem::into_system(query_system);
        system.initialize(&mut world);
        system.update_archetype_component_access(world.as_unsafe_world_cell());

        system.run((), &mut world);
    }

    #[test]
    #[should_panic]
    fn inherited_components_system_conflict() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let base = world.spawn(CompA(10)).id();
        let _inherited = world.spawn(InheritFrom(base)).id();

        fn query_system(
            _q1: Query<&mut CompA, With<Inherited>>,
            _q2: Query<&CompA, Without<Inherited>>,
        ) {
        }
        let mut system = IntoSystem::into_system(query_system);
        system.initialize(&mut world);
        system.update_archetype_component_access(world.as_unsafe_world_cell());

        system.run((), &mut world);
    }

    #[test]
    #[ignore = "Properly supporting required components needs first-class fragmenting value components or similar"]
    fn inherited_with_required_components() {
        let mut world = World::new();

        #[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
        #[require(CompB)]
        struct CompA;

        #[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
        struct CompB(bool);

        let base = world.spawn(CompB(true)).id();
        let inherited = world.spawn((InheritFrom(base), CompA)).id();

        assert_eq!(world.get::<CompB>(inherited), Some(&CompB(true)));
    }

    #[test]
    #[ignore = "Mutable inherited components support is not yet implemented"]
    fn inherited_mutable() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component = CompA(6);

        let base = world.spawn(component).id();
        let inherited = world.spawn(InheritFrom(base)).id();

        let mut comp = world.get_mut::<CompA>(inherited).unwrap();
        comp.0 = 4;
        world.flush();

        let mut query = world.query::<&CompA>();
        assert_eq!(query.iter(&world).map(|c| c.0).sum::<i32>(), 10);
    }
}
