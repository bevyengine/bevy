use std::cell::UnsafeCell;

use bevy_ptr::{Ptr, ThinSlicePtr, UnsafeCellDeref};
use bevy_utils::prelude::default;

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, StorageType, Tick},
    entity::{Entity, EntityLocation},
    query::{Access, DebugCheckedUnwrap, FilteredAccess},
    storage::{ComponentSparseSet, Table, TableRow},
    world::unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell},
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
    /// True if this term accesses it's target at all i.e. Read or Write
    pub fn is_some(&self) -> bool {
        self > &TermAccess::None
    }

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
    /// Whether or not this is an entity term i.e. [`Entity`], [`EntityRef`](crate::prelude::EntityRef) or [`EntityMut`](crate::prelude::EntityMut)
    pub entity: bool,
    /// The [`Component`](crate::prelude::Component) this term targets if any, i.e. `&T`, `&mut T`
    pub component: Option<ComponentId>,
    /// Whether this Term reads/writes the component or entity
    pub access: TermAccess,
    /// The operator to use while resolving this term, see [`TermOperator`]
    pub operator: TermOperator,
    /// Whether or not this term requires change detection information i.e. `&mut T` or [`Changed<T>`](crate::prelude::Changed)
    pub change_detection: bool,
    /// Sub terms if any, used for groups like [`Or`](crate::prelude::Or) or [`AnyOf`](crate::prelude::AnyOf)
    pub sub_terms: Vec<Term>,
}

impl Term {
    /// Create a term representing [`Or`](crate::prelude::Or) with the given sub terms
    pub fn or_terms(sub_terms: Vec<Term>) -> Self {
        Term {
            sub_terms,
            ..default()
        }
    }

    /// Create a term representing [`AnyOf`](crate::prelude::AnyOf) with the given sub terms
    pub fn any_of_terms(sub_terms: Vec<Term>) -> Self {
        Self::or_terms(sub_terms).set_access(TermAccess::Read)
    }

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

    /// Creates a term representing [`Entity`]
    pub fn entity() -> Self {
        Term {
            entity: true,
            ..default()
        }
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
        Self::with_id(id).set_operator(TermOperator::Added)
    }

    /// Creates a term representing [`Changed<T>`](crate::prelude::Changed) where T is the [`Component`](crate::prelude::Component)
    /// associated with id: `id`
    pub fn changed_id(id: ComponentId) -> Self {
        Self::with_id(id).set_operator(TermOperator::Changed)
    }

    /// Adds change detection as requirement for this term
    pub fn with_change_detection(mut self) -> Self {
        self.change_detection = true;
        self
    }

    /// Whether this term can be safely interpreted as `other` i.e. `&T => With<T>` or `&mut T => &T`
    pub fn interpretable_as(&self, other: &Term) -> bool {
        self.entity == other.entity
            && self.operator == other.operator
            && self.access >= other.access
            && (!self.change_detection || other.change_detection)
            && self.sub_terms.iter().enumerate().all(|(i, term)| {
                other
                    .sub_terms
                    .get(i)
                    .is_some_and(|other| term.interpretable_as(other))
            })
    }
}

// Stores each possible pointer type that could be stored in [`TermState`]
pub(crate) enum TermStatePtr<'w> {
    // A reference to the components associated sparse set
    SparseSet(&'w ComponentSparseSet),
    // A reference to the components associated table, set in [`Term::set_table`]
    Table(Option<Ptr<'w>>),
    // A world reference used to construct an [`UnsafeEntityCell`]
    World(UnsafeWorldCell<'w>),
    // A set of sub states for group terms
    Group(Vec<TermState<'w>>),
}

// Stores state for change detection, ptrs gets set in [`Term::set_table`] for table components
// and is otherwise None
pub(crate) struct TermStateTicks<'w> {
    ptrs: Option<(
        ThinSlicePtr<'w, UnsafeCell<Tick>>,
        ThinSlicePtr<'w, UnsafeCell<Tick>>,
    )>,

    last_run: Tick,
    this_run: Tick,
}

// Stores the state for a single term
pub(crate) struct TermState<'w> {
    // Pointer to wherever we need to fetch data to resolve this term
    ptr: TermStatePtr<'w>,
    // Change detection information
    ticks: TermStateTicks<'w>,

    // Size of the associated component
    size: usize,
    // Whether this term matches the associated archetype
    matches: bool,
}

impl TermState<'_> {
    // Returns true of this state can be densely iterated
    #[inline]
    pub fn dense(&self) -> bool {
        !matches!(self.ptr, TermStatePtr::SparseSet(_))
    }
}

