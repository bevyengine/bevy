use std::cell::UnsafeCell;
use std::fmt::Debug;

use bevy_ptr::{Ptr, ThinSlicePtr, UnsafeCellDeref};

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, StorageType, Tick},
    entity::Entity,
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
pub enum TermFilter {
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
#[derive(Clone, Debug)]
pub struct Term {
    /// The [`Component`](crate::prelude::Component) this term targets if any, i.e. `&T`, `&mut T`
    pub component: Option<ComponentId>,
    /// Whether this Term reads/writes the component or entity
    pub access: TermAccess,
    /// The operator to use while resolving this term, see [`TermOperator`]
    pub filter: TermFilter,
    /// Whether or not this term requires change detection information i.e. `&mut T` or [`Changed<T>`](crate::prelude::Changed)
    pub change_detection: bool,
    /// Whether or not this term is connected to the following term with an or
    pub or: bool,
    /// Count of the nested brackets this term is surrounded by
    pub depth: u8,
}

impl Term {
    /// Creates an empty term with the given depth
    pub fn new(depth: u8) -> Self {
        Self {
            component: None,
            access: TermAccess::None,
            filter: TermFilter::With,
            change_detection: false,
            or: false,
            depth,
        }
    }

    /// Set the [`TermOperator`] of this [`Term`]
    pub fn with_filter(mut self, op: TermFilter) -> Self {
        self.filter = op;
        self
    }

    /// Set the [`TermAccess`] of this [`Term`]
    pub fn with_access(mut self, access: TermAccess) -> Self {
        self.access = access;
        self
    }

    /// Set the [`TermAccess`] of this [`Term`]
    pub fn with_id(mut self, id: ComponentId) -> Self {
        self.component = Some(id);
        self
    }

    /// Adds change detection as requirement for this term
    pub fn with_change_detection(mut self) -> Self {
        self.change_detection = true;
        self
    }

    /// Returns true if this term is an [`TermOperator::Added`] or [`TermOperator::Changed`] term
    #[inline(always)]
    pub fn filtered(&self) -> bool {
        self.filter == TermFilter::Added || self.filter == TermFilter::Changed
    }

    /// Whether this term can be safely interpreted as `other` i.e. `&T => With<T>` or `&mut T => &T`
    pub fn interpretable_as(&self, other: &Term) -> bool {
        self.component == other.component
            && self.filter == other.filter
            && self.access >= other.access
    }
}

/// Pointer to the location of data for a component, includes the component data itself and optionally
/// change ticks
#[derive(Clone)]
pub struct ComponentPtr<'w> {
    /// Pointer to the data contained in this component
    pub component: Ptr<'w>,
    /// Added ticks
    pub added: Option<ThinSlicePtr<'w, UnsafeCell<Tick>>>,
    /// Changed ticks
    pub changed: Option<ThinSlicePtr<'w, UnsafeCell<Tick>>>,
}

impl Debug for ComponentPtr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentPtr")
            .field("added", &self.added.is_some())
            .field("changed", &self.changed.is_some())
            .finish()
    }
}

/// State stored in [`TermState`] for a component
#[derive(Clone, Debug)]
pub struct ComponentState<'w> {
    /// Pointer to the location of the data for this component
    pub ptr: Option<ComponentPtr<'w>>,
    /// Id of this component
    pub id: ComponentId,
    /// Size of this component
    pub size: usize,
    /// ComponentSparseSet this component belongs too if any
    pub set: Option<&'w ComponentSparseSet>,
}

/// Stores the state for a single [`Term`]
#[derive(Clone, Debug)]
pub struct TermState<'w> {
    /// State related to the component this term targets if any
    pub(crate) component: Option<ComponentState<'w>>,
    /// Whether or not matching this term is optional
    pub optional: bool,
    /// Whether this term matches this archetype
    pub matches: bool,
}

impl<'w> TermState<'w> {
    /// Creates an empty [`TermState`]
    #[inline(always)]
    pub fn empty() -> Self {
        Self {
            component: None,
            optional: false,
            matches: true,
        }
    }

    /// Creates a [`TermState`] with the given [`ComponentState`]
    #[inline(always)]
    pub fn new(component: ComponentState<'w>) -> Self {
        Self {
            component: Some(component),
            optional: false,
            matches: true,
        }
    }

