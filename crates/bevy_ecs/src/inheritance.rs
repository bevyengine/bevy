//! This module contains code relating to component inheritance.
//!
//! [`InheritedComponents`] is the main structure holding data about inherited components, it can be used
//! to record and resolve archetypes/tables that contain components from an entity in another archetype/table.
//!
//! [`InheritFrom`] is the main user-facing component that allows some entity to inherit components from some other entity.
use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::string::ToString;
use alloc::vec::Vec;
use bevy_platform::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};
use bumpalo::Bump;
use core::{
    alloc::Layout,
    ops::{Deref, DerefMut},
    panic::Location,
    ptr::NonNull,
};

use bevy_ptr::OwningPtr;

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
use crate::{change_detection::TicksMut, query::DebugCheckedUnwrap};

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

#[derive(Clone)]
pub(crate) struct MutComponentSharedData {
    pub(crate) component_info: ComponentInfo,
    pub(crate) bump: NonNull<Mutex<Bump>>,
    pub(crate) component_ptrs: NonNull<Mutex<HashMap<usize, MutComponentPtrs>>>,
}

impl Drop for MutComponentSharedData {
    fn drop(&mut self) {
        unsafe { drop(bumpalo::boxed::Box::from_raw(self.component_ptrs.as_ptr())) }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum InheritedBehavior {
    Disabled,
    Shared,
}

impl MutComponentSharedData {
    #[inline]
    fn alloc(
        bump: &Mutex<Bump>,
        component_info: &ComponentInfo,
    ) -> NonNull<MutComponentSharedData> {
        let bump_lock = bump.lock().unwrap();
        let component_ptrs = unsafe {
            NonNull::new_unchecked(bumpalo::boxed::Box::into_raw(bumpalo::boxed::Box::new_in(
                Default::default(),
                &bump_lock,
            )))
        };
        let shared_data = bumpalo::boxed::Box::new_in(
            MutComponentSharedData {
                bump: NonNull::from(bump),
                component_info: component_info.clone(),
                component_ptrs,
            },
            &bump_lock,
        );
        unsafe { NonNull::new_unchecked(bumpalo::boxed::Box::into_raw(shared_data)) }
    }

    #[inline]
    unsafe fn from_ptr<'a>(
        shared_data: NonNull<MutComponentSharedData>,
    ) -> bumpalo::boxed::Box<'a, MutComponentSharedData> {
        unsafe { bumpalo::boxed::Box::from_raw(shared_data.as_ptr()) }
    }

    #[inline]
    fn components(&self) -> &Mutex<HashMap<usize, MutComponentPtrs>> {
        unsafe { self.component_ptrs.as_ref() }
    }

    #[inline]
    fn get_or_cloned<'w, T: Component>(
        &self,
        data: &'w T,
        table_row: TableRow,
        this_run: Tick,
        caller: MaybeLocation,
    ) -> ComponentMut<'w, T> {
        unsafe {
            let mut lock = self.component_ptrs.as_ref().lock().unwrap();
            let ptrs = lock.entry(table_row.as_usize()).or_insert_with(|| {
                let bump_lock = self.bump.as_ref().lock().unwrap();
                let value = bump_lock.alloc_layout(self.component_info.layout());
                let added = bump_lock.alloc(this_run);
                let changed = bump_lock.alloc(this_run);
                let changed_by = caller.map(|l| NonNull::from(bump_lock.alloc(l)));
                // TODO: use clone function from component_info
                core::ptr::copy_nonoverlapping(data, value.as_ptr().cast(), 1);

                MutComponentPtrs {
                    value,
                    added: NonNull::from(added),
                    changed: NonNull::from(changed),
                    changed_by,
                }
            });
            ComponentMut {
                value: ptrs.value.cast().as_mut(),
                added: ptrs.added.as_mut(),
                changed: ptrs.changed.as_mut(),
                changed_by: ptrs.changed_by.map(|mut l| l.as_mut()),
            }
        }
    }

    #[inline]
    fn try_get<'w, T: Component>(&self, table_row: TableRow) -> Option<ComponentRef<'w, T>> {
        unsafe {
            self.component_ptrs
                .as_ref()
                .lock()
                .unwrap()
                .get(&table_row.as_usize())
                .map(|ptrs| ComponentRef {
                    value: ptrs.value.cast().as_ref(),
                    added: ptrs.added.as_ref(),
                    changed: ptrs.changed.as_ref(),
                    changed_by: ptrs.changed_by.map(|l| l.as_ref()).copied(),
                })
        }
    }
}

