use bevy_ptr::{Ptr, UnsafeCellDeref};

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, StorageType, Tick},
    entity::Entity,
    prelude::World,
    query::{Access, DebugCheckedUnwrap, FilteredAccess},
    storage::{ComponentSparseSet, Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};

/// Defines whether a [`Term`] has mutable or immutable access to a [`Component`](crate::prelude::Component) or [`Entity`].
#[derive(Eq, PartialEq, Clone, Copy, Debug, Default, PartialOrd, Ord)]
pub enum TermAccess {
    /// This term doesn't access it's target at all i.e. `With<T>` or `Entity`
    #[default]
    None = 0,
    /// This term has immutable access to it's target i.e. `&T` or `EntityRef`
    Read,
    /// This term has mutable access to it's target i.e. `&mut T` or `EntityMut`
    Write,
}

impl TermAccess {
    /// True if this term doesn't access it's target at all
    pub fn is_none(&self) -> bool {
        self == &TermAccess::None
    }
}

/// Defines possible operators for a [`Term`].
#[derive(Clone, Default, Eq, PartialEq, Copy, Debug)]
pub enum TermOperator {
    /// An [`Entity`] must have the associated component to match this term
    #[default]
    With,
    /// An [`Entity`] must not have the associated component to match this term
    Without,
    /// An [`Entity`] must have the associated component and it's value
    /// must have changed since the last time this query was run
    Changed,
    /// An [`Entity`] must have the associated component and it was
    /// added since the last time this query was run
    Added,
    /// An [`Entity`] will always match an optional term
    Optional,
}

/// A single term in a [`TermQuery`](crate::prelude::TermQuery), each valid [`QueryTerm`](crate::prelude::QueryTerm) generates a
/// matching [`Term`] in [`QueryTerm::init_term`](crate::prelude::QueryTerm).
///
/// The [`Term`] is used while resolving a query to determine how the
/// resulting [`FetchedTerm`] is populated.
#[derive(Clone, Default, Debug)]
pub struct Term {
    /// The [`Component`](crate::prelude::Component) this term targets if any, i.e. `&T`, `&mut T`
    pub component: Option<ComponentId>,
    /// Whether this Term reads/writes the component or entity
    pub access: TermAccess,
    /// The operator to use while resolving this term, see [`TermOperator`]
    pub operator: TermOperator,
    /// Whether or not this term requires change detection information i.e. `&mut T` or [`Changed<T>`](crate::prelude::Changed)
    pub change_detection: bool,
    /// Whether or not this term is connected to the following term with an or
    pub or: bool,
}

impl Term {
    /// Set the target [`ComponentId`] of this [`Term`]
    pub fn set_id(mut self, id: ComponentId) -> Self {
        self.component = Some(id);
        self
    }

    /// Set the [`TermOperator`] of this [`Term`]
    pub fn set_operator(mut self, op: TermOperator) -> Self {
        self.operator = op;
        self
    }

    /// Set the [`TermAccess`] of this [`Term`]
    pub fn set_access(mut self, access: TermAccess) -> Self {
        self.access = access;
        self
    }

    /// Creates a term representing [`With<T>`](crate::prelude::With) where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn with_id(id: ComponentId) -> Self {
        Self::default().set_id(id)
    }

    /// Creates a term representing [`Without<T>`](crate::prelude::Without) where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn without_id(id: ComponentId) -> Self {
        Self::default()
            .set_operator(TermOperator::Without)
            .set_id(id)
    }

    /// Creates a term representing [`Ptr`]
    pub fn read() -> Self {
        Self::default().set_access(TermAccess::Read)
    }

    /// Creates a term representing `&T` where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn read_id(id: ComponentId) -> Self {
        Self::read().set_id(id)
    }

    /// Creates a term representing [`PtrMut`](bevy_ptr::PtrMut)
    pub fn write() -> Self {
        Self::default().set_access(TermAccess::Write)
    }

    /// Creates a term representing `&mut T` where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn write_id(id: ComponentId) -> Self {
        Self::write().set_id(id)
    }

    /// Creates a term representing [`Added<T>`](crate::prelude::Added) where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn added_id(id: ComponentId) -> Self {
        Self::with_id(id)
            .set_operator(TermOperator::Added)
            .with_change_detection()
    }

    /// Creates a term representing [`Changed<T>`](crate::prelude::Changed) where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn changed_id(id: ComponentId) -> Self {
        Self::with_id(id)
            .set_operator(TermOperator::Changed)
            .with_change_detection()
    }

    /// Adds change detection as requirement for this term
    pub fn with_change_detection(mut self) -> Self {
        self.change_detection = true;
        self
    }

    /// Returns true if this term is an [`TermOperator::Added`] or [`TermOperator::Changed`] term
    #[inline(always)]
    pub fn filtered(&self) -> bool {
        self.operator == TermOperator::Added || self.operator == TermOperator::Changed
    }

    /// Returns false if the component this term accesses is not a [`StorageType::Table`] component.
    ///
    /// # Safety:
    ///  - caller must ensure any component accesses by this term exists
    #[inline(always)]
    pub unsafe fn dense(&self, world: &World) -> bool {
        if let Some(id) = self.component {
            world.components().get_info_unchecked(id).storage_type() == StorageType::Table
        } else {
            true
        }
    }

    /// Whether this term can be safely interpreted as `other` i.e. `&T => With<T>` or `&mut T => &T`
    pub fn interpretable_as(&self, other: &Term) -> bool {
        self.component == other.component
            && self.operator == other.operator
            && self.access >= other.access
    }
}

