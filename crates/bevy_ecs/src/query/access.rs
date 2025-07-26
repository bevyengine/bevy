use crate::component::ComponentId;
use crate::world::World;
use alloc::{format, string::String, vec, vec::Vec};
use core::{fmt, fmt::Debug};
use derive_more::From;
use fixedbitset::FixedBitSet;
use thiserror::Error;

/// A wrapper struct to make Debug representations of [`FixedBitSet`] easier
/// to read.
///
/// Instead of the raw integer representation of the `FixedBitSet`, the list of
/// indexes are shown.
///
/// Normal `FixedBitSet` `Debug` output:
/// ```text
/// read_and_writes: FixedBitSet { data: [ 160 ], length: 8 }
/// ```
///
/// Which, unless you are a computer, doesn't help much understand what's in
/// the set. With `FormattedBitSet`, we convert the present set entries into
/// what they stand for, it is much clearer what is going on:
/// ```text
/// read_and_writes: [ 5, 7 ]
/// ```
struct FormattedBitSet<'a> {
    bit_set: &'a FixedBitSet,
}

impl<'a> FormattedBitSet<'a> {
    fn new(bit_set: &'a FixedBitSet) -> Self {
        Self { bit_set }
    }
}

impl<'a> Debug for FormattedBitSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.bit_set.ones()).finish()
    }
}

/// Tracks read and write access to specific elements in a collection.
///
/// Used internally to ensure soundness during system initialization and execution.
/// See the [`is_compatible`](Access::is_compatible) and [`get_conflicts`](Access::get_conflicts) functions.
#[derive(Eq, PartialEq, Default)]
pub struct Access {
    /// All accessed components, or forbidden components if
    /// `Self::component_read_and_writes_inverted` is set.
    component_read_and_writes: FixedBitSet,
    /// All exclusively-accessed components, or components that may not be
    /// exclusively accessed if `Self::component_writes_inverted` is set.
    component_writes: FixedBitSet,
    /// All accessed resources.
    resource_read_and_writes: FixedBitSet,
    /// The exclusively-accessed resources.
    resource_writes: FixedBitSet,
    /// Is `true` if this component can read all components *except* those
    /// present in `Self::component_read_and_writes`.
    component_read_and_writes_inverted: bool,
    /// Is `true` if this component can write to all components *except* those
    /// present in `Self::component_writes`.
    component_writes_inverted: bool,
    /// Is `true` if this has access to all resources.
    /// This field is a performance optimization for `&World` (also harder to mess up for soundness).
    reads_all_resources: bool,
    /// Is `true` if this has mutable access to all resources.
    /// If this is true, then `reads_all` must also be true.
    writes_all_resources: bool,
    // Components that are not accessed, but whose presence in an archetype affect query results.
    archetypal: FixedBitSet,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for Access {
    fn clone(&self) -> Self {
        Self {
            component_read_and_writes: self.component_read_and_writes.clone(),
            component_writes: self.component_writes.clone(),
            resource_read_and_writes: self.resource_read_and_writes.clone(),
            resource_writes: self.resource_writes.clone(),
            component_read_and_writes_inverted: self.component_read_and_writes_inverted,
            component_writes_inverted: self.component_writes_inverted,
            reads_all_resources: self.reads_all_resources,
            writes_all_resources: self.writes_all_resources,
            archetypal: self.archetypal.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.component_read_and_writes
            .clone_from(&source.component_read_and_writes);
        self.component_writes.clone_from(&source.component_writes);
        self.resource_read_and_writes
            .clone_from(&source.resource_read_and_writes);
        self.resource_writes.clone_from(&source.resource_writes);
        self.component_read_and_writes_inverted = source.component_read_and_writes_inverted;
        self.component_writes_inverted = source.component_writes_inverted;
        self.reads_all_resources = source.reads_all_resources;
        self.writes_all_resources = source.writes_all_resources;
        self.archetypal.clone_from(&source.archetypal);
    }
}

impl Debug for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Access")
            .field(
                "component_read_and_writes",
                &FormattedBitSet::new(&self.component_read_and_writes),
            )
            .field(
                "component_writes",
                &FormattedBitSet::new(&self.component_writes),
            )
            .field(
                "resource_read_and_writes",
                &FormattedBitSet::new(&self.resource_read_and_writes),
            )
            .field(
                "resource_writes",
                &FormattedBitSet::new(&self.resource_writes),
            )
            .field(
                "component_read_and_writes_inverted",
                &self.component_read_and_writes_inverted,
            )
            .field("component_writes_inverted", &self.component_writes_inverted)
            .field("reads_all_resources", &self.reads_all_resources)
            .field("writes_all_resources", &self.writes_all_resources)
            .field("archetypal", &FormattedBitSet::new(&self.archetypal))
            .finish()
    }
}

impl Access {
    /// Creates an empty [`Access`] collection.
    pub const fn new() -> Self {
        Self {
            reads_all_resources: false,
            writes_all_resources: false,
            component_read_and_writes_inverted: false,
            component_writes_inverted: false,
            component_read_and_writes: FixedBitSet::new(),
            component_writes: FixedBitSet::new(),
            resource_read_and_writes: FixedBitSet::new(),
            resource_writes: FixedBitSet::new(),
            archetypal: FixedBitSet::new(),
        }
    }

    /// Creates an [`Access`] with read access to all components.
    /// This is equivalent to calling `read_all()` on `Access::new()`,
    /// but is available in a `const` context.
    pub(crate) const fn new_read_all() -> Self {
        let mut access = Self::new();
        access.reads_all_resources = true;
        // Note that we cannot use `read_all_components()`
        // because `FixedBitSet::clear()` is not `const`.
        access.component_read_and_writes_inverted = true;
        access
    }

    /// Creates an [`Access`] with read access to all components.
    /// This is equivalent to calling `read_all()` on `Access::new()`,
    /// but is available in a `const` context.
    pub(crate) const fn new_write_all() -> Self {
        let mut access = Self::new();
        access.reads_all_resources = true;
        access.writes_all_resources = true;
        // Note that we cannot use `write_all_components()`
        // because `FixedBitSet::clear()` is not `const`.
        access.component_read_and_writes_inverted = true;
        access.component_writes_inverted = true;
        access
    }

    fn add_component_sparse_set_index_read(&mut self, index: usize) {
        if !self.component_read_and_writes_inverted {
            self.component_read_and_writes.grow_and_insert(index);
        } else if index < self.component_read_and_writes.len() {
            self.component_read_and_writes.remove(index);
        }
    }

    fn add_component_sparse_set_index_write(&mut self, index: usize) {
        if !self.component_writes_inverted {
            self.component_writes.grow_and_insert(index);
        } else if index < self.component_writes.len() {
            self.component_writes.remove(index);
        }
    }

    /// Adds access to the component given by `index`.
    pub fn add_component_read(&mut self, index: ComponentId) {
        let sparse_set_index = index.index();
        self.add_component_sparse_set_index_read(sparse_set_index);
    }

    /// Adds exclusive access to the component given by `index`.
    pub fn add_component_write(&mut self, index: ComponentId) {
        let sparse_set_index = index.index();
        self.add_component_sparse_set_index_read(sparse_set_index);
        self.add_component_sparse_set_index_write(sparse_set_index);
    }

    /// Adds access to the resource given by `index`.
    pub fn add_resource_read(&mut self, index: ComponentId) {
        self.resource_read_and_writes.grow_and_insert(index.index());
    }

    /// Adds exclusive access to the resource given by `index`.
    pub fn add_resource_write(&mut self, index: ComponentId) {
        self.resource_read_and_writes.grow_and_insert(index.index());
        self.resource_writes.grow_and_insert(index.index());
    }