/// Fetched change detection data from a resolved [`Term`]
#[derive(Clone)]
pub struct FetchedTicks<'w> {
    /// Added tick for this component
    pub added: &'w UnsafeCell<Tick>,
    /// Changed tick for this component
    pub changed: &'w UnsafeCell<Tick>,

    /// Last run tick for this query
    pub last_run: Tick,
    /// This run tick for this query
    pub this_run: Tick,
}

/// Fetched pointer to data from a resolved [`Term`]
#[derive(Clone)]
pub enum FetchPtr<'w> {
    /// Component fetch, e.g. `&T`
    Component {
        /// A pointer to the component data
        component: Ptr<'w>,
        /// Change detection ticks
        change_ticks: Option<FetchedTicks<'w>>,
    },
    /// Entity fetch, e.g. [`EntityRef`](crate::prelude::EntityRef)
    Entity {
        /// The location of the entity
        location: EntityLocation,
        /// A world reference to construct [`UnsafeEntityCell`]
        world: UnsafeWorldCell<'w>,
    },
    /// Group fetch e.g. [`AnyOf`](crate::prelude::AnyOf)
    Group {
        /// A set of fetched sub terms
        sub_terms: Vec<FetchedTerm<'w>>,
    },
    /// Used if the term accesses no data or doesn't match
    None,
}

/// Represents a [`Term`] that has been fetched from a [`TermQuery`](crate::prelude::TermQuery)
#[derive(Clone)]
pub struct FetchedTerm<'w> {
    /// The [`Entity`] this [`Term`] was resolved with
    pub entity: Entity,
    /// The a pointer to the data fetched with this [`Term`]
    pub ptr: FetchPtr<'w>,
    /// Whether or not this term matched this [`Entity`]
    pub matched: bool,
}

impl<'w> FetchedTerm<'w> {
    /// Helper method to get the ponter to the component data from a [`FetchedTerm`]
    pub fn component_ptr(&self) -> Option<Ptr<'w>> {
        if let FetchPtr::Component { component, .. } = self.ptr {
            Some(component)
        } else {
            None
        }
    }

    /// Helper method to get the fetched change detection data from a [`FetchedTerm`]
    pub fn change_ticks(&self) -> Option<&FetchedTicks<'w>> {
        if let FetchPtr::Component {
            change_ticks: Some(change_ticks),
            ..
        } = &self.ptr
        {
            Some(change_ticks)
        } else {
            None
        }
    }

    /// Helper method to get the fetched entity cell from a [`FetchedTerm`]
    pub fn entity_cell(&self) -> Option<UnsafeEntityCell<'w>> {
        if let FetchPtr::Entity { location, world } = self.ptr {
            Some(UnsafeEntityCell::new(world, self.entity, location))
        } else {
            None
        }
    }

    /// Helper method to get the fetched sub terms from a [`FetchedTerm`]
    pub fn sub_terms(&self) -> Option<&Vec<FetchedTerm<'w>>> {
        if let FetchPtr::Group { sub_terms } = &self.ptr {
            Some(sub_terms)
        } else {
            None
        }
    }
}