    /// Returns true if we can table iterate this term
    #[inline(always)]
    pub fn dense(&self) -> bool {
        !self.component.as_ref().is_some_and(|c| c.set.is_some())
    }
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
                    TermState::new(ComponentState {
                        ptr: None,
                        id: component_id,
                        size: info.layout().size(),
                        set: Some(set),
                    })
                }
                // For table components we wait until `set_table`
                StorageType::Table => TermState::new(ComponentState {
                    ptr: None,
                    id: component_id,
                    size: info.layout().size(),
                    set: None,
                }),
            }
        } else {
            TermState::empty()
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
        if state.matches && state.component.is_some() {
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
        if state.matches && state.component.is_some() {
            self.set_table_manual(state, table);
        }
    }

    /// Set the table and change tick pointers for table component [`Term`]s.
    #[inline]
    unsafe fn set_table_manual<'w>(&self, state: &mut TermState<'w>, table: &'w Table) {
        let component = state.component.as_mut().debug_checked_unwrap();
        if component.set.is_none() {
            let column = table.get_column(component.id).debug_checked_unwrap();
            component.ptr = Some(ComponentPtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().into()),
                changed: Some(column.get_changed_ticks_slice().into()),
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
        match self.filter {
            TermFilter::Added => {
                let component = state.component.as_ref().debug_checked_unwrap();
                let added = if let Some(set) = component.set {
                    let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                    cells.added.deref()
                } else {
                    let ptr = component.ptr.as_ref().debug_checked_unwrap();
                    ptr.added
                        .debug_checked_unwrap()
                        .get(table_row.index())
                        .deref()
                };
                added.is_newer_than(last_run, this_run)
            }
            TermFilter::Changed => {
                let component = state.component.as_ref().debug_checked_unwrap();
                let changed = if let Some(set) = component.set {
                    let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                    cells.changed.deref()
                } else {
                    let ptr = component.ptr.as_ref().debug_checked_unwrap();
                    ptr.changed
                        .debug_checked_unwrap()
                        .get(table_row.index())
                        .deref()
                };
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
            match self.filter {
                TermFilter::Without => !set_contains_id(component_id),
                _ => set_contains_id(component_id),
            }
        } else {
            true
        }
    }

    /// Update the current [`TermQueryState`] with information from the provided [`Archetype`]
    /// (if applicable, i.e. if the archetype has any intersecting [`ComponentId`] with the current [`TermQueryState`]).
    pub fn matches_archetype(terms: &[Term], archetype: &Archetype) -> bool {
        let matches = |term: &Term| {
            term.filter == TermFilter::Optional
                || term.matches_component_set(&|id| archetype.contains(id))
        };

        let set_bit = |mut mask: u32, index: u8, val: bool| -> u32 {
            if val {
                mask |= 1 << index;
            } else {
                mask &= !(1 << index);
            }
            mask
        };

        let mut result_mask: u32 = u32::MAX;
        let mut or_mask: u32 = 0;
        let mut depth = 0;
        let mut skip_depth = false;

        for term in terms {
            if skip_depth && term.depth >= depth {
                continue;
            };
            if term.depth > depth {
                for d in (depth + 1)..=term.depth {
                    result_mask = set_bit(result_mask, d, true);
                    or_mask = set_bit(or_mask, d, false);
                }
                depth = term.depth;
            }

            if term.depth < depth {
                for d in (term.depth..depth).rev() {
                    if or_mask & (1 << d) > 0 {
                        result_mask |= (result_mask >> 1) & (1 << d);
                    } else {
                        result_mask &= (result_mask >> 1) & (1 << d);
                    }
                }
                depth = term.depth;
            }

            let matches = matches(term);
            // If we are part of an or group
            if or_mask & (1 << term.depth) > 0 {
                // If we already have a true
                if result_mask & (1 << term.depth) > 0 {
                    or_mask = set_bit(or_mask, term.depth, term.or);
                    continue;
                }
            } else {
                // If we already have a false
                if result_mask & (1 << term.depth) == 0 {
                    or_mask = set_bit(or_mask, term.depth, term.or);
                    skip_depth = true;
                    continue;
                }
            }

            result_mask = set_bit(result_mask, term.depth, matches);
            or_mask = set_bit(or_mask, term.depth, term.or);
        }

        if depth > 0 {
            for d in (0..depth).rev() {
                if or_mask & (1 << d) > 0 {
                    result_mask |= (result_mask >> 1) & (1 << d);
                } else {
                    result_mask &= (result_mask >> 1) & (1 << d);
                }
            }
        }

        result_mask & 1 > 0
    }
}