#[derive(Clone, Copy)]
pub struct MutComponentPtrs {
    pub(crate) value: NonNull<u8>,
    pub(crate) added: NonNull<Tick>,
    pub(crate) changed: NonNull<Tick>,
    pub(crate) changed_by: MaybeLocation<NonNull<&'static Location<'static>>>,
}

pub struct MutComponent<'w, T: Component> {
    pub(crate) value: NonNull<T>,
    pub(crate) added: NonNull<Tick>,
    pub(crate) changed: NonNull<Tick>,
    pub(crate) changed_by: MaybeLocation<NonNull<&'static Location<'static>>>,

    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
    pub(crate) is_shared: bool,
    pub(crate) shared_data: Option<&'w MutComponentSharedData>,
    pub(crate) table_row: TableRow,
}

struct ComponentRef<'w, T: Component> {
    value: &'w T,
    added: &'w Tick,
    changed: &'w Tick,
    changed_by: MaybeLocation,
}

struct ComponentMut<'w, T: Component> {
    value: &'w mut T,
    added: &'w mut Tick,
    changed: &'w mut Tick,
    changed_by: MaybeLocation<&'w mut &'static Location<'static>>,
}

impl<'w, T: Component> MutComponent<'w, T> {
    #[inline(always)]
    fn dispatch<R>(&self, func: impl FnOnce(ComponentRef<'w, T>) -> R) -> R {
        let (value, added, changed, changed_by) = unsafe {
            (
                self.value.cast().as_ref(),
                self.added.as_ref(),
                self.changed.as_ref(),
                self.changed_by.map(|l| l.as_ref()).copied(),
            )
        };

        if T::INHERITED_BEHAVIOR == InheritedBehavior::Shared && self.is_shared {
            #[cold]
            #[inline(never)]
            fn unlikely<'w, R, T: Component>(
                func: impl FnOnce(ComponentRef<'w, T>) -> R,
                shared_data: &'w MutComponentSharedData,
                value: &'w T,
                added: &'w Tick,
                changed: &'w Tick,
                changed_by: MaybeLocation,
                table_row: TableRow,
            ) -> R {
                func(shared_data.try_get(table_row).unwrap_or(ComponentRef {
                    value,
                    added,
                    changed,
                    changed_by,
                }))
            }

            unlikely(
                func,
                unsafe { self.shared_data.debug_checked_unwrap() },
                value,
                added,
                changed,
                changed_by,
                self.table_row,
            )
        } else {
            func(ComponentRef {
                value,
                added,
                changed,
                changed_by,
            })
        }
    }

    #[track_caller]
    fn dispatch_mut<const CHANGE: bool, R>(
        &mut self,
        func: impl FnOnce(ComponentMut<'w, T>) -> R,
    ) -> R {
        if T::INHERITED_BEHAVIOR == InheritedBehavior::Shared && self.is_shared {
            #[cold]
            #[inline(never)]
            #[track_caller]
            fn unlikely<'w, R, T: Component, const CHANGE: bool>(
                func: impl FnOnce(ComponentMut<'w, T>) -> R,
                shared_data: &'w MutComponentSharedData,
                value: &'w T,
                this_run: Tick,
                table_row: TableRow,
                changed_by: MaybeLocation,
            ) -> R {
                let mut values = shared_data.get_or_cloned(value, table_row, this_run, changed_by);
                if CHANGE {
                    values.changed_by.assign(MaybeLocation::caller());
                }
                func(values)
            }
            let (shared_data, value, this_run, table_row, changed_by) = unsafe {
                (
                    self.shared_data.debug_checked_unwrap(),
                    self.value.cast().as_ref(),
                    self.this_run,
                    self.table_row,
                    self.changed_by.map(|l| l.as_ref()).copied(),
                )
            };

            unlikely::<_, _, CHANGE>(func, shared_data, value, this_run, table_row, changed_by)
        } else {
            let (value, added, changed, mut changed_by) = unsafe {
                (
                    &mut *self.value.as_ptr(),
                    &mut *self.added.as_ptr(),
                    &mut *self.changed.as_ptr(),
                    self.changed_by.map(|l| &mut *l.as_ptr()),
                )
            };
            if CHANGE {
                *changed = self.this_run;
                changed_by.assign(MaybeLocation::caller());
            }

            func(ComponentMut {
                value,
                added,
                changed,
                changed_by,
            })
        }
    }
}

impl<'w, T: Component> MutComponent<'w, T> {
    #[track_caller]
    #[inline(always)]
    pub fn into_inner(mut self) -> &'w mut T {
        self.dispatch_mut::<true, _>(|args| args.value)
    }

    #[inline(always)]
    pub fn map_unchanged<U: ?Sized>(mut self, f: impl FnOnce(&mut T) -> &mut U) -> Mut<'w, U> {
        let last_run = self.last_run;
        let this_run = self.this_run;
        self.dispatch_mut::<false, _>(|args| Mut {
            value: f(args.value),
            changed_by: args.changed_by,
            ticks: TicksMut {
                added: args.added,
                changed: args.changed,
                last_run,
                this_run,
            },
        })
    }

