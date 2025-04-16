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
use bevy_platform_support::sync::Mutex;
use bumpalo::Bump;
use core::mem::offset_of;
use core::{
    alloc::Layout,
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    panic::Location,
    ptr::NonNull,
};
use std::boxed::Box;

use bevy_ptr::{OwningPtr, PtrMut};

use crate::change_detection::TicksMut;
use crate::query::DebugCheckedUnwrap;
use crate::{
    archetype::{ArchetypeComponentId, ArchetypeEntity, ArchetypeId, ArchetypeRecord, Archetypes},
    change_detection::MaybeLocation,
    component::{
        Component, ComponentCloneBehavior, ComponentDescriptor, ComponentId, ComponentInfo,
        HookContext, StorageType, Tick,
    },
    entity::{Entities, Entity, EntityHashMap, EntityLocation},
    prelude::{DetectChanges, DetectChangesMut},
    storage::{TableId, TableRow, Tables},
    world::{DeferredWorld, Mut, World},
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum InheritedArchetypeComponent {
    Sparse {
        archetype_component_id: ArchetypeComponentId,
        entity: Entity,
    },
    Table {
        archetype_component_id: ArchetypeComponentId,
        table_id: TableId,
        table_row: TableRow,
    },
}

impl InheritedArchetypeComponent {
    pub fn archetype_component_id(&self) -> ArchetypeComponentId {
        match self {
            InheritedArchetypeComponent::Table {
                archetype_component_id,
                ..
            }
            | InheritedArchetypeComponent::Sparse {
                archetype_component_id,
                ..
            } => *archetype_component_id,
        }
    }
}

pub(crate) struct InheritedTableComponent {
    pub(crate) table_id: TableId,
    pub(crate) table_row: TableRow,
}

#[derive(Debug)]
pub(crate) struct SharedMutComponentData {
    pub(crate) component_info: ComponentInfo,
    pub(crate) component_ptrs: UnsafeCell<HashMap<usize, NonNull<u8>>>,
    pub(crate) bump: Bump,
}

unsafe impl Send for SharedMutComponentData {}
// SharedMutComponentData is NOT actually Sync, we need either a mutex or a concurrent hashmap + bumpalo-herd here.
// But this shouldn't affect performance too much since this only matters whenever an inherited mutable component
// is encountered.
unsafe impl Sync for SharedMutComponentData {}

impl SharedMutComponentData {
    #[cold]
    #[inline(never)]
    pub unsafe fn get_or_clone<'w, T>(&self, data: &'w mut T, storage_idx: usize) -> &'w mut T {
        // let data = self.data_ptr.cast::<T>().as_ref();
        self.component_ptrs
            .get()
            .as_mut()
            .debug_checked_unwrap()
            .entry(storage_idx)
            .or_insert_with(|| {
                let data_ptr = self.bump.alloc_layout(self.component_info.layout());
                // TODO: use clone function from component_info
                core::ptr::copy_nonoverlapping(data, data_ptr.as_ptr().cast(), 1);
                data_ptr
            })
            .cast::<T>()
            .as_mut()
    }
}

#[derive(Debug)]
pub struct MutInherited<'w, T> {
    pub(crate) original_data: Mut<'w, T>,
    pub(crate) is_inherited: bool,
    pub(crate) shared_data: Option<&'w SharedMutComponentData>,
    pub(crate) table_row: usize,
}

impl<'w, T> MutInherited<'w, T> {
    pub fn ptr(&mut self) -> &mut Mut<'w, T> {
        &mut self.original_data
    }

    pub fn into_inner(mut self) -> &'w mut T {
        if self.is_inherited {
            unsafe {
                self.shared_data
                    .debug_checked_unwrap()
                    .get_or_clone::<T>(self.original_data.value, self.table_row)
            }
        } else {
            self.original_data.set_changed();
            self.original_data.value
        }
    }

    pub fn map_unchanged<U>(self, f: impl FnOnce(&mut T) -> &mut U) -> MutInherited<'w, U> {
        if self.is_inherited {
            unsafe {
                let new_value = f(self
                    .shared_data
                    .debug_checked_unwrap()
                    .get_or_clone::<T>(self.original_data.value, self.table_row));
                MutInherited {
                    original_data: Mut {
                        value: new_value,
                        ticks: self.original_data.ticks,
                        changed_by: self.original_data.changed_by,
                    },
                    is_inherited: self.is_inherited,
                    shared_data: self.shared_data,
                    table_row: self.table_row,
                }
            }
        } else {
            MutInherited {
                original_data: Mut {
                    value: f(self.original_data.value),
                    ticks: self.original_data.ticks,
                    changed_by: self.original_data.changed_by,
                },
                is_inherited: self.is_inherited,
                shared_data: self.shared_data,
                table_row: self.table_row,
            }
        }
    }
}