// Stores each possible pointer type that could be stored in [`TermState`]
#[derive(Clone)]
pub enum ComponentPtr<'w> {
    // A reference to the components associated sparse set
    SparseSet(&'w ComponentSparseSet),
    // A reference to the components associated table, set in [`Term::set_table`]
    Table(Option<TablePtr<'w>>),
}

impl<'w> ComponentPtr<'w> {
    #[inline(always)]
    pub fn table(&self) -> Option<&TablePtr<'w>> {
        match self {
            Self::Table(Some(ptr)) => Some(ptr),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn table_mut(&mut self) -> Option<&mut Option<TablePtr<'w>>> {
        match self {
            Self::Table(ptr) => Some(ptr),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn sparse_set(&self) -> Option<&'w ComponentSparseSet> {
        match self {
            Self::SparseSet(set) => Some(set),
            _ => None,
        }
    }
}

// Stores state for change detection, ptrs gets set in [`Term::set_table`] for table components
// and is otherwise None
#[derive(Clone)]
pub struct TablePtr<'w> {
    pub component: Ptr<'w>,
    pub added: Option<Ptr<'w>>,
    pub changed: Option<Ptr<'w>>,
}

impl<'w> TablePtr<'w> {
    const TICK_SIZE: usize = std::mem::size_of::<Tick>();

    #[inline(always)]
    pub unsafe fn get_row(&self, size: usize, index: usize) -> Self {
        Self {
            component: self.component.byte_add(size * index),
            added: Some(
                self.added
                    .debug_checked_unwrap()
                    .byte_add(Self::TICK_SIZE * index),
            ),
            changed: Some(
                self.changed
                    .debug_checked_unwrap()
                    .byte_add(Self::TICK_SIZE * index),
            ),
        }
    }
}

#[derive(Clone)]
pub struct ComponentState<'w> {
    pub ptr: ComponentPtr<'w>,
    pub id: ComponentId,
    pub size: usize,
}

// Stores the state for a single term
#[derive(Clone)]
pub struct TermState<'w> {
    // Pointer to wherever we need to fetch data to resolve this term
    pub component: Option<ComponentState<'w>>,
    // Whether this term matches the associated archetype
    pub matches: bool,
}