    #[inline(always)]
    pub fn reborrow(&mut self) -> MutComponent<'_, T> {
        MutComponent { ..*self }
    }

    #[inline(always)]
    pub fn filter_map_unchanged<U: ?Sized>(
        mut self,
        f: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Option<Mut<'w, U>> {
        let last_run = self.last_run;
        let this_run = self.this_run;
        self.dispatch_mut::<false, _>(|args| {
            f(args.value).map(|value| Mut {
                value,
                changed_by: args.changed_by,
                ticks: TicksMut {
                    added: args.added,
                    changed: args.changed,
                    last_run,
                    this_run,
                },
            })
        })
    }

    #[inline(always)]
    pub fn try_map_unchanged<U: ?Sized, E>(
        mut self,
        f: impl FnOnce(&mut T) -> Result<&mut U, E>,
    ) -> Result<Mut<'w, U>, E> {
        let last_run = self.last_run;
        let this_run = self.this_run;
        self.dispatch_mut::<false, _>(|args| {
            f(args.value).map(|value| Mut {
                value,
                changed_by: args.changed_by,
                ticks: TicksMut {
                    added: args.added,
                    changed: args.changed,
                    last_run,
                    this_run,
                },
            })
        })
    }

    #[inline(always)]
    pub fn as_deref_mut(&mut self) -> Mut<'_, <T as Deref>::Target>
    where
        T: DerefMut,
    {
        self.reborrow().map_unchanged(|v| v.deref_mut())
    }

    #[inline(always)]
    pub fn as_exclusive(&mut self) -> Mut<'w, T> {
        let last_run = self.last_run;
        let this_run = self.this_run;
        self.dispatch_mut::<false, _>(|args| Mut {
            value: args.value,
            changed_by: args.changed_by,
            ticks: TicksMut {
                added: args.added,
                changed: args.changed,
                last_run,
                this_run,
            },
        })
    }
}

impl<'w, T: Component> AsRef<T> for MutComponent<'w, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'w, T: Component> Deref for MutComponent<'w, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.dispatch(|args| args.value)
    }
}