impl<'w, T> AsRef<T> for MutInherited<'w, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'w, T> Deref for MutInherited<'w, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.original_data.deref()
    }
}

impl<'w, T> AsMut<T> for MutInherited<'w, T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'w, T> DerefMut for MutInherited<'w, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.is_inherited {
            unsafe {
                self.shared_data
                    .debug_checked_unwrap()
                    .get_or_clone::<T>(self.original_data.value, self.table_row)
            }
        } else {
            self.original_data.deref_mut()
        }
    }
}

impl<'w, T> DetectChanges for MutInherited<'w, T> {
    #[inline(always)]
    fn is_added(&self) -> bool {
        self.original_data.is_added()
    }

    #[inline(always)]
    fn is_changed(&self) -> bool {
        self.original_data.is_changed()
    }

    #[inline(always)]
    fn last_changed(&self) -> Tick {
        self.original_data.last_changed()
    }

    #[inline(always)]
    fn changed_by(&self) -> MaybeLocation {
        self.original_data.changed_by()
    }
}

impl<'w, T> DetectChangesMut for MutInherited<'w, T> {
    type Inner = <Mut<'w, T> as DetectChangesMut>::Inner;

    #[inline(always)]
    fn set_changed(&mut self) {
        self.original_data.set_changed();
    }

    #[inline(always)]
    fn set_last_changed(&mut self, last_changed: Tick) {
        self.original_data.set_last_changed(last_changed);
    }

    #[inline(always)]
    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        self.original_data.bypass_change_detection()
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

    /// These need proper multithreading support.
    pub(crate) shared_table_components:
        UnsafeCell<HashMap<(ComponentId, TableId), SharedMutComponentData>>,
    pub(crate) shared_sparse_components:
        UnsafeCell<HashMap<(ComponentId, ArchetypeId), SharedMutComponentData>>,
}

impl InheritedComponents {
    /// This method must be called after a new archetype is created to initialized inherited components once.
    pub(crate) fn init_inherited_components(
        &mut self,
        entities: &Entities,
        archetypes: &mut Archetypes,
        tables: &mut Tables,
        archetype_id: ArchetypeId,
    ) {
        let archetype = &archetypes.archetypes[archetype_id.index()];
        if !archetype.has_inherited_components() {
            return;
        }
        let archetype_inherited_components: HashMap<ComponentId, InheritedArchetypeComponent> =
            archetype
                .table_components()
                .filter_map(|id| {
                    self.ids_to_entities.get(&id).and_then(|entity| {
                        entities.get(*entity).and_then(|location| {
                            archetypes
                                .get(location.archetype_id)
                                .map(|archetype| (archetype, *entity, location))
                        })
                    })
                })
                .flat_map(|(inherited_archetype, entity, location)| {
                    inherited_archetype
                        .components_with_archetype_component_id()
                        .map(move |(component_id, archetype_component_id)| {
                            (
                                component_id,
                                match inherited_archetype.get_storage_type(component_id).unwrap() {
                                    StorageType::Table => InheritedArchetypeComponent::Table {
                                        archetype_component_id,
                                        table_id: location.table_id,
                                        table_row: location.table_row,
                                    },
                                    StorageType::SparseSet => InheritedArchetypeComponent::Sparse {
                                        archetype_component_id,
                                        entity,
                                    },
                                },
                            )
                        })
                        .chain(
                            archetypes
                                .get(inherited_archetype.id())
                                .unwrap()
                                .inherited_components
                                .iter()
                                .map(|(a, b)| (*a, *b)),
                        )
                })
                .filter(|(component, ..)| !archetype.contains(*component))
                .collect();

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
        let table_id = archetype.table_id();
        if tables[table_id].inherited_components.is_empty() {
            let inherited_components = archetype_inherited_components
                .iter()
                .filter_map(|(&component_id, component)| match component {
                    InheritedArchetypeComponent::Sparse { .. } => None,
                    &InheritedArchetypeComponent::Table {
                        table_id,
                        table_row,
                        ..
                    } => Some((
                        component_id,
                        InheritedTableComponent {
                            table_id,
                            table_row,
                        },
                    )),
                })
                .collect();
            tables[table_id].inherited_components = inherited_components;
        }

        let archetype = &mut archetypes[archetype_id];
        archetype.inherited_components = archetype_inherited_components;
    }