    fn remove_component_sparse_set_index_read(&mut self, index: usize) {
        if self.component_read_and_writes_inverted {
            self.component_read_and_writes.grow_and_insert(index);
        } else if index < self.component_read_and_writes.len() {
            self.component_read_and_writes.remove(index);
        }
    }

    fn remove_component_sparse_set_index_write(&mut self, index: usize) {
        if self.component_writes_inverted {
            self.component_writes.grow_and_insert(index);
        } else if index < self.component_writes.len() {
            self.component_writes.remove(index);
        }
    }

    /// Removes both read and write access to the component given by `index`.
    ///
    /// Because this method corresponds to the set difference operator ∖, it can
    /// create complicated logical formulas that you should verify correctness
    /// of. For example, A ∪ (B ∖ A) isn't equivalent to (A ∪ B) ∖ A, so you
    /// can't replace a call to `remove_component_read` followed by a call to
    /// `extend` with a call to `extend` followed by a call to
    /// `remove_component_read`.
    pub fn remove_component_read(&mut self, index: ComponentId) {
        let sparse_set_index = index.index();
        self.remove_component_sparse_set_index_write(sparse_set_index);
        self.remove_component_sparse_set_index_read(sparse_set_index);
    }

    /// Removes write access to the component given by `index`.
    ///
    /// Because this method corresponds to the set difference operator ∖, it can
    /// create complicated logical formulas that you should verify correctness
    /// of. For example, A ∪ (B ∖ A) isn't equivalent to (A ∪ B) ∖ A, so you
    /// can't replace a call to `remove_component_write` followed by a call to
    /// `extend` with a call to `extend` followed by a call to
    /// `remove_component_write`.
    pub fn remove_component_write(&mut self, index: ComponentId) {
        let sparse_set_index = index.index();
        self.remove_component_sparse_set_index_write(sparse_set_index);
    }

    /// Adds an archetypal (indirect) access to the component given by `index`.
    ///
    /// This is for components whose values are not accessed (and thus will never cause conflicts),
    /// but whose presence in an archetype may affect query results.
    ///
    /// Currently, this is only used for [`Has<T>`] and [`Allows<T>`].
    ///
    /// [`Has<T>`]: crate::query::Has
    /// [`Allows<T>`]: crate::query::filter::Allows
    pub fn add_archetypal(&mut self, index: ComponentId) {
        self.archetypal.grow_and_insert(index.index());
    }

    /// Returns `true` if this can access the component given by `index`.
    pub fn has_component_read(&self, index: ComponentId) -> bool {
        self.component_read_and_writes_inverted
            ^ self.component_read_and_writes.contains(index.index())
    }

    /// Returns `true` if this can access any component.
    pub fn has_any_component_read(&self) -> bool {
        self.component_read_and_writes_inverted || !self.component_read_and_writes.is_clear()
    }

    /// Returns `true` if this can exclusively access the component given by `index`.
    pub fn has_component_write(&self, index: ComponentId) -> bool {
        self.component_writes_inverted ^ self.component_writes.contains(index.index())
    }

    /// Returns `true` if this accesses any component mutably.
    pub fn has_any_component_write(&self) -> bool {
        self.component_writes_inverted || !self.component_writes.is_clear()
    }

    /// Returns `true` if this can access the resource given by `index`.
    pub fn has_resource_read(&self, index: ComponentId) -> bool {
        self.reads_all_resources || self.resource_read_and_writes.contains(index.index())
    }

    /// Returns `true` if this can access any resource.
    pub fn has_any_resource_read(&self) -> bool {
        self.reads_all_resources || !self.resource_read_and_writes.is_clear()
    }

    /// Returns `true` if this can exclusively access the resource given by `index`.
    pub fn has_resource_write(&self, index: ComponentId) -> bool {
        self.writes_all_resources || self.resource_writes.contains(index.index())
    }

    /// Returns `true` if this accesses any resource mutably.
    pub fn has_any_resource_write(&self) -> bool {
        self.writes_all_resources || !self.resource_writes.is_clear()
    }

    /// Returns `true` if this accesses any resources or components.
    pub fn has_any_read(&self) -> bool {
        self.has_any_component_read() || self.has_any_resource_read()
    }

    /// Returns `true` if this accesses any resources or components mutably.
    pub fn has_any_write(&self) -> bool {
        self.has_any_component_write() || self.has_any_resource_write()
    }

    /// Returns true if this has an archetypal (indirect) access to the component given by `index`.
    ///
    /// This is a component whose value is not accessed (and thus will never cause conflicts),
    /// but whose presence in an archetype may affect query results.
    ///
    /// Currently, this is only used for [`Has<T>`].
    ///
    /// [`Has<T>`]: crate::query::Has
    pub fn has_archetypal(&self, index: ComponentId) -> bool {
        self.archetypal.contains(index.index())
    }

    /// Sets this as having access to all components (i.e. `EntityRef`).
    #[inline]
    pub fn read_all_components(&mut self) {
        self.component_read_and_writes_inverted = true;
        self.component_read_and_writes.clear();
    }

    /// Sets this as having mutable access to all components (i.e. `EntityMut`).
    #[inline]
    pub fn write_all_components(&mut self) {
        self.read_all_components();
        self.component_writes_inverted = true;
        self.component_writes.clear();
    }

    /// Sets this as having access to all resources (i.e. `&World`).
    #[inline]
    pub const fn read_all_resources(&mut self) {
        self.reads_all_resources = true;
    }

    /// Sets this as having mutable access to all resources (i.e. `&mut World`).
    #[inline]
    pub const fn write_all_resources(&mut self) {
        self.reads_all_resources = true;
        self.writes_all_resources = true;
    }

    /// Sets this as having access to all indexed elements (i.e. `&World`).
    #[inline]
    pub fn read_all(&mut self) {
        self.read_all_components();
        self.read_all_resources();
    }

    /// Sets this as having mutable access to all indexed elements (i.e. `&mut World`).
    #[inline]
    pub fn write_all(&mut self) {
        self.write_all_components();
        self.write_all_resources();
    }

    /// Returns `true` if this has access to all components (i.e. `EntityRef`).
    #[inline]
    pub fn has_read_all_components(&self) -> bool {
        self.component_read_and_writes_inverted && self.component_read_and_writes.is_clear()
    }

    /// Returns `true` if this has write access to all components (i.e. `EntityMut`).
    #[inline]
    pub fn has_write_all_components(&self) -> bool {
        self.component_writes_inverted && self.component_writes.is_clear()
    }

    /// Returns `true` if this has access to all resources (i.e. `EntityRef`).
    #[inline]
    pub fn has_read_all_resources(&self) -> bool {
        self.reads_all_resources
    }

    /// Returns `true` if this has write access to all resources (i.e. `EntityMut`).
    #[inline]
    pub fn has_write_all_resources(&self) -> bool {
        self.writes_all_resources
    }

    /// Returns `true` if this has access to all indexed elements (i.e. `&World`).
    pub fn has_read_all(&self) -> bool {
        self.has_read_all_components() && self.has_read_all_resources()
    }

    /// Returns `true` if this has write access to all indexed elements (i.e. `&mut World`).
    pub fn has_write_all(&self) -> bool {
        self.has_write_all_components() && self.has_write_all_resources()
    }

    /// Removes all writes.
    pub fn clear_writes(&mut self) {
        self.writes_all_resources = false;
        self.component_writes_inverted = false;
        self.component_writes.clear();
        self.resource_writes.clear();
    }

    /// Removes all accesses.
    pub fn clear(&mut self) {
        self.reads_all_resources = false;
        self.writes_all_resources = false;
        self.component_read_and_writes_inverted = false;
        self.component_writes_inverted = false;
        self.component_read_and_writes.clear();
        self.component_writes.clear();
        self.resource_read_and_writes.clear();
        self.resource_writes.clear();
    }