impl<'w, T: Component> AsMut<T> for MutComponent<'w, T> {
    #[track_caller]
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'w, T: Component> DerefMut for MutComponent<'w, T> {
    #[track_caller]
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dispatch_mut::<true, _>(|args| args.value)
    }
}

impl<'w, T: Component> DetectChanges for MutComponent<'w, T> {
    #[inline(always)]
    fn is_added(&self) -> bool {
        self.dispatch(|args| args.added.is_newer_than(self.last_run, self.this_run))
    }

    #[inline(always)]
    fn is_changed(&self) -> bool {
        self.dispatch(|args| args.changed.is_newer_than(self.last_run, self.this_run))
    }

    #[inline(always)]
    fn last_changed(&self) -> Tick {
        self.dispatch(|args| *args.changed)
    }

    #[inline(always)]
    fn changed_by(&self) -> MaybeLocation {
        self.dispatch(|args| args.changed_by)
    }

    #[inline(always)]
    fn added(&self) -> Tick {
        self.dispatch(|args| *args.added)
    }
}

impl<'w, T: Component> DetectChangesMut for MutComponent<'w, T> {
    type Inner = <Mut<'w, T> as DetectChangesMut>::Inner;

    #[inline(always)]
    #[track_caller]
    fn set_changed(&mut self) {
        self.dispatch_mut::<true, _>(|_| {});
    }

    #[inline(always)]
    #[track_caller]
    fn set_last_changed(&mut self, last_changed: Tick) {
        self.dispatch_mut::<true, _>(|args| *args.changed = last_changed);
    }

    #[inline(always)]
    fn bypass_change_detection(&mut self) -> &mut Self::Inner {
        self.dispatch_mut::<false, _>(|args| args.value)
    }

    #[inline(always)]
    #[track_caller]
    fn set_added(&mut self) {
        let this_run = self.this_run;
        self.dispatch_mut::<true, _>(|args| *args.added = this_run);
    }

    #[inline(always)]
    #[track_caller]
    fn set_last_added(&mut self, last_added: Tick) {
        self.dispatch_mut::<true, _>(|args| {
            *args.added = last_added;
            *args.changed = last_added;
        });
    }
}

impl<'w, T: Component> core::fmt::Debug for MutComponent<'w, T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple(stringify!(MutComponent))
            .field(self.dispatch(|original| original.value))
            .finish()
    }
}

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

    pub(crate) queued_shared_mutations_level: usize,
    pub(crate) shared_table_components:
        Mutex<HashMap<(ComponentId, TableId), NonNull<MutComponentSharedData>>>,
    pub(crate) shared_sparse_components:
        Mutex<HashMap<(ComponentId, ArchetypeId), NonNull<MutComponentSharedData>>>,
    pub(crate) shared_components_bump: NonNull<Mutex<Bump>>,
}

impl Default for InheritedComponents {
    fn default() -> Self {
        Self {
            entities_to_ids: Default::default(),
            ids_to_entities: Default::default(),
            queued_shared_mutations_level: Default::default(),
            shared_table_components: Default::default(),
            shared_sparse_components: Default::default(),
            shared_components_bump: unsafe {
                NonNull::new_unchecked(Box::into_raw(Box::default()))
            },
        }
    }
}

impl InheritedComponents {
    fn shared_components_bump(&self) -> &Mutex<Bump> {
        unsafe { self.shared_components_bump.as_ref() }
    }

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