    /// This method must be called after an entity moves to update all inherited archetypes/tables.
    pub(crate) fn update_inherited_archetypes<const UPDATE_TABLES: bool>(
        &mut self,
        archetypes: &mut Archetypes,
        tables: &mut Tables,
        new_archetype_id: ArchetypeId,
        old_archetype_id: ArchetypeId,
        entity: Entity,
        new_location: EntityLocation,
    ) {
        let Some(component_id) = self.entities_to_ids.get(&entity) else {
            return;
        };
        let new_archetype = &archetypes[new_archetype_id];
        let new_components: HashMap<_, _> = new_archetype
            .components_with_archetype_component_id()
            .map(|(component_id, archetype_component_id)| {
                (
                    component_id,
                    (
                        archetype_component_id,
                        new_archetype.get_storage_type(component_id).unwrap(),
                    ),
                )
            })
            .collect();
        let removed_components: HashSet<_> = archetypes[old_archetype_id]
            .components()
            .filter(|&component_id| !new_archetype.contains(component_id))
            .collect();

        let mut base_entities = VecDeque::from([(entity, *component_id)]);
        let mut inherited_archetypes = Vec::new();
        let mut processed_archetypes = HashSet::<ArchetypeId>::default();

        while let Some((entity, inherited_entity_component_id)) = base_entities.pop_front() {
            if let Some(map) = archetypes.by_component.get(&inherited_entity_component_id) {
                inherited_archetypes.extend(
                    map.keys()
                        .copied()
                        .filter(|archetype_id| !processed_archetypes.contains(archetype_id)),
                );
            };
            for archetype_id in inherited_archetypes.drain(..) {
                // Update archetype's inherited components
                let mut archetype_inherited_components =
                    core::mem::take(&mut archetypes[archetype_id].inherited_components);
                archetype_inherited_components.retain(|component_id, _| {
                    !new_components.contains_key(component_id)
                        && !removed_components.contains(component_id)
                });
                archetype_inherited_components.extend(new_components.iter().map(
                    |(&component_id, &(archetype_component_id, storage_type))| {
                        (
                            component_id,
                            match storage_type {
                                StorageType::Table => InheritedArchetypeComponent::Table {
                                    archetype_component_id,
                                    table_id: new_location.table_id,
                                    table_row: new_location.table_row,
                                },
                                StorageType::SparseSet => InheritedArchetypeComponent::Sparse {
                                    archetype_component_id,
                                    entity,
                                },
                            },
                        )
                    },
                ));
                archetypes[archetype_id].inherited_components = archetype_inherited_components;

                // Update component index
                for &component in new_components.keys() {
                    archetypes.by_component.entry(component).and_modify(|map| {
                        map.insert(
                            new_archetype_id,
                            ArchetypeRecord {
                                column: None,
                                is_inherited: true,
                            },
                        );
                    });
                }

                // Update archetype table's inherited components
                // This needs to be done only if entity moved tables
                if UPDATE_TABLES {
                    let table_id = archetypes[archetype_id].table_id();
                    let mut table_inherited_components =
                        core::mem::take(&mut tables[table_id].inherited_components);
                    let table = &tables[table_id];
                    table_inherited_components.retain(|component_id, _| {
                        !new_components.contains_key(component_id)
                            && !removed_components.contains(component_id)
                    });
                    table_inherited_components.extend(new_components.iter().filter_map(
                        |(&component_id, &(_, storage_type))| {
                            if !table.has_column(component_id) && storage_type == StorageType::Table
                            {
                                Some((
                                    component_id,
                                    InheritedTableComponent {
                                        table_id: new_location.table_id,
                                        table_row: new_location.table_row,
                                    },
                                ))
                            } else {
                                None
                            }
                        },
                    ));
                    tables[table_id].inherited_components = table_inherited_components;
                }

                let archetype = &archetypes[archetype_id];
                if archetype.is_inherited() {
                    for (entity, component_id) in archetype.entities().iter().filter_map(|entity| {
                        self.entities_to_ids
                            .get(&entity.id())
                            .map(|component_id| (entity.id(), *component_id))
                    }) {
                        base_entities.push_back((entity, component_id));
                    }
                    processed_archetypes.insert(archetype.id());
                }
            }
        }
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
    fn inherited_mutable() {
        let mut world = World::new();

        #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
        struct CompA(i32);

        let component = CompA(6);

        let base = world.spawn(component).id();
        let inherited1 = world.spawn(InheritFrom(base)).id();
        let inherited2 = world.spawn(InheritFrom(base)).id();

        // let mut comp = world.get_mut::<CompA>(inherited).unwrap();
        let mut comp = world
            .query::<&mut CompA>()
            .get_mut(&mut world, inherited1)
            .unwrap();
        comp.0 = 4;
        let mut entity2 = world.get_entity_mut(inherited2).unwrap();
        let mut comp2 = entity2.get_mut::<CompA>().unwrap();
        comp2.0 = 1;
        world.flush();

        let mut query = world.query::<&CompA>();
        assert_eq!(query.iter(&world).map(|c| c.0).sum::<i32>(), 11);
    }
}