impl Term {
    /// Creates and initializes a [`TermState`] for this [`Term`].
    ///
    /// # Safety
    ///
    /// - `world` must have permission to access any of the components specified in `Self::update_archetype_component_access`.
    #[inline]
    pub(crate) unsafe fn init_state<'w>(&self, world: UnsafeWorldCell<'w>) -> TermState<'w> {
        if let Some(component_id) = self.component {
            let info = world.components().get_info_unchecked(component_id);
            let storage = info.storage_type();
            match storage {
                // For sparse set components we take a pointer to the associated sparse set and will always match
                StorageType::SparseSet => {
                    let set = world
                        .storages()
                        .sparse_sets
                        .get(component_id)
                        .debug_checked_unwrap();
                    TermState {
                        component: Some(ComponentState {
                            ptr: ComponentPtr::SparseSet(set),
                            id: component_id,
                            size: info.layout().size(),
                        }),
                        matches: true,
                    }
                }
                // For table components we wait until `set_table`
                StorageType::Table => TermState {
                    component: Some(ComponentState {
                        ptr: ComponentPtr::Table(None),
                        id: component_id,
                        size: info.layout().size(),
                    }),
                    matches: false,
                },
            }
        } else {
            TermState {
                component: None,
                matches: true,
            }
        }
    }

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`Term`].
    ///
    /// # Safety
    ///
    /// - `archetype` and `tables` must be from the same [`World`](crate::prelude::World) that [`Self::init_state`] was called on.
    /// - [`Self::update_archetype_component_access`] must have been previously called with `archetype`.
    /// - `table` must correspond to `archetype`.
    /// - `state` must be the must be the same [`TermState`] that was created in [`Self::init_state`].
    #[inline]
    pub(crate) unsafe fn set_archetype<'w>(
        &self,
        state: &mut TermState<'w>,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        state.matches = self.matches_component_set(&|id| archetype.contains(id));
        if state.matches && (self.change_detection || !self.access.is_none()) {
            self.set_table_manual(state, table);
        }
    }

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`Term`].
    ///
    /// # Safety
    ///
    /// - `table` must be from the same [`World`](crate::prelude::World) that [`Self::init_state`] was called on.
    /// - `table` must belong to an archetype that was previously registered with
    ///   [`Self::update_archetype_component_access`].
    /// - `state` must be the same [`TermState`] that was created in [`Self::init_state`].
    #[inline]
    pub(crate) unsafe fn set_table<'w>(&self, state: &mut TermState<'w>, table: &'w Table) {
        state.matches = self.matches_component_set(&|id| table.has_column(id));
        if state.matches && (self.change_detection || !self.access.is_none()) {
            self.set_table_manual(state, table);
        }
    }

    /// Set the table and change tick pointers for table component [`Term`]s.
    #[inline]
    unsafe fn set_table_manual<'w>(&self, state: &mut TermState<'w>, table: &'w Table) {
        if let ComponentPtr::Table(ptr) = &mut state.component.as_mut().debug_checked_unwrap().ptr {
            let component = self.component.debug_checked_unwrap();
            let column = table.get_column(component).debug_checked_unwrap();
            *ptr = Some(TablePtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().get_unchecked(0).into()),
                changed: Some(column.get_changed_ticks_slice().get_unchecked(0).into()),
            });
        }
    }

    /// Fetch whether or not the given `entity` in the current [`Table`] matches this query.
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`Self::set_table`]. `entity` and `table_row` must be
    /// in the range of the current table and archetype.
    #[inline(always)]
    pub(crate) unsafe fn filter_fetch(
        &self,
        state: &TermState<'_>,
        entity: Entity,
        table_row: TableRow,
        last_run: Tick,
        this_run: Tick,
    ) -> bool {
        match self.operator {
            TermOperator::Added => {
                let component = state.component.as_ref().debug_checked_unwrap();
                let added = match &component.ptr {
                    ComponentPtr::SparseSet(set) => {
                        let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                        Some(cells.added.deref())
                    }
                    ComponentPtr::Table(Some(table)) => {
                        let added = table
                            .added
                            .debug_checked_unwrap()
                            .byte_add(std::mem::size_of::<Tick>() * table_row.index());
                        Some(added.deref::<Tick>())
                    }
                    _ => None,
                }
                .debug_checked_unwrap();
                added.is_newer_than(last_run, this_run)
            }
            TermOperator::Changed => {
                let component = state.component.as_ref().debug_checked_unwrap();
                let changed = match &component.ptr {
                    ComponentPtr::SparseSet(set) => {
                        let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                        Some(cells.changed.deref())
                    }
                    ComponentPtr::Table(Some(table)) => {
                        let changed = table
                            .changed
                            .debug_checked_unwrap()
                            .byte_add(std::mem::size_of::<Tick>() * table_row.index());
                        Some(changed.deref::<Tick>())
                    }
                    _ => None,
                }
                .debug_checked_unwrap();
                changed.is_newer_than(last_run, this_run)
            }
            _ => true,
        }
    }

    /// Adds any component accesses used by this [`Term`] to `access`.
    #[inline]
    pub fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        // Components terms access their corresponding component id as a filter, to read or to write
        if let Some(component_id) = self.component {
            debug_assert!(
                self.access.is_none() || !access.access().has_write(component_id),
                "{:?} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                component_id,
            );
            match self.access {
                TermAccess::Read => access.add_read(component_id),
                TermAccess::Write => access.add_write(component_id),
                TermAccess::None => {}
            };
        // Entities access all components
        } else {
            debug_assert!(
                self.access.is_none() || !access.access().has_any_write(),
                "EntityTerm has conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
            );
            match self.access {
                TermAccess::Read => access.read_all(),
                TermAccess::Write => access.write_all(),
                TermAccess::None => {}
            }
        }
    }

    /// For the given `archetype`, adds any component accessed used by this [`Term`] to `access`.
    #[inline]
    pub fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        // Components terms access their corresponding component id as a filter, to read or to write
        if let Some(component_id) = self.component {
            if let Some(archetype_component_id) = archetype.get_archetype_component_id(component_id)
            {
                match self.access {
                    TermAccess::Read => access.add_read(archetype_component_id),
                    TermAccess::Write => access.add_write(archetype_component_id),
                    TermAccess::None => {}
                }
            }
        // Entity terms access all of the components in the archetype
        } else {
            match self.access {
                TermAccess::Read => {
                    for component_id in archetype.components() {
                        let archetype_id =
                            archetype.get_archetype_component_id(component_id).unwrap();
                        access.add_read(archetype_id);
                    }
                }
                TermAccess::Write => {
                    for component_id in archetype.components() {
                        let archetype_id =
                            archetype.get_archetype_component_id(component_id).unwrap();
                        access.add_write(archetype_id);
                    }
                }
                TermAccess::None => {}
            }
        }
    }

    /// Returns `true` if this term matches a set of components. Otherwise, returns `false`.
    #[inline]
    pub fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        if let Some(component_id) = self.component {
            match self.operator {
                TermOperator::Without => !set_contains_id(component_id),
                _ => set_contains_id(component_id),
            }
        } else {
            true
        }
    }
}