    /// Adds all access from `other`.
    pub fn extend(&mut self, other: &Access) {
        invertible_union_with(
            &mut self.component_read_and_writes,
            &mut self.component_read_and_writes_inverted,
            &other.component_read_and_writes,
            other.component_read_and_writes_inverted,
        );
        invertible_union_with(
            &mut self.component_writes,
            &mut self.component_writes_inverted,
            &other.component_writes,
            other.component_writes_inverted,
        );

        self.reads_all_resources = self.reads_all_resources || other.reads_all_resources;
        self.writes_all_resources = self.writes_all_resources || other.writes_all_resources;
        self.resource_read_and_writes
            .union_with(&other.resource_read_and_writes);
        self.resource_writes.union_with(&other.resource_writes);
        self.archetypal.union_with(&other.archetypal);
    }

    /// Removes any access from `self` that would conflict with `other`.
    /// This removes any reads and writes for any component written by `other`,
    /// and removes any writes for any component read by `other`.
    pub fn remove_conflicting_access(&mut self, other: &Access) {
        invertible_difference_with(
            &mut self.component_read_and_writes,
            &mut self.component_read_and_writes_inverted,
            &other.component_writes,
            other.component_writes_inverted,
        );
        invertible_difference_with(
            &mut self.component_writes,
            &mut self.component_writes_inverted,
            &other.component_read_and_writes,
            other.component_read_and_writes_inverted,
        );

        if other.reads_all_resources {
            self.writes_all_resources = false;
            self.resource_writes.clear();
        }
        if other.writes_all_resources {
            self.reads_all_resources = false;
            self.resource_read_and_writes.clear();
        }
        self.resource_read_and_writes
            .difference_with(&other.resource_writes);
        self.resource_writes
            .difference_with(&other.resource_read_and_writes);
    }