    pub(crate) fn apply_queued_shared_mutations(world: &mut World) {
        world.inherited_components.queued_shared_mutations_level += 1;

        let mut queue: EntityHashMap<(Vec<ComponentId>, Vec<MutComponentPtrs>)> =
            EntityHashMap::default();

        for ((component_id, table_id), shared_data) in world
            .inherited_components
            .shared_table_components
            .lock()
            .unwrap()
            .drain()
        {
            let shared_data = unsafe { MutComponentSharedData::from_ptr(shared_data) };
            let components = shared_data.components().lock().unwrap();
            for (table_row, ptrs) in components.iter() {
                let entity = world.storages.tables[table_id].entities()[*table_row];
                let (components_queue, ptrs_queue, ..) = queue
                    .entry(entity)
                    .or_insert_with(|| (Vec::default(), Vec::default()));
                components_queue.push(component_id);
                ptrs_queue.push(*ptrs);
            }
        }

        for ((component_id, archetype_id), shared_data) in world
            .inherited_components
            .shared_sparse_components
            .lock()
            .unwrap()
            .drain()
        {
            let shared_data = unsafe { MutComponentSharedData::from_ptr(shared_data) };
            let components = shared_data.components().lock().unwrap();
            for (table_row, ptrs) in components.iter() {
                let entity = world.archetypes[archetype_id].entities()[*table_row].id();
                let (components_queue, ptrs_queue, ..) = queue
                    .entry(entity)
                    .or_insert_with(|| (Vec::default(), Vec::default()));
                components_queue.push(component_id);
                ptrs_queue.push(*ptrs);
            }
        }

        for (entity, (component_ids, component_ptrs)) in queue.drain() {
            unsafe {
                world.entity_mut(entity).insert_by_ids(
                    &component_ids,
                    component_ptrs.iter().map(|ptrs| OwningPtr::new(ptrs.value)),
                );

                // Fixup change detection fields after the fact since BundleInserter can't set them on per-component basis.
                // This means that these fields might be inconsistent when observed inside hooks and observers.
                for (ptrs, component_id) in component_ptrs.iter().zip(component_ids.iter()) {
                    let mut entity = world.entity_mut(entity);
                    let Ok(mut component) = entity.get_mut_by_id(*component_id) else {
                        continue;
                    };
                    *component.ticks.added = ptrs.added.read();
                    *component.ticks.changed = ptrs.changed.read();
                    component
                        .changed_by
                        .assign(ptrs.changed_by.map(|l| l.read()));
                }
            }
        }

        // entity_mut.insert_by_ids calls flush, which might reenter this function.
        // To prevent clearing bump before all components are written, we keep track of
        // recursion level and clear only on the most outer level.
        world.inherited_components.queued_shared_mutations_level -= 1;
        if world.inherited_components.queued_shared_mutations_level == 0 {
            world
                .inherited_components
                .shared_components_bump()
                .lock()
                .unwrap()
                .reset();
        }
    }

    #[cold]
    #[inline(always)]
    pub(crate) fn get_shared_table_component_data<'w>(
        &'w self,
        component_info: &ComponentInfo,
        table_id: TableId,
    ) -> &'w MutComponentSharedData {
        let mut lock = self.shared_table_components.lock().unwrap();
        let ptr = lock
            .entry((component_info.id(), table_id))
            .or_insert_with(|| {
                MutComponentSharedData::alloc(self.shared_components_bump(), component_info)
            });
        unsafe { ptr.as_ref() }
    }

    #[cold]
    #[inline(always)]
    pub(crate) fn get_shared_sparse_component_data<'w>(
        &'w self,
        component_info: &ComponentInfo,
        archetype_id: ArchetypeId,
    ) -> &'w MutComponentSharedData {
        let mut lock = self.shared_sparse_components.lock().unwrap();
        let ptr = lock
            .entry((component_info.id(), archetype_id))
            .or_insert_with(|| {
                MutComponentSharedData::alloc(self.shared_components_bump(), component_info)
            });
        unsafe { ptr.as_ref() }
    }
}

impl Drop for InheritedComponents {
    fn drop(&mut self) {
        self.shared_table_components
            .lock()
            .unwrap()
            .values()
            .chain(self.shared_sparse_components.lock().unwrap().values())
            .for_each(|ptr| unsafe {
                drop(MutComponentSharedData::from_ptr(*ptr));
            });
        unsafe {
            drop(Box::from_raw(self.shared_components_bump.as_ptr()));
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