impl Term {
    /// Creates and initializes a [`TermState`] for this [`Term`].
    ///
    /// # Safety
    ///
    /// - `world` must have permission to access any of the components specified in `Self::update_archetype_component_access`.
    #[inline]
    pub(crate) unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> TermState<'w> {
        let change_ticks = TermStateTicks {
            ptrs: None,
            last_run,
            this_run,
        };
        // For entity terms we only need a reference to the world and we always match.
        if self.entity {
            TermState {
                ptr: TermStatePtr::World(world),
                ticks: change_ticks,
                size: 0,
                matches: true,
            }
        } else if let Some(component_id) = self.component {
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
                        ptr: TermStatePtr::SparseSet(set),
                        size: info.layout().size(),
                        ticks: change_ticks,
                        matches: true,
                    }
                }
                // For table components we wait until `set_table`
                StorageType::Table => TermState {
                    ptr: TermStatePtr::Table(None),
                    size: info.layout().size(),
                    ticks: change_ticks,
                    matches: false,
                },
            }

        // Group terms initialise state for each sub term and then assemble them into a Vec
        } else {
            let state = self
                .sub_terms
                .iter()
                .map(|term| term.init_state(world, last_run, this_run))
                .collect();
            TermState {
                ptr: TermStatePtr::Group(state),
                ticks: change_ticks,
                size: 0,
                matches: false,
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
        if let TermStatePtr::Group(sub_states) = &mut state.ptr {
            self.sub_terms
                .iter()
                .zip(sub_states.iter_mut())
                .for_each(|(sub_term, sub_state)| {
                    sub_term.set_archetype(sub_state, archetype, table);
                    state.matches |= sub_state.matches;
                });
        }
        if state.matches {
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
        if let TermStatePtr::Group(sub_states) = &mut state.ptr {
            self.sub_terms
                .iter()
                .zip(sub_states.iter_mut())
                .for_each(|(sub_term, sub_state)| {
                    sub_term.set_table(sub_state, table);
                    state.matches |= sub_state.matches;
                });
        }
        if state.matches {
            self.set_table_manual(state, table);
        }
    }

    /// Set the table and change tick pointers for table component [`Term`]s.
    unsafe fn set_table_manual<'w>(&self, state: &mut TermState<'w>, table: &'w Table) {
        if let TermStatePtr::Table(_) = state.ptr {
            if let Some(column) = table.get_column(self.component.debug_checked_unwrap()) {
                state.ptr = TermStatePtr::Table(Some(column.get_data_ptr()));
                state.ticks.ptrs = Some((
                    column.get_added_ticks_slice().into(),
                    column.get_changed_ticks_slice().into(),
                ));
            }
        }
    }

    /// Fetch [`FetchedTerm`] for either the given `entity` in the current [`Table`], or
    /// for the given `entity` in the current [`Archetype`]. This must always be called after
    /// [`Self::set_table`] with a `table_row` in the range of the current [`Table`].
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`Self::set_table`]. `entity` and `table_row` must be
    /// in the range of the current table and archetype.
    ///
    /// If `update_component_access` includes any mutable accesses, then the caller must ensure
    /// that `fetch` is called no more than once for each `entity`/`table_row` in each archetype.
    #[inline(always)]
    pub(crate) unsafe fn fetch<'w>(
        &self,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> FetchedTerm<'w> {
        // If we don't match our current archetype, return an empty fetch
        if !state.matches {
            return FetchedTerm {
                entity,
                ptr: FetchPtr::None,
                matched: false,
            };
        }

        // If we don't access any data return a match but no pointer
        if self.access.is_none() {
            return FetchedTerm {
                entity,
                ptr: FetchPtr::None,
                matched: true,
            };
        }

        match &state.ptr {
            // For entity terms we fetch the current location of the entity to be assembled into a ref
            TermStatePtr::World(world) => FetchedTerm {
                entity,
                ptr: if self.access.is_some() {
                    FetchPtr::Entity {
                        world: *world,
                        location: world.entities().get(entity).debug_checked_unwrap(),
                    }
                } else {
                    FetchPtr::None
                },
                matched: true,
            },
            // For table components we fetch the ptr and change ticks from the table pointer in our state
            TermStatePtr::Table(table) => FetchedTerm {
                entity,
                ptr: FetchPtr::Component {
                    component: table
                        .debug_checked_unwrap()
                        .byte_add(table_row.index() * state.size),
                    change_ticks: if self.change_detection {
                        let (added, changed) = state.ticks.ptrs.debug_checked_unwrap();

                        Some(FetchedTicks {
                            added: added.get(table_row.index()),
                            changed: changed.get(table_row.index()),

                            last_run: state.ticks.last_run,
                            this_run: state.ticks.this_run,
                        })
                    } else {
                        None
                    },
                },
                matched: true,
            },
            // For sparse set components we fetch the ptr and change ticks from the sparse set in our state
            TermStatePtr::SparseSet(sparse_set) => FetchedTerm {
                entity,
                ptr: FetchPtr::Component {
                    component: sparse_set.get(entity).debug_checked_unwrap(),
                    change_ticks: if self.change_detection {
                        let ticks = sparse_set.get_tick_cells(entity).debug_checked_unwrap();
                        Some(FetchedTicks {
                            added: ticks.added,
                            changed: ticks.changed,

                            last_run: state.ticks.last_run,
                            this_run: state.ticks.this_run,
                        })
                    } else {
                        None
                    },
                },
                matched: true,
            },
            // For group terms we recurse into our sub terms
            TermStatePtr::Group(sub_state) => FetchedTerm {
                entity,
                ptr: FetchPtr::Group {
                    sub_terms: {
                        self.sub_terms
                            .iter()
                            .zip(sub_state.iter())
                            .map(|(term, state)| term.fetch(state, entity, table_row))
                            .collect()
                    },
                },
                matched: true,
            },
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
    ) -> bool {
        match &state.ptr {
            // Entity terms always match
            TermStatePtr::World(_) => true,
            // Big code duplication here due to the different ways sparse set and table copmonents access their change ticks
            // Someone smart can probably condense this, but it needs to be performant since it's in the hot loop
            TermStatePtr::SparseSet(set) => {
                // Determine whether the term matches based on the operator
                match self.operator {
                    // These are checked in matches_component_set
                    TermOperator::Optional | TermOperator::With | TermOperator::Without => true,
                    TermOperator::Added => {
                        let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                        cells
                            .added
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                    TermOperator::Changed => {
                        let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                        cells
                            .changed
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                }
            }
            TermStatePtr::Table(_) => {
                // Determine whether the term matches based on the operator
                match self.operator {
                    // These are checked in matches_component_set
                    TermOperator::Optional | TermOperator::With | TermOperator::Without => true,
                    TermOperator::Added => {
                        let (added, _) = state.ticks.ptrs.debug_checked_unwrap();
                        added
                            .get(table_row.index())
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                    TermOperator::Changed => {
                        let (_, changed) = state.ticks.ptrs.debug_checked_unwrap();
                        changed
                            .get(table_row.index())
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                }
            }
            // Recurse to sub terms
            TermStatePtr::Group(states) => self
                .sub_terms
                .iter()
                .zip(states.iter())
                .all(|(term, state)| term.filter_fetch(state, entity, table_row)),
        }
    }

    /// Adds any component accesses used by this [`Term`] to `access`.
    #[inline]
    pub fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        // Entity terms either access just the entity id, read all components or write all components
        if self.entity {
            debug_assert!(
                self.access.is_none() || !access.access().has_any_write(),
                "EntityTerm has conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
            );
            match self.access {
                TermAccess::Read => access.read_all(),
                TermAccess::Write => access.write_all(),
                TermAccess::None => {}
            }
        // Components terms access their corresponding component id as a filter, to read or to write
        } else if let Some(component_id) = self.component {
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
        // For groups recurse into our sub_terms building an or group
        } else {
            let mut iter = self.sub_terms.iter();
            let Some(term) = iter.next() else {
                return
            };
            let mut new_access = access.clone();
            term.update_component_access(&mut new_access);
            iter.for_each(|term| {
                let mut intermediate = access.clone();
                term.update_component_access(&mut intermediate);
                new_access.append_or(&intermediate);
                new_access.extend_access(&intermediate);
            });
            *access = new_access;
        }
    }

    /// For the given `archetype`, adds any component accessed used by this [`Term`] to `access`.
    #[inline]
    pub fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        // Entity terms either access just the entity id, read all components or write all components
        if self.entity {
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
        // Components terms access their corresponding component id as a filter, to read or to write
        } else if let Some(component_id) = self.component {
            if let Some(archetype_component_id) = archetype.get_archetype_component_id(component_id)
            {
                match self.access {
                    TermAccess::Read => access.add_read(archetype_component_id),
                    TermAccess::Write => access.add_write(archetype_component_id),
                    TermAccess::None => {}
                }
            }
        // For groups recurse into our sub_terms
        } else {
            self.sub_terms
                .iter()
                .for_each(|term| term.update_archetype_component_access(archetype, access));
        }
    }

    /// Returns `true` if this term matches a set of components. Otherwise, returns `false`.
    #[inline]
    pub fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        if self.entity {
            true
        } else if let Some(component_id) = self.component {
            match self.operator {
                TermOperator::Without => !set_contains_id(component_id),
                _ => set_contains_id(component_id),
            }
        } else {
            self.sub_terms
                .iter()
                .any(|term| term.matches_component_set(set_contains_id))
        }
    }
}