    /// Returns `true` if the access and `other` can be active at the same time,
    /// only looking at their component access.
    ///
    /// [`Access`] instances are incompatible if one can write
    /// an element that the other can read or write.
    pub fn is_components_compatible(&self, other: &Access) -> bool {
        // We have a conflict if we write and they read or write, or if they
        // write and we read or write.
        for (
            lhs_writes,
            rhs_reads_and_writes,
            lhs_writes_inverted,
            rhs_reads_and_writes_inverted,
        ) in [
            (
                &self.component_writes,
                &other.component_read_and_writes,
                self.component_writes_inverted,
                other.component_read_and_writes_inverted,
            ),
            (
                &other.component_writes,
                &self.component_read_and_writes,
                other.component_writes_inverted,
                self.component_read_and_writes_inverted,
            ),
        ] {
            match (lhs_writes_inverted, rhs_reads_and_writes_inverted) {
                (true, true) => return false,
                (false, true) => {
                    if !lhs_writes.is_subset(rhs_reads_and_writes) {
                        return false;
                    }
                }
                (true, false) => {
                    if !rhs_reads_and_writes.is_subset(lhs_writes) {
                        return false;
                    }
                }
                (false, false) => {
                    if !lhs_writes.is_disjoint(rhs_reads_and_writes) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Returns `true` if the access and `other` can be active at the same time,
    /// only looking at their resource access.
    ///
    /// [`Access`] instances are incompatible if one can write
    /// an element that the other can read or write.
    pub fn is_resources_compatible(&self, other: &Access) -> bool {
        if self.writes_all_resources {
            return !other.has_any_resource_read();
        }

        if other.writes_all_resources {
            return !self.has_any_resource_read();
        }

        if self.reads_all_resources {
            return !other.has_any_resource_write();
        }

        if other.reads_all_resources {
            return !self.has_any_resource_write();
        }

        self.resource_writes
            .is_disjoint(&other.resource_read_and_writes)
            && other
                .resource_writes
                .is_disjoint(&self.resource_read_and_writes)
    }

    /// Returns `true` if the access and `other` can be active at the same time.
    ///
    /// [`Access`] instances are incompatible if one can write
    /// an element that the other can read or write.
    pub fn is_compatible(&self, other: &Access) -> bool {
        self.is_components_compatible(other) && self.is_resources_compatible(other)
    }

    /// Returns `true` if the set's component access is a subset of another, i.e. `other`'s component access
    /// contains at least all the values in `self`.
    pub fn is_subset_components(&self, other: &Access) -> bool {
        for (
            our_components,
            their_components,
            our_components_inverted,
            their_components_inverted,
        ) in [
            (
                &self.component_read_and_writes,
                &other.component_read_and_writes,
                self.component_read_and_writes_inverted,
                other.component_read_and_writes_inverted,
            ),
            (
                &self.component_writes,
                &other.component_writes,
                self.component_writes_inverted,
                other.component_writes_inverted,
            ),
        ] {
            match (our_components_inverted, their_components_inverted) {
                (true, true) => {
                    if !their_components.is_subset(our_components) {
                        return false;
                    }
                }
                (true, false) => {
                    return false;
                }
                (false, true) => {
                    if !our_components.is_disjoint(their_components) {
                        return false;
                    }
                }
                (false, false) => {
                    if !our_components.is_subset(their_components) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Returns `true` if the set's resource access is a subset of another, i.e. `other`'s resource access
    /// contains at least all the values in `self`.
    pub fn is_subset_resources(&self, other: &Access) -> bool {
        if self.writes_all_resources {
            return other.writes_all_resources;
        }

        if other.writes_all_resources {
            return true;
        }

        if self.reads_all_resources {
            return other.reads_all_resources;
        }

        if other.reads_all_resources {
            return self.resource_writes.is_subset(&other.resource_writes);
        }

        self.resource_read_and_writes
            .is_subset(&other.resource_read_and_writes)
            && self.resource_writes.is_subset(&other.resource_writes)
    }

    /// Returns `true` if the set is a subset of another, i.e. `other` contains
    /// at least all the values in `self`.
    pub fn is_subset(&self, other: &Access) -> bool {
        self.is_subset_components(other) && self.is_subset_resources(other)
    }

    fn get_component_conflicts(&self, other: &Access) -> AccessConflicts {
        let mut conflicts = FixedBitSet::new();

        // We have a conflict if we write and they read or write, or if they
        // write and we read or write.
        for (
            lhs_writes,
            rhs_reads_and_writes,
            lhs_writes_inverted,
            rhs_reads_and_writes_inverted,
        ) in [
            (
                &self.component_writes,
                &other.component_read_and_writes,
                self.component_writes_inverted,
                other.component_read_and_writes_inverted,
            ),
            (
                &other.component_writes,
                &self.component_read_and_writes,
                other.component_writes_inverted,
                self.component_read_and_writes_inverted,
            ),
        ] {
            // There's no way that I can see to do this without a temporary.
            // Neither CNF nor DNF allows us to avoid one.
            let temp_conflicts: FixedBitSet =
                match (lhs_writes_inverted, rhs_reads_and_writes_inverted) {
                    (true, true) => return AccessConflicts::All,
                    (false, true) => lhs_writes.difference(rhs_reads_and_writes).collect(),
                    (true, false) => rhs_reads_and_writes.difference(lhs_writes).collect(),
                    (false, false) => lhs_writes.intersection(rhs_reads_and_writes).collect(),
                };
            conflicts.union_with(&temp_conflicts);
        }

        AccessConflicts::Individual(conflicts)
    }

    /// Returns a vector of elements that the access and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &Access) -> AccessConflicts {
        let mut conflicts = match self.get_component_conflicts(other) {
            AccessConflicts::All => return AccessConflicts::All,
            AccessConflicts::Individual(conflicts) => conflicts,
        };

        if self.reads_all_resources {
            if other.writes_all_resources {
                return AccessConflicts::All;
            }
            conflicts.extend(other.resource_writes.ones());
        }

        if other.reads_all_resources {
            if self.writes_all_resources {
                return AccessConflicts::All;
            }
            conflicts.extend(self.resource_writes.ones());
        }
        if self.writes_all_resources {
            conflicts.extend(other.resource_read_and_writes.ones());
        }

        if other.writes_all_resources {
            conflicts.extend(self.resource_read_and_writes.ones());
        }

        conflicts.extend(
            self.resource_writes
                .intersection(&other.resource_read_and_writes),
        );
        conflicts.extend(
            self.resource_read_and_writes
                .intersection(&other.resource_writes),
        );
        AccessConflicts::Individual(conflicts)
    }

    /// Returns the indices of the resources this has access to.
    pub fn resource_reads_and_writes(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.resource_read_and_writes.ones().map(ComponentId::new)
    }

    /// Returns the indices of the resources this has non-exclusive access to.
    pub fn resource_reads(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.resource_read_and_writes
            .difference(&self.resource_writes)
            .map(ComponentId::new)
    }

    /// Returns the indices of the resources this has exclusive access to.
    pub fn resource_writes(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.resource_writes.ones().map(ComponentId::new)
    }

    /// Returns the indices of the components that this has an archetypal access to.
    ///
    /// These are components whose values are not accessed (and thus will never cause conflicts),
    /// but whose presence in an archetype may affect query results.
    ///
    /// Currently, this is only used for [`Has<T>`].
    ///
    /// [`Has<T>`]: crate::query::Has
    pub fn archetypal(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.archetypal.ones().map(ComponentId::new)
    }

    /// Returns an iterator over the component IDs and their [`ComponentAccessKind`].
    ///
    /// Returns `Err(UnboundedAccess)` if the access is unbounded.
    /// This typically occurs when an [`Access`] is marked as accessing all
    /// components, and then adding exceptions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::query::{Access, ComponentAccessKind};
    /// # use bevy_ecs::component::ComponentId;
    /// let mut access = Access::default();
    ///
    /// access.add_component_read(ComponentId::new(1));
    /// access.add_component_write(ComponentId::new(2));
    /// access.add_archetypal(ComponentId::new(3));
    ///
    /// let result = access
    ///     .try_iter_component_access()
    ///     .map(Iterator::collect::<Vec<_>>);
    ///
    /// assert_eq!(
    ///     result,
    ///     Ok(vec![
    ///         ComponentAccessKind::Shared(ComponentId::new(1)),
    ///         ComponentAccessKind::Exclusive(ComponentId::new(2)),
    ///         ComponentAccessKind::Archetypal(ComponentId::new(3)),
    ///     ]),
    /// );
    /// ```
    pub fn try_iter_component_access(
        &self,
    ) -> Result<impl Iterator<Item = ComponentAccessKind> + '_, UnboundedAccessError> {
        // component_writes_inverted is only ever true when component_read_and_writes_inverted is
        // also true. Therefore it is sufficient to check just component_read_and_writes_inverted.
        if self.component_read_and_writes_inverted {
            return Err(UnboundedAccessError {
                writes_inverted: self.component_writes_inverted,
                read_and_writes_inverted: self.component_read_and_writes_inverted,
            });
        }

        let reads_and_writes = self.component_read_and_writes.ones().map(|index| {
            let sparse_index = ComponentId::new(index);

            if self.component_writes.contains(index) {
                ComponentAccessKind::Exclusive(sparse_index)
            } else {
                ComponentAccessKind::Shared(sparse_index)
            }
        });

        let archetypal = self
            .archetypal
            .ones()
            .filter(|&index| {
                !self.component_writes.contains(index)
                    && !self.component_read_and_writes.contains(index)
            })
            .map(|index| ComponentAccessKind::Archetypal(ComponentId::new(index)));

        Ok(reads_and_writes.chain(archetypal))
    }
}

/// Performs an in-place union of `other` into `self`, where either set may be inverted.
///
/// Each set corresponds to a `FixedBitSet` if `inverted` is `false`,
/// or to the infinite (co-finite) complement of the `FixedBitSet` if `inverted` is `true`.
///
/// This updates the `self` set to include any elements in the `other` set.
/// Note that this may change `self_inverted` to `true` if we add an infinite
/// set to a finite one, resulting in a new infinite set.
fn invertible_union_with(
    self_set: &mut FixedBitSet,
    self_inverted: &mut bool,
    other_set: &FixedBitSet,
    other_inverted: bool,
) {
    match (*self_inverted, other_inverted) {
        (true, true) => self_set.intersect_with(other_set),
        (true, false) => self_set.difference_with(other_set),
        (false, true) => {
            *self_inverted = true;
            // We have to grow here because the new bits are going to get flipped to 1.
            self_set.grow(other_set.len());
            self_set.toggle_range(..);
            self_set.intersect_with(other_set);
        }
        (false, false) => self_set.union_with(other_set),
    }
}

/// Performs an in-place set difference of `other` from `self`, where either set may be inverted.
///
/// Each set corresponds to a `FixedBitSet` if `inverted` is `false`,
/// or to the infinite (co-finite) complement of the `FixedBitSet` if `inverted` is `true`.
///
/// This updates the `self` set to remove any elements in the `other` set.
/// Note that this may change `self_inverted` to `false` if we remove an
/// infinite set from another infinite one, resulting in a finite difference.
fn invertible_difference_with(
    self_set: &mut FixedBitSet,
    self_inverted: &mut bool,
    other_set: &FixedBitSet,
    other_inverted: bool,
) {
    // We can share the implementation of `invertible_union_with` with some algebra:
    // A - B = A & !B = !(!A | B)
    *self_inverted = !*self_inverted;
    invertible_union_with(self_set, self_inverted, other_set, other_inverted);
    *self_inverted = !*self_inverted;
}

/// Error returned when attempting to iterate over items included in an [`Access`]
/// if the access excludes items rather than including them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
#[error("Access is unbounded")]
pub struct UnboundedAccessError {
    /// [`Access`] is defined in terms of _excluding_ [exclusive](ComponentAccessKind::Exclusive)
    /// access.
    pub writes_inverted: bool,
    /// [`Access`] is defined in terms of _excluding_ [shared](ComponentAccessKind::Shared) and
    /// [exclusive](ComponentAccessKind::Exclusive) access.
    pub read_and_writes_inverted: bool,
}

/// Describes the level of access for a particular component as defined in an [`Access`].
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum ComponentAccessKind {
    /// Archetypical access, such as `Has<Foo>`.
    Archetypal(ComponentId),
    /// Shared access, such as `&Foo`.
    Shared(ComponentId),
    /// Exclusive access, such as `&mut Foo`.
    Exclusive(ComponentId),
}

impl ComponentAccessKind {
    /// Gets the index of this `ComponentAccessKind`.
    pub fn index(&self) -> &ComponentId {
        let (Self::Archetypal(value) | Self::Shared(value) | Self::Exclusive(value)) = self;
        value
    }
}

/// An [`Access`] that has been filtered to include and exclude certain combinations of elements.
///
/// Used internally to statically check if queries are disjoint.
///
/// Subtle: a `read` or `write` in `access` should not be considered to imply a
/// `with` access.
///
/// For example consider `Query<Option<&T>>` this only has a `read` of `T` as doing
/// otherwise would allow for queries to be considered disjoint when they shouldn't:
/// - `Query<(&mut T, Option<&U>)>` read/write `T`, read `U`, with `U`
/// - `Query<&mut T, Without<U>>` read/write `T`, without `U`
///   from this we could reasonably conclude that the queries are disjoint but they aren't.
///
/// In order to solve this the actual access that `Query<(&mut T, Option<&U>)>` has
/// is read/write `T`, read `U`. It must still have a read `U` access otherwise the following
/// queries would be incorrectly considered disjoint:
/// - `Query<&mut T>`  read/write `T`
/// - `Query<Option<&T>>` accesses nothing
///
/// See comments the [`WorldQuery`](super::WorldQuery) impls of [`AnyOf`](super::AnyOf)/`Option`/[`Or`](super::Or) for more information.
#[derive(Debug, Eq, PartialEq)]
pub struct FilteredAccess {
    pub(crate) access: Access,
    pub(crate) required: FixedBitSet,
    // An array of filter sets to express `With` or `Without` clauses in disjunctive normal form, for example: `Or<(With<A>, With<B>)>`.
    // Filters like `(With<A>, Or<(With<B>, Without<C>)>` are expanded into `Or<((With<A>, With<B>), (With<A>, Without<C>))>`.
    pub(crate) filter_sets: Vec<AccessFilters>,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for FilteredAccess {
    fn clone(&self) -> Self {
        Self {
            access: self.access.clone(),
            required: self.required.clone(),
            filter_sets: self.filter_sets.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.access.clone_from(&source.access);
        self.required.clone_from(&source.required);
        self.filter_sets.clone_from(&source.filter_sets);
    }
}

impl Default for FilteredAccess {
    fn default() -> Self {
        Self::matches_everything()
    }
}

impl From<FilteredAccess> for FilteredAccessSet {
    fn from(filtered_access: FilteredAccess) -> Self {
        let mut base = FilteredAccessSet::default();
        base.add(filtered_access);
        base
    }
}

/// Records how two accesses conflict with each other
#[derive(Debug, PartialEq, From)]
pub enum AccessConflicts {
    /// Conflict is for all indices
    All,
    /// There is a conflict for a subset of indices
    Individual(FixedBitSet),
}

impl AccessConflicts {
    fn add(&mut self, other: &Self) {
        match (self, other) {
            (s, AccessConflicts::All) => {
                *s = AccessConflicts::All;
            }
            (AccessConflicts::Individual(this), AccessConflicts::Individual(other)) => {
                this.extend(other.ones());
            }
            _ => {}
        }
    }

    /// Returns true if there are no conflicts present
    pub fn is_empty(&self) -> bool {
        match self {
            Self::All => false,
            Self::Individual(set) => set.is_empty(),
        }
    }

    pub(crate) fn format_conflict_list(&self, world: &World) -> String {
        match self {
            AccessConflicts::All => String::new(),
            AccessConflicts::Individual(indices) => indices
                .ones()
                .map(|index| {
                    format!(
                        "{}",
                        world
                            .components
                            .get_name(ComponentId::new(index))
                            .unwrap()
                            .shortname()
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    /// An [`AccessConflicts`] which represents the absence of any conflict
    pub(crate) fn empty() -> Self {
        Self::Individual(FixedBitSet::new())
    }
}

impl From<Vec<ComponentId>> for AccessConflicts {
    fn from(value: Vec<ComponentId>) -> Self {
        Self::Individual(value.iter().map(|c| c.index()).collect())
    }
}

impl FilteredAccess {
    /// Returns a `FilteredAccess` which has no access and matches everything.
    /// This is the equivalent of a `TRUE` logic atom.
    pub fn matches_everything() -> Self {
        Self {
            access: Access::default(),
            required: FixedBitSet::default(),
            filter_sets: vec![AccessFilters::default()],
        }
    }

    /// Returns a `FilteredAccess` which has no access and matches nothing.
    /// This is the equivalent of a `FALSE` logic atom.
    pub fn matches_nothing() -> Self {
        Self {
            access: Access::default(),
            required: FixedBitSet::default(),
            filter_sets: Vec::new(),
        }
    }

    /// Returns a reference to the underlying unfiltered access.
    #[inline]
    pub fn access(&self) -> &Access {
        &self.access
    }

    /// Returns a mutable reference to the underlying unfiltered access.
    #[inline]
    pub fn access_mut(&mut self) -> &mut Access {
        &mut self.access
    }

    /// Adds access to the component given by `index`.
    pub fn add_component_read(&mut self, index: ComponentId) {
        self.access.add_component_read(index);
        self.add_required(index);
        self.and_with(index);
    }

    /// Adds exclusive access to the component given by `index`.
    pub fn add_component_write(&mut self, index: ComponentId) {
        self.access.add_component_write(index);
        self.add_required(index);
        self.and_with(index);
    }

    /// Adds access to the resource given by `index`.
    pub fn add_resource_read(&mut self, index: ComponentId) {
        self.access.add_resource_read(index);
    }

    /// Adds exclusive access to the resource given by `index`.
    pub fn add_resource_write(&mut self, index: ComponentId) {
        self.access.add_resource_write(index);
    }

    fn add_required(&mut self, index: ComponentId) {
        self.required.grow_and_insert(index.index());
    }

    /// Adds a `With` filter: corresponds to a conjunction (AND) operation.
    ///
    /// Suppose we begin with `Or<(With<A>, With<B>)>`, which is represented by an array of two `AccessFilter` instances.
    /// Adding `AND With<C>` via this method transforms it into the equivalent of  `Or<((With<A>, With<C>), (With<B>, With<C>))>`.
    pub fn and_with(&mut self, index: ComponentId) {
        for filter in &mut self.filter_sets {
            filter.with.grow_and_insert(index.index());
        }
    }

    /// Adds a `Without` filter: corresponds to a conjunction (AND) operation.
    ///
    /// Suppose we begin with `Or<(With<A>, With<B>)>`, which is represented by an array of two `AccessFilter` instances.
    /// Adding `AND Without<C>` via this method transforms it into the equivalent of  `Or<((With<A>, Without<C>), (With<B>, Without<C>))>`.
    pub fn and_without(&mut self, index: ComponentId) {
        for filter in &mut self.filter_sets {
            filter.without.grow_and_insert(index.index());
        }
    }

    /// Appends an array of filters: corresponds to a disjunction (OR) operation.
    ///
    /// As the underlying array of filters represents a disjunction,
    /// where each element (`AccessFilters`) represents a conjunction,
    /// we can simply append to the array.
    pub fn append_or(&mut self, other: &FilteredAccess) {
        self.filter_sets.append(&mut other.filter_sets.clone());
    }

    /// Adds all of the accesses from `other` to `self`.
    pub fn extend_access(&mut self, other: &FilteredAccess) {
        self.access.extend(&other.access);
    }

    /// Returns `true` if this and `other` can be active at the same time.
    pub fn is_compatible(&self, other: &FilteredAccess) -> bool {
        // Resources are read from the world rather than the filtered archetypes,
        // so they must be compatible even if the filters are disjoint.
        if !self.access.is_resources_compatible(&other.access) {
            return false;
        }

        if self.access.is_components_compatible(&other.access) {
            return true;
        }

        // If the access instances are incompatible, we want to check that whether filters can
        // guarantee that queries are disjoint.
        // Since the `filter_sets` array represents a Disjunctive Normal Form formula ("ORs of ANDs"),
        // we need to make sure that each filter set (ANDs) rule out every filter set from the `other` instance.
        //
        // For example, `Query<&mut C, Or<(With<A>, Without<B>)>>` is compatible `Query<&mut C, (With<B>, Without<A>)>`,
        // but `Query<&mut C, Or<(Without<A>, Without<B>)>>` isn't compatible with `Query<&mut C, Or<(With<A>, With<B>)>>`.
        self.filter_sets.iter().all(|filter| {
            other
                .filter_sets
                .iter()
                .all(|other_filter| filter.is_ruled_out_by(other_filter))
        })
    }

    /// Returns a vector of elements that this and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &FilteredAccess) -> AccessConflicts {
        if !self.is_compatible(other) {
            // filters are disjoint, so we can just look at the unfiltered intersection
            return self.access.get_conflicts(&other.access);
        }
        AccessConflicts::empty()
    }

    /// Adds all access and filters from `other`.
    ///
    /// Corresponds to a conjunction operation (AND) for filters.
    ///
    /// Extending `Or<(With<A>, Without<B>)>` with `Or<(With<C>, Without<D>)>` will result in
    /// `Or<((With<A>, With<C>), (With<A>, Without<D>), (Without<B>, With<C>), (Without<B>, Without<D>))>`.
    pub fn extend(&mut self, other: &FilteredAccess) {
        self.access.extend(&other.access);
        self.required.union_with(&other.required);

        // We can avoid allocating a new array of bitsets if `other` contains just a single set of filters:
        // in this case we can short-circuit by performing an in-place union for each bitset.
        if other.filter_sets.len() == 1 {
            for filter in &mut self.filter_sets {
                filter.with.union_with(&other.filter_sets[0].with);
                filter.without.union_with(&other.filter_sets[0].without);
            }
            return;
        }

        let mut new_filters = Vec::with_capacity(self.filter_sets.len() * other.filter_sets.len());
        for filter in &self.filter_sets {
            for other_filter in &other.filter_sets {
                let mut new_filter = filter.clone();
                new_filter.with.union_with(&other_filter.with);
                new_filter.without.union_with(&other_filter.without);
                new_filters.push(new_filter);
            }
        }
        self.filter_sets = new_filters;
    }

    /// Sets the underlying unfiltered access as having access to all indexed elements.
    pub fn read_all(&mut self) {
        self.access.read_all();
    }

    /// Sets the underlying unfiltered access as having mutable access to all indexed elements.
    pub fn write_all(&mut self) {
        self.access.write_all();
    }

    /// Sets the underlying unfiltered access as having access to all components.
    pub fn read_all_components(&mut self) {
        self.access.read_all_components();
    }

    /// Sets the underlying unfiltered access as having mutable access to all components.
    pub fn write_all_components(&mut self) {
        self.access.write_all_components();
    }

    /// Returns `true` if the set is a subset of another, i.e. `other` contains
    /// at least all the values in `self`.
    pub fn is_subset(&self, other: &FilteredAccess) -> bool {
        self.required.is_subset(&other.required) && self.access().is_subset(other.access())
    }

    /// Returns the indices of the elements that this access filters for.
    pub fn with_filters(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.filter_sets
            .iter()
            .flat_map(|f| f.with.ones().map(ComponentId::new))
    }

    /// Returns the indices of the elements that this access filters out.
    pub fn without_filters(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.filter_sets
            .iter()
            .flat_map(|f| f.without.ones().map(ComponentId::new))
    }

    /// Returns true if the index is used by this `FilteredAccess` in filters or archetypal access.
    /// This includes most ways to access a component, but notably excludes `EntityRef` and `EntityMut`
    /// along with anything inside `Option<T>`.
    pub fn contains(&self, index: ComponentId) -> bool {
        self.access().has_archetypal(index)
            || self
                .filter_sets
                .iter()
                .any(|f| f.with.contains(index.index()) || f.without.contains(index.index()))
    }
}

#[derive(Eq, PartialEq, Default)]
pub(crate) struct AccessFilters {
    pub(crate) with: FixedBitSet,
    pub(crate) without: FixedBitSet,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for AccessFilters {
    fn clone(&self) -> Self {
        Self {
            with: self.with.clone(),
            without: self.without.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.with.clone_from(&source.with);
        self.without.clone_from(&source.without);
    }
}

impl Debug for AccessFilters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessFilters")
            .field("with", &FormattedBitSet::new(&self.with))
            .field("without", &FormattedBitSet::new(&self.without))
            .finish()
    }
}

impl AccessFilters {
    fn is_ruled_out_by(&self, other: &Self) -> bool {
        // Although not technically complete, we don't consider the case when `AccessFilters`'s
        // `without` bitset contradicts its own `with` bitset (e.g. `(With<A>, Without<A>)`).
        // Such query would be considered compatible with any other query, but as it's almost
        // always an error, we ignore this case instead of treating such query as compatible
        // with others.
        !self.with.is_disjoint(&other.without) || !self.without.is_disjoint(&other.with)
    }
}

/// A collection of [`FilteredAccess`] instances.
///
/// Used internally to statically check if systems have conflicting access.
///
/// It stores multiple sets of accesses.
/// - A "combined" set, which is the access of all filters in this set combined.
/// - The set of access of each individual filters in this set.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct FilteredAccessSet {
    combined_access: Access,
    filtered_accesses: Vec<FilteredAccess>,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for FilteredAccessSet {
    fn clone(&self) -> Self {
        Self {
            combined_access: self.combined_access.clone(),
            filtered_accesses: self.filtered_accesses.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.combined_access.clone_from(&source.combined_access);
        self.filtered_accesses.clone_from(&source.filtered_accesses);
    }
}

impl FilteredAccessSet {
    /// Creates an empty [`FilteredAccessSet`].
    pub const fn new() -> Self {
        Self {
            combined_access: Access::new(),
            filtered_accesses: Vec::new(),
        }
    }

    /// Returns a reference to the unfiltered access of the entire set.
    #[inline]
    pub fn combined_access(&self) -> &Access {
        &self.combined_access
    }

    /// Returns `true` if this and `other` can be active at the same time.
    ///
    /// Access conflict resolution happen in two steps:
    /// 1. A "coarse" check, if there is no mutual unfiltered conflict between
    ///    `self` and `other`, we already know that the two access sets are
    ///    compatible.
    /// 2. A "fine grained" check, it kicks in when the "coarse" check fails.
    ///    the two access sets might still be compatible if some of the accesses
    ///    are restricted with the [`With`](super::With) or [`Without`](super::Without) filters so that access is
    ///    mutually exclusive. The fine grained phase iterates over all filters in
    ///    the `self` set and compares it to all the filters in the `other` set,
    ///    making sure they are all mutually compatible.
    pub fn is_compatible(&self, other: &FilteredAccessSet) -> bool {
        if self.combined_access.is_compatible(other.combined_access()) {
            return true;
        }
        for filtered in &self.filtered_accesses {
            for other_filtered in &other.filtered_accesses {
                if !filtered.is_compatible(other_filtered) {
                    return false;
                }
            }
        }
        true
    }

    /// Returns a vector of elements that this set and `other` cannot access at the same time.
    pub fn get_conflicts(&self, other: &FilteredAccessSet) -> AccessConflicts {
        // if the unfiltered access is incompatible, must check each pair
        let mut conflicts = AccessConflicts::empty();
        if !self.combined_access.is_compatible(other.combined_access()) {
            for filtered in &self.filtered_accesses {
                for other_filtered in &other.filtered_accesses {
                    conflicts.add(&filtered.get_conflicts(other_filtered));
                }
            }
        }
        conflicts
    }

    /// Returns a vector of elements that this set and `other` cannot access at the same time.
    pub fn get_conflicts_single(&self, filtered_access: &FilteredAccess) -> AccessConflicts {
        // if the unfiltered access is incompatible, must check each pair
        let mut conflicts = AccessConflicts::empty();
        if !self.combined_access.is_compatible(filtered_access.access()) {
            for filtered in &self.filtered_accesses {
                conflicts.add(&filtered.get_conflicts(filtered_access));
            }
        }
        conflicts
    }

    /// Adds the filtered access to the set.
    pub fn add(&mut self, filtered_access: FilteredAccess) {
        self.combined_access.extend(&filtered_access.access);
        self.filtered_accesses.push(filtered_access);
    }

    /// Adds a read access to a resource to the set.
    pub fn add_unfiltered_resource_read(&mut self, index: ComponentId) {
        let mut filter = FilteredAccess::default();
        filter.add_resource_read(index);
        self.add(filter);
    }

    /// Adds a write access to a resource to the set.
    pub fn add_unfiltered_resource_write(&mut self, index: ComponentId) {
        let mut filter = FilteredAccess::default();
        filter.add_resource_write(index);
        self.add(filter);
    }

    /// Adds read access to all resources to the set.
    pub fn add_unfiltered_read_all_resources(&mut self) {
        let mut filter = FilteredAccess::default();
        filter.access.read_all_resources();
        self.add(filter);
    }

    /// Adds write access to all resources to the set.
    pub fn add_unfiltered_write_all_resources(&mut self) {
        let mut filter = FilteredAccess::default();
        filter.access.write_all_resources();
        self.add(filter);
    }

    /// Adds all of the accesses from the passed set to `self`.
    pub fn extend(&mut self, filtered_access_set: FilteredAccessSet) {
        self.combined_access
            .extend(&filtered_access_set.combined_access);
        self.filtered_accesses
            .extend(filtered_access_set.filtered_accesses);
    }

    /// Marks the set as reading all possible indices of type T.
    pub fn read_all(&mut self) {
        let mut filter = FilteredAccess::matches_everything();
        filter.read_all();
        self.add(filter);
    }

    /// Marks the set as writing all T.
    pub fn write_all(&mut self) {
        let mut filter = FilteredAccess::matches_everything();
        filter.write_all();
        self.add(filter);
    }

    /// Removes all accesses stored in this set.
    pub fn clear(&mut self) {
        self.combined_access.clear();
        self.filtered_accesses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{invertible_difference_with, invertible_union_with};
    use crate::{
        component::ComponentId,
        query::{
            access::AccessFilters, Access, AccessConflicts, ComponentAccessKind, FilteredAccess,
            FilteredAccessSet, UnboundedAccessError,
        },
    };
    use alloc::{vec, vec::Vec};
    use fixedbitset::FixedBitSet;

    fn create_sample_access() -> Access {
        let mut access = Access::default();

        access.add_component_read(ComponentId::new(1));
        access.add_component_read(ComponentId::new(2));
        access.add_component_write(ComponentId::new(3));
        access.add_archetypal(ComponentId::new(5));
        access.read_all();

        access
    }

    fn create_sample_filtered_access() -> FilteredAccess {
        let mut filtered_access = FilteredAccess::default();

        filtered_access.add_component_write(ComponentId::new(1));
        filtered_access.add_component_read(ComponentId::new(2));
        filtered_access.add_required(ComponentId::new(3));
        filtered_access.and_with(ComponentId::new(4));

        filtered_access
    }

    fn create_sample_access_filters() -> AccessFilters {
        let mut access_filters = AccessFilters::default();

        access_filters.with.grow_and_insert(3);
        access_filters.without.grow_and_insert(5);

        access_filters
    }

    fn create_sample_filtered_access_set() -> FilteredAccessSet {
        let mut filtered_access_set = FilteredAccessSet::default();

        filtered_access_set.add_unfiltered_resource_read(ComponentId::new(2));
        filtered_access_set.add_unfiltered_resource_write(ComponentId::new(4));
        filtered_access_set.read_all();

        filtered_access_set
    }

    #[test]
    fn test_access_clone() {
        let original = create_sample_access();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_access_clone_from() {
        let original = create_sample_access();
        let mut cloned = Access::default();

        cloned.add_component_write(ComponentId::new(7));
        cloned.add_component_read(ComponentId::new(4));
        cloned.add_archetypal(ComponentId::new(8));
        cloned.write_all();

        cloned.clone_from(&original);

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_filtered_access_clone() {
        let original = create_sample_filtered_access();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_filtered_access_clone_from() {
        let original = create_sample_filtered_access();
        let mut cloned = FilteredAccess::default();

        cloned.add_component_write(ComponentId::new(7));
        cloned.add_component_read(ComponentId::new(4));
        cloned.append_or(&FilteredAccess::default());

        cloned.clone_from(&original);

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_access_filters_clone() {
        let original = create_sample_access_filters();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_access_filters_clone_from() {
        let original = create_sample_access_filters();
        let mut cloned = AccessFilters::default();

        cloned.with.grow_and_insert(1);
        cloned.without.grow_and_insert(2);

        cloned.clone_from(&original);

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_filtered_access_set_clone() {
        let original = create_sample_filtered_access_set();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_filtered_access_set_from() {
        let original = create_sample_filtered_access_set();
        let mut cloned = FilteredAccessSet::default();

        cloned.add_unfiltered_resource_read(ComponentId::new(7));
        cloned.add_unfiltered_resource_write(ComponentId::new(9));
        cloned.write_all();

        cloned.clone_from(&original);

        assert_eq!(original, cloned);
    }

    #[test]
    fn read_all_access_conflicts() {
        // read_all / single write
        let mut access_a = Access::default();
        access_a.add_component_write(ComponentId::new(0));

        let mut access_b = Access::default();
        access_b.read_all();

        assert!(!access_b.is_compatible(&access_a));

        // read_all / read_all
        let mut access_a = Access::default();
        access_a.read_all();

        let mut access_b = Access::default();
        access_b.read_all();

        assert!(access_b.is_compatible(&access_a));
    }

    #[test]
    fn access_get_conflicts() {
        let mut access_a = Access::default();
        access_a.add_component_read(ComponentId::new(0));
        access_a.add_component_read(ComponentId::new(1));

        let mut access_b = Access::default();
        access_b.add_component_read(ComponentId::new(0));
        access_b.add_component_write(ComponentId::new(1));

        assert_eq!(
            access_a.get_conflicts(&access_b),
            vec![ComponentId::new(1)].into()
        );

        let mut access_c = Access::default();
        access_c.add_component_write(ComponentId::new(0));
        access_c.add_component_write(ComponentId::new(1));

        assert_eq!(
            access_a.get_conflicts(&access_c),
            vec![ComponentId::new(0), ComponentId::new(1)].into()
        );
        assert_eq!(
            access_b.get_conflicts(&access_c),
            vec![ComponentId::new(0), ComponentId::new(1)].into()
        );

        let mut access_d = Access::default();
        access_d.add_component_read(ComponentId::new(0));

        assert_eq!(access_d.get_conflicts(&access_a), AccessConflicts::empty());
        assert_eq!(access_d.get_conflicts(&access_b), AccessConflicts::empty());
        assert_eq!(
            access_d.get_conflicts(&access_c),
            vec![ComponentId::new(0)].into()
        );
    }

    #[test]
    fn filtered_combined_access() {
        let mut access_a = FilteredAccessSet::default();
        access_a.add_unfiltered_resource_read(ComponentId::new(1));

        let mut filter_b = FilteredAccess::default();
        filter_b.add_resource_write(ComponentId::new(1));

        let conflicts = access_a.get_conflicts_single(&filter_b);
        assert_eq!(
            &conflicts,
            &AccessConflicts::from(vec![ComponentId::new(1)]),
            "access_a: {access_a:?}, filter_b: {filter_b:?}"
        );
    }

    #[test]
    fn filtered_access_extend() {
        let mut access_a = FilteredAccess::default();
        access_a.add_component_read(ComponentId::new(0));
        access_a.add_component_read(ComponentId::new(1));
        access_a.and_with(ComponentId::new(2));

        let mut access_b = FilteredAccess::default();
        access_b.add_component_read(ComponentId::new(0));
        access_b.add_component_write(ComponentId::new(3));
        access_b.and_without(ComponentId::new(4));

        access_a.extend(&access_b);

        let mut expected = FilteredAccess::default();
        expected.add_component_read(ComponentId::new(0));
        expected.add_component_read(ComponentId::new(1));
        expected.and_with(ComponentId::new(2));
        expected.add_component_write(ComponentId::new(3));
        expected.and_without(ComponentId::new(4));

        assert!(access_a.eq(&expected));
    }

    #[test]
    fn filtered_access_extend_or() {
        let mut access_a = FilteredAccess::default();
        // Exclusive access to `(&mut A, &mut B)`.
        access_a.add_component_write(ComponentId::new(0));
        access_a.add_component_write(ComponentId::new(1));

        // Filter by `With<C>`.
        let mut access_b = FilteredAccess::default();
        access_b.and_with(ComponentId::new(2));

        // Filter by `(With<D>, Without<E>)`.
        let mut access_c = FilteredAccess::default();
        access_c.and_with(ComponentId::new(3));
        access_c.and_without(ComponentId::new(4));

        // Turns `access_b` into `Or<(With<C>, (With<D>, Without<D>))>`.
        access_b.append_or(&access_c);
        // Applies the filters to the initial query, which corresponds to the FilteredAccess'
        // representation of `Query<(&mut A, &mut B), Or<(With<C>, (With<D>, Without<E>))>>`.
        access_a.extend(&access_b);

        // Construct the expected `FilteredAccess` struct.
        // The intention here is to test that exclusive access implied by `add_write`
        // forms correct normalized access structs when extended with `Or` filters.
        let mut expected = FilteredAccess::default();
        expected.add_component_write(ComponentId::new(0));
        expected.add_component_write(ComponentId::new(1));
        // The resulted access is expected to represent `Or<((With<A>, With<B>, With<C>), (With<A>, With<B>, With<D>, Without<E>))>`.
        expected.filter_sets = vec![
            AccessFilters {
                with: FixedBitSet::with_capacity_and_blocks(3, [0b111]),
                without: FixedBitSet::default(),
            },
            AccessFilters {
                with: FixedBitSet::with_capacity_and_blocks(4, [0b1011]),
                without: FixedBitSet::with_capacity_and_blocks(5, [0b10000]),
            },
        ];

        assert_eq!(access_a, expected);
    }

    #[test]
    fn try_iter_component_access_simple() {
        let mut access = Access::default();

        access.add_component_read(ComponentId::new(1));
        access.add_component_read(ComponentId::new(2));
        access.add_component_write(ComponentId::new(3));
        access.add_archetypal(ComponentId::new(5));

        let result = access
            .try_iter_component_access()
            .map(Iterator::collect::<Vec<_>>);

        assert_eq!(
            result,
            Ok(vec![
                ComponentAccessKind::Shared(ComponentId::new(1)),
                ComponentAccessKind::Shared(ComponentId::new(2)),
                ComponentAccessKind::Exclusive(ComponentId::new(3)),
                ComponentAccessKind::Archetypal(ComponentId::new(5)),
            ]),
        );
    }

    #[test]
    fn try_iter_component_access_unbounded_write_all() {
        let mut access = Access::default();

        access.add_component_read(ComponentId::new(1));
        access.add_component_read(ComponentId::new(2));
        access.write_all();

        let result = access
            .try_iter_component_access()
            .map(Iterator::collect::<Vec<_>>);

        assert_eq!(
            result,
            Err(UnboundedAccessError {
                writes_inverted: true,
                read_and_writes_inverted: true
            }),
        );
    }

    #[test]
    fn try_iter_component_access_unbounded_read_all() {
        let mut access = Access::default();

        access.add_component_read(ComponentId::new(1));
        access.add_component_read(ComponentId::new(2));
        access.read_all();

        let result = access
            .try_iter_component_access()
            .map(Iterator::collect::<Vec<_>>);

        assert_eq!(
            result,
            Err(UnboundedAccessError {
                writes_inverted: false,
                read_and_writes_inverted: true
            }),
        );
    }

    /// Create a `FixedBitSet` with a given number of total bits and a given list of bits to set.
    /// Setting the number of bits is important in tests since the `PartialEq` impl checks that the length matches.
    fn bit_set(bits: usize, iter: impl IntoIterator<Item = usize>) -> FixedBitSet {
        let mut result = FixedBitSet::with_capacity(bits);
        result.extend(iter);
        result
    }

    #[test]
    fn invertible_union_with_tests() {
        let invertible_union = |mut self_inverted: bool, other_inverted: bool| {
            // Check all four possible bit states: In both sets, the first, the second, or neither
            let mut self_set = bit_set(4, [0, 1]);
            let other_set = bit_set(4, [0, 2]);
            invertible_union_with(
                &mut self_set,
                &mut self_inverted,
                &other_set,
                other_inverted,
            );
            (self_set, self_inverted)
        };

        // Check each combination of `inverted` flags
        let (s, i) = invertible_union(false, false);
        // [0, 1] | [0, 2] = [0, 1, 2]
        assert_eq!((s, i), (bit_set(4, [0, 1, 2]), false));

        let (s, i) = invertible_union(false, true);
        // [0, 1] | [1, 3, ...] = [0, 1, 3, ...]
        assert_eq!((s, i), (bit_set(4, [2]), true));

        let (s, i) = invertible_union(true, false);
        // [2, 3, ...] | [0, 2] = [0, 2, 3, ...]
        assert_eq!((s, i), (bit_set(4, [1]), true));

        let (s, i) = invertible_union(true, true);
        // [2, 3, ...] | [1, 3, ...] = [1, 2, 3, ...]
        assert_eq!((s, i), (bit_set(4, [0]), true));
    }

    #[test]
    fn invertible_union_with_different_lengths() {
        // When adding a large inverted set to a small normal set,
        // make sure we invert the bits beyond the original length.
        // Failing to call `grow` before `toggle_range` would cause bit 1 to be zero,
        // which would incorrectly treat it as included in the output set.
        let mut self_set = bit_set(1, [0]);
        let mut self_inverted = false;
        let other_set = bit_set(3, [0, 1]);
        let other_inverted = true;
        invertible_union_with(
            &mut self_set,
            &mut self_inverted,
            &other_set,
            other_inverted,
        );

        // [0] | [2, ...] = [0, 2, ...]
        assert_eq!((self_set, self_inverted), (bit_set(3, [1]), true));
    }

    #[test]
    fn invertible_difference_with_tests() {
        let invertible_difference = |mut self_inverted: bool, other_inverted: bool| {
            // Check all four possible bit states: In both sets, the first, the second, or neither
            let mut self_set = bit_set(4, [0, 1]);
            let other_set = bit_set(4, [0, 2]);
            invertible_difference_with(
                &mut self_set,
                &mut self_inverted,
                &other_set,
                other_inverted,
            );
            (self_set, self_inverted)
        };

        // Check each combination of `inverted` flags
        let (s, i) = invertible_difference(false, false);
        // [0, 1] - [0, 2] = [1]
        assert_eq!((s, i), (bit_set(4, [1]), false));

        let (s, i) = invertible_difference(false, true);
        // [0, 1] - [1, 3, ...] = [0]
        assert_eq!((s, i), (bit_set(4, [0]), false));

        let (s, i) = invertible_difference(true, false);
        // [2, 3, ...] - [0, 2] = [3, ...]
        assert_eq!((s, i), (bit_set(4, [0, 1, 2]), true));

        let (s, i) = invertible_difference(true, true);
        // [2, 3, ...] - [1, 3, ...] = [2]
        assert_eq!((s, i), (bit_set(4, [2]), false));
    }
}
